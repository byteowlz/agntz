//! Thin, safe wrapper around `git` subprocesses used by the board and wiki
//! modules. Never stashes, resets, force-pushes, or discards another session's
//! work; callers are responsible for the high-level safety policy.

use std::path::Path;
use std::process::Command;

use anyhow::{Result, anyhow};

/// Raw result of a `git` invocation.
#[derive(Debug, Clone)]
pub struct GitOut {
    /// Whether the command exited 0.
    pub ok: bool,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

impl GitOut {
    /// Trimmed stdout as a string.
    #[must_use]
    pub fn out(&self) -> &str {
        self.stdout.trim()
    }
}

/// Run `git` in `dir` with `args`, never panicking.
#[must_use]
pub fn run(dir: &Path, args: &[&str]) -> GitOut {
    match Command::new("git").current_dir(dir).args(args).output() {
        Ok(o) => GitOut {
            ok: o.status.success(),
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
        },
        Err(e) => GitOut {
            ok: false,
            stdout: String::new(),
            stderr: format!("git not available: {e}"),
        },
    }
}

/// Run `git` and require success, returning trimmed stdout.
///
/// # Errors
///
/// Returns an error if git fails or is unavailable.
pub fn run_need(dir: &Path, args: &[&str]) -> Result<String> {
    let out = run(dir, args);
    if !out.ok {
        return Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            out.stderr.trim()
        ));
    }
    Ok(out.out().to_string())
}

/// Verify git is on PATH and returns a usable version.
///
/// # Errors
///
/// Returns an error if git is not installed.
pub fn ensure_git() -> Result<()> {
    let out = run(Path::new("."), &["--version"]);
    if !out.ok {
        return Err(anyhow!("git is not installed or not on PATH"));
    }
    Ok(())
}

/// Whether `dir` is inside a Git working tree.
#[must_use]
pub fn is_repo(dir: &Path) -> bool {
    run(dir, &["rev-parse", "--is-inside-work-tree"]).ok
}

/// Whether `dir` has a dirty working tree (modified/untracked staged or not).
#[must_use]
pub fn is_dirty(dir: &Path) -> bool {
    let out = run(dir, &["status", "--porcelain"]);
    out.ok && !out.out().is_empty()
}

/// Current branch name, if any.
#[must_use]
pub fn current_branch(dir: &Path) -> Option<String> {
    let out = run(dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    out.ok
        .then(|| out.out().to_string())
        .filter(|s| s != "HEAD")
}

/// Resolve the configured remote URL for `name`, if set.
#[must_use]
pub fn remote_url(dir: &Path, name: &str) -> Option<String> {
    let out = run(dir, &["remote", "get-url", name]);
    out.ok
        .then(|| out.out().to_string())
        .filter(|s| !s.is_empty())
}

/// Whether a remote by `name` is configured.
#[must_use]
pub fn has_remote(dir: &Path, name: &str) -> bool {
    remote_url(dir, name).is_some()
}

/// Number of commits on HEAD that are not pushed to `remote`/`branch`.
#[must_use]
pub fn commits_ahead(dir: &Path, remote: &str, branch: &str) -> Option<usize> {
    let out = run(
        dir,
        &["rev-list", "--count", &format!("{remote}/{branch}..HEAD")],
    );
    out.ok.then(|| out.out().parse::<usize>().ok()).flatten()
}

/// Commits on HEAD but not in the remote tracking ref, as short SHAs.
#[must_use]
pub fn unpushed_commits(dir: &Path, remote: &str, branch: &str) -> Vec<String> {
    let out = run(
        dir,
        &["rev-list", "--oneline", &format!("{remote}/{branch}..HEAD")],
    );
    if out.ok {
        out.stdout.lines().map(str::to_string).collect()
    } else {
        Vec::new()
    }
}

/// Initialize a git repository at `dir`.
///
/// # Errors
///
/// Returns an error if `git init` fails.
pub fn init(dir: &Path) -> Result<()> {
    run_need(dir, &["init"])?;
    Ok(())
}

/// Stage `paths` (relative to `dir`).
///
/// # Errors
///
/// Returns an error if `git add` fails.
pub fn add(dir: &Path, paths: &[&str]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut args = vec!["add".to_string(), "--".to_string()];
    args.extend(paths.iter().map(ToString::to_string));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_need(dir, &refs)?;
    Ok(())
}

/// Stage a single path.
///
/// # Errors
///
/// Returns an error if `git add` fails.
pub fn add_one(dir: &Path, path: &str) -> Result<()> {
    add(dir, &[path])
}

/// Create a commit with `message` using `--only` for the staged paths.
///
/// # Errors
///
/// Returns an error if `git commit` fails or nothing is committed.
pub fn commit(dir: &Path, message: &str) -> Result<()> {
    run_need(dir, &["commit", "-m", message])?;
    Ok(())
}

/// Fetch from `remote`.
///
/// # Errors
///
/// Returns an error if the fetch fails.
pub fn fetch(dir: &Path, remote: &str) -> Result<()> {
    run_need(dir, &["fetch", remote])?;
    Ok(())
}

/// Fetch and fast-forward the working tree to the remote tracking branch for a
/// READ. Returns `Ok` even if `remote/branch` does not yet exist (e.g. a fresh
/// remote that has never been pushed to), so a local-only/brand-new repo still
/// scans its local tree. Never force-moves or discards local work.
///
/// # Errors
///
/// Returns an error if the fetch fails or the fast-forward fails (e.g. the
/// local tree has diverged or is dirty).
/// Best-effort freshness for a read path: fetch the remote and fast-forward
/// the local branch when possible. Never fails a read because of a network
/// error, a missing remote branch, or incompatible/unrelated history — in those
/// cases it warns and the caller reads the local tree as-is. This keeps a read
/// usable even when the remote is unreachable or the histories diverged.
pub fn sync_for_read(dir: &Path, remote: &str, branch: &str) -> Result<()> {
    if let Err(e) = run_need(dir, &["fetch", remote]) {
        log::warn!("fetch from {remote} failed ({e:#}); reading local tree");
        return Ok(());
    }
    let track = format!("{remote}/{branch}");
    if !run(dir, &["rev-parse", "--verify", &track]).ok {
        // No remote branch to merge (fresh/empty remote).
        return Ok(());
    }
    if let Err(e) = run_need(dir, &["merge", "--ff-only", &track]) {
        log::warn!("fast-forward from {track} skipped ({e:#}); reading local tree");
    }
    Ok(())
}

/// Rebase the local branch onto `remote`/`branch`.
///
/// # Errors
///
/// Returns an error if the rebase fails.
pub fn rebase(dir: &Path, remote: &str, branch: &str) -> Result<()> {
    run_need(dir, &["rebase", &format!("{remote}/{branch}")])?;
    Ok(())
}

/// Push to `remote`/`branch` without force.
///
/// # Errors
///
/// Returns an error if the push fails.
pub fn push(dir: &Path, remote: &str, branch: &str) -> Result<()> {
    run_need(dir, &["push", remote, branch])?;
    Ok(())
}

/// Ensure a configurable `user.name` and `user.email` exist, without changing
/// global git config.
///
/// # Errors
///
/// Returns an error if identity is missing or unsettable locally.
pub fn ensure_identity(dir: &Path) -> Result<()> {
    let name = run(dir, &["config", "user.name"]);
    let email = run(dir, &["config", "user.email"]);
    if name.ok && !name.out().is_empty() && email.ok && !email.out().is_empty() {
        return Ok(());
    }

    let name = env_or("GIT_AUTHOR_NAME", "AGENT_NAME").filter(|s| !s.is_empty());
    let email = env_or("GIT_AUTHOR_EMAIL", "AGENT_EMAIL").filter(|s| !s.is_empty());

    if let (Some(name), Some(email)) = (name, email)
        && !name.is_empty()
        && !email.is_empty()
    {
        run_need(dir, &["config", "user.name", &name])?;
        run_need(dir, &["config", "user.email", &email])?;
        return Ok(());
    }

    Err(anyhow!(
        "git user identity is not configured; set git config user.name/user.email in this repo (never globally)"
    ))
}

/// Read an env var, returning an empty string if unset.
fn env_or(primary: &str, fallback: &str) -> Option<String> {
    std::env::var(primary)
        .ok()
        .or_else(|| std::env::var(fallback).ok())
}
