//! `agntz find` — one query, every store.
//!
//! Fans a query out to hstry (session history), mmry (memories), trx (tasks),
//! wiki, board and the git working tree, then renders grouped results (or one
//! unified JSON envelope). Backends degrade independently: a missing or broken
//! tool lands in `unavailable` with its reason and never fails the whole find.
//!
//! Results are cached per source under `agntz/cache/find/` with exact version
//! tokens where they exist (git HEAD for wiki/board/git, the trx ledger mtime)
//! and a TTL for stores without one (mmry, hstry).

use crate::cache;
use anyhow::{Result, anyhow};
use clap::Subcommand;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

const NS: &str = "find";

#[derive(Subcommand)]
pub enum FindCommand {
    /// Search every store (hstry, mmry, trx, wiki, board, git) at once
    Find {
        /// The query
        query: String,
        /// Comma-separated sources to query (default: all available)
        #[arg(long, value_delimiter = ',')]
        source: Vec<String>,
        /// Max hits per source
        #[arg(long, default_value_t = 5)]
        limit: usize,
        /// Also search git history (git log -S; slower)
        #[arg(long)]
        git_history: bool,
        /// Ignore cached results and recompute
        #[arg(long)]
        refresh: bool,
        /// Do not read or write the cache at all
        #[arg(long)]
        no_cache: bool,
        /// Cache TTL in seconds
        #[arg(long, default_value_t = cache::DEFAULT_TTL)]
        ttl: i64,
    },
}

pub async fn handle(command: FindCommand, json: bool) -> Result<()> {
    match command {
        FindCommand::Find {
            query,
            source,
            limit,
            git_history,
            refresh,
            no_cache,
            ttl,
        } => {
            handle_find(
                &query,
                &source,
                limit,
                git_history,
                refresh,
                no_cache,
                ttl,
                json,
            )
            .await
        }
    }
}

/// One per-source outcome.
#[derive(Debug)]
enum SourceResult {
    /// Structured hits (already rendered into `Value` for output).
    Hits { count: usize, rows: Vec<Value> },
    /// The backend is missing or broken; find continues without it.
    Unavailable { reason: String },
}

struct Backend {
    name: String,
    result: SourceResult,
    cached: bool,
}

async fn handle_find(
    query: &str,
    sources: &[String],
    limit: usize,
    git_history: bool,
    refresh: bool,
    no_cache: bool,
    ttl: i64,
    json: bool,
) -> Result<()> {
    if query.trim().is_empty() {
        return Err(anyhow!("empty query"));
    }
    let enabled: Vec<String> = if sources.is_empty() {
        vec![
            "hstry".into(),
            "mmry".into(),
            "trx".into(),
            "wiki".into(),
            "board".into(),
            "git".into(),
        ]
    } else {
        let valid = ["hstry", "mmry", "trx", "wiki", "board", "git"];
        for s in sources {
            if !valid.contains(&s.as_str()) {
                return Err(anyhow!(
                    "unknown source '{}'; valid: {}",
                    s,
                    valid.join(",")
                ));
            }
        }
        sources.to_vec()
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    // Preserve the caller's global flags (notably --json): find builds its own
    // RuntimeContext because the fan-out needs config-backed repo resolution.
    let mut common = crate::CommonOpts::default();
    common.json = json;
    let ctx = crate::RuntimeContext::new(common)?;

    let mut backends = Vec::new();
    let owned_sources: Vec<String> = enabled.clone();
    for source in owned_sources {
        let (result, cached) = run_source(
            &source,
            &ctx,
            &cwd,
            query,
            limit,
            git_history,
            refresh,
            no_cache,
            ttl,
        )
        .await;
        backends.push(Backend {
            name: source.clone(),
            result,
            cached,
        });
    }

    render(&backends, query, ctx.common.json);
    Ok(())
}

/// Run one source through the cache (version-checked) or its backend.
async fn run_source(
    source: &str,
    ctx: &crate::RuntimeContext,
    cwd: &std::path::Path,
    query: &str,
    limit: usize,
    git_history: bool,
    refresh: bool,
    no_cache: bool,
    ttl: i64,
) -> (SourceResult, bool) {
    let key = format!("{}|{source}|{query}|{limit}|{}", cwd.display(), git_history);

    // Version token: exact where a cheap signal exists.
    let version: cache::VersionToken = match source {
        "git" => git_version_token(cwd),
        "wiki" => repo_head_token(ctx, Repo::Wiki),
        "board" => repo_head_token(ctx, Repo::Board),
        "trx" => file_mtime_token(&cwd.join(".trx").join("issues.jsonl")),
        // No cheap version vector: TTL-only.
        _ => None,
    };

    if !refresh && let Some(payload) = cache::get(NS, &key, ttl, &version, no_cache) {
        let result = payload_to_result(source, &payload);
        return (result, true);
    }

    let (result, _payload) = match source {
        "hstry" => find_hstry(query, limit).await,
        "mmry" => find_mmry(query, limit).await,
        "trx" => find_trx(query, limit).await,
        "wiki" => find_wiki(ctx, query, limit).await,
        "board" => find_board(ctx, query, limit).await,
        "git" => find_git(cwd, query, limit, git_history),
        other => (
            SourceResult::Unavailable {
                reason: format!("unknown source {other}"),
            },
            Value::Null,
        ),
    };

    if let SourceResult::Hits { rows, .. } = &result {
        cache::put(
            NS,
            &key,
            &json!({ "rows": rows, "count": rows.len() }),
            ttl,
            &version,
        );
    }
    (result, false)
}

fn payload_to_result(source: &str, payload: &Value) -> SourceResult {
    let rows: Vec<Value> = payload
        .get("rows")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    let _ = source;
    SourceResult::Hits {
        count: rows.len(),
        rows,
    }
}

/// Version token for a git worktree/repo: HEAD sha + dirty-file count, so both
/// commits and uncommitted edits invalidate.
fn git_version_token(dir: &std::path::Path) -> cache::VersionToken {
    if !dir.join(".git").exists() {
        return None;
    }
    let head = crate::gitx::run(dir, &["rev-parse", "HEAD"])
        .out()
        .to_string();
    let head = head.trim().to_string();
    if head.is_empty() {
        return None;
    }
    let dirty = crate::gitx::run(dir, &["status", "--porcelain"])
        .out()
        .to_string()
        .lines()
        .count();
    Some(format!("{head}+{dirty}"))
}

/// Version token for a file: its mtime (e.g. `.trx/issues.jsonl`).
fn file_mtime_token(path: &std::path::Path) -> cache::VersionToken {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos().to_string())
}

enum Repo {
    Wiki,
    Board,
}

fn repo_head_token(ctx: &crate::RuntimeContext, repo: Repo) -> cache::VersionToken {
    let resolved = match repo {
        Repo::Wiki => crate::wiki::resolve_repo_pub(ctx, None),
        Repo::Board => crate::board::resolve_repo_pub(ctx, None),
    };
    let Ok(repo) = resolved else {
        return None;
    };
    git_version_token(std::path::Path::new(&repo.path))
}

// ---------------------------------------------------------------------------
// Backends
// ---------------------------------------------------------------------------

fn run_capture(binary: &str, args: &[String]) -> Result<(bool, String, String), String> {
    let out = Command::new(binary)
        .args(args)
        .output()
        .map_err(|e| format!("{binary} not installed ({e})"))?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    ))
}

async fn find_hstry(query: &str, limit: usize) -> (SourceResult, Value) {
    let args = vec![
        "search".to_string(),
        query.to_string(),
        "--limit".to_string(),
        limit.to_string(),
        "--json".to_string(),
    ];
    match run_capture("hstry", &args) {
        Ok((true, out, _)) => {
            #[derive(serde::Deserialize)]
            struct Resp {
                ok: bool,
                error: Option<String>,
                #[serde(default)]
                result: Option<HstryResult>,
            }
            #[derive(serde::Deserialize)]
            struct HstryResult {
                #[serde(default)]
                hits: Vec<HstryHit>,
            }
            #[derive(serde::Deserialize)]
            struct HstryHit {
                #[serde(default)]
                conversation_id: String,
                #[serde(default)]
                readable_id: Option<String>,
                #[serde(default)]
                role: String,
                #[serde(default)]
                title: Option<String>,
                #[serde(default)]
                snippet: String,
                #[serde(default)]
                timestamp: Option<String>,
            }
            match serde_json::from_str::<Resp>(out.trim()) {
                Ok(r) if r.ok => {
                    let rows: Vec<Value> = r
                        .result
                        .map(|res| res.hits)
                        .unwrap_or_default()
                        .into_iter()
                        .take(limit)
                        .map(|h| {
                            json!({
                                "session": h.readable_id.unwrap_or(h.conversation_id),
                                "role": h.role,
                                "title": h.title.unwrap_or_default(),
                                "snippet": h.snippet,
                                "when": h.timestamp.unwrap_or_default(),
                            })
                        })
                        .collect();
                    (
                        SourceResult::Hits {
                            count: rows.len(),
                            rows,
                        },
                        Value::Null,
                    )
                }
                Ok(r) => (
                    SourceResult::Unavailable {
                        reason: r.error.unwrap_or_else(|| "hstry search failed".into()),
                    },
                    Value::Null,
                ),
                Err(e) => (
                    SourceResult::Unavailable {
                        reason: format!("unparseable hstry output: {e}"),
                    },
                    Value::Null,
                ),
            }
        }
        Ok((false, _, err)) => (SourceResult::Unavailable { reason: err }, Value::Null),
        Err(e) => (SourceResult::Unavailable { reason: e }, Value::Null),
    }
}

async fn find_mmry(query: &str, limit: usize) -> (SourceResult, Value) {
    let args = vec![
        "search".to_string(),
        query.to_string(),
        "--json".to_string(),
        "--limit".to_string(),
        limit.to_string(),
    ];
    match run_capture("mmry", &args) {
        Ok((true, out, _)) => {
            let parsed: Result<Vec<Value>, _> = serde_json::from_str(out.trim());
            match parsed {
                Ok(items) => {
                    let rows: Vec<Value> = items
                        .into_iter()
                        .take(limit)
                        .map(|m| {
                            json!({
                                "id": m.get("memory_id").cloned().unwrap_or(Value::Null),
                                "content": m.get("content").cloned().unwrap_or(Value::Null),
                                "why": m.get("why").cloned().unwrap_or(Value::Null),
                                "repo": m.get("repo").cloned().unwrap_or(Value::Null),
                                "memory_type": m.get("memory_type").cloned().unwrap_or(Value::Null),
                            })
                        })
                        .collect();
                    (
                        SourceResult::Hits {
                            count: rows.len(),
                            rows,
                        },
                        Value::Null,
                    )
                }
                Err(e) => (
                    SourceResult::Unavailable {
                        reason: format!("unparseable mmry output: {e}"),
                    },
                    Value::Null,
                ),
            }
        }
        Ok((false, _, err)) => (SourceResult::Unavailable { reason: err }, Value::Null),
        Err(e) => (SourceResult::Unavailable { reason: e }, Value::Null),
    }
}

async fn find_trx(query: &str, limit: usize) -> (SourceResult, Value) {
    let args = vec!["list".to_string(), "--json".to_string()];
    match run_capture("trx", &args) {
        Ok((true, out, _)) => {
            let parsed: Result<Vec<Value>, _> = serde_json::from_str(out.trim());
            match parsed {
                Ok(issues) => {
                    let q = query.to_ascii_lowercase();
                    let rows: Vec<Value> = issues
                        .into_iter()
                        .filter(|i| {
                            let title = i
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_ascii_lowercase();
                            let desc = i
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_ascii_lowercase();
                            title.contains(&q) || desc.contains(&q)
                        })
                        .take(limit)
                        .map(|i| {
                            json!({
                                "id": i.get("id").cloned().unwrap_or(Value::Null),
                                "title": i.get("title").cloned().unwrap_or(Value::Null),
                                "priority": i.get("priority").cloned().unwrap_or(Value::Null),
                                "status": i.get("status").cloned().unwrap_or(Value::Null),
                            })
                        })
                        .collect();
                    (
                        SourceResult::Hits {
                            count: rows.len(),
                            rows,
                        },
                        Value::Null,
                    )
                }
                Err(e) => (
                    SourceResult::Unavailable {
                        reason: format!("unparseable trx output: {e}"),
                    },
                    Value::Null,
                ),
            }
        }
        Ok((false, _, err)) => (SourceResult::Unavailable { reason: err }, Value::Null),
        Err(e) => (SourceResult::Unavailable { reason: e }, Value::Null),
    }
}

async fn find_wiki(
    ctx: &crate::RuntimeContext,
    query: &str,
    limit: usize,
) -> (SourceResult, Value) {
    match crate::wiki::resolve_repo_pub(ctx, None) {
        Err(e) => (
            SourceResult::Unavailable {
                reason: format!("{e:#}"),
            },
            Value::Null,
        ),
        Ok(repo) => match crate::wiki::search_pages(
            std::path::Path::new(&repo.path),
            query,
            limit,
            Some(&ctx.paths.state_dir),
        ) {
            Ok(hits) => {
                let count = hits.len();
                let rows = hits
                    .into_iter()
                    .map(|h| {
                        json!({
                            "id": h.id,
                            "title": h.title,
                            "path": h.path,
                            "excerpt": h.excerpt,
                        })
                    })
                    .collect();
                (SourceResult::Hits { count, rows }, Value::Null)
            }
            Err(e) => (
                SourceResult::Unavailable {
                    reason: format!("{e:#}"),
                },
                Value::Null,
            ),
        },
    }
}

async fn find_board(
    ctx: &crate::RuntimeContext,
    query: &str,
    limit: usize,
) -> (SourceResult, Value) {
    match crate::board::resolve_repo_pub(ctx, None) {
        Err(e) => (
            SourceResult::Unavailable {
                reason: format!("{e:#}"),
            },
            Value::Null,
        ),
        Ok(repo) => {
            match crate::board::search_messages(std::path::Path::new(&repo.path), query, limit) {
                Ok(hits) => {
                    let count = hits.len();
                    let rows = hits
                        .into_iter()
                        .map(|h| {
                            json!({
                                "id": h.id,
                                "topic": h.topic,
                                "subject": h.subject,
                                "sent_at": h.sent_at,
                                "path": h.path,
                            })
                        })
                        .collect();
                    (SourceResult::Hits { count, rows }, Value::Null)
                }
                Err(e) => (
                    SourceResult::Unavailable {
                        reason: format!("{e:#}"),
                    },
                    Value::Null,
                ),
            }
        }
    }
}

fn find_git(
    cwd: &std::path::Path,
    query: &str,
    limit: usize,
    history: bool,
) -> (SourceResult, Value) {
    if !cwd.join(".git").exists() {
        return (
            SourceResult::Unavailable {
                reason: "not a git repository".into(),
            },
            Value::Null,
        );
    }

    let mut rows = Vec::new();

    // Working tree.
    let args = vec![
        "grep".to_string(),
        "-n".to_string(),
        "-i".to_string(),
        "-I".to_string(),
        "--fixed-strings".to_string(),
        query.to_string(),
    ];
    match run_capture("git", &args) {
        Ok((true, out, _)) => {
            for line in out.lines().take(limit) {
                let (path, rest) = match line.split_once(':') {
                    Some(p) => p,
                    None => continue,
                };
                let (lineno, text) = match rest.split_once(':') {
                    Some(p) => p,
                    None => (rest, ""),
                };
                rows.push(json!({
                    "kind": "tree",
                    "path": path,
                    "line": lineno,
                    "text": text.trim(),
                }));
            }
        }
        Ok((false, _, _)) => {} // no matches in the tree is not an error
        Err(e) => return (SourceResult::Unavailable { reason: e }, Value::Null),
    }

    // Optional pickaxe history search.
    if history && rows.len() < limit {
        let args = vec![
            "log".to_string(),
            "-S".to_string(),
            query.to_string(),
            "--oneline".to_string(),
            "--no-color".to_string(),
            "-n".to_string(),
            limit.to_string(),
        ];
        if let Ok((true, out, _)) = run_capture("git", &args) {
            for line in out.lines().take(limit) {
                let (sha, subject) = match line.split_once(' ') {
                    Some(p) => p,
                    None => continue,
                };
                rows.push(json!({
                    "kind": "history",
                    "sha": sha,
                    "subject": subject,
                }));
            }
        }
    }

    let count = rows.len();
    (SourceResult::Hits { count, rows }, Value::Null)
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(backends: &[Backend], query: &str, as_json: bool) {
    let mut unavailable: Vec<String> = Vec::new();
    let mut any_cached = false;

    if as_json {
        let mut sources = serde_json::Map::new();
        let mut unavailable_json: Vec<Value> = Vec::new();
        for b in backends {
            match &b.result {
                SourceResult::Hits { count, rows } => {
                    if b.cached {
                        any_cached = true;
                    }
                    sources.insert(
                        b.name.clone(),
                        json!({ "count": count, "cached": b.cached, "hits": rows }),
                    );
                }
                SourceResult::Unavailable { reason } => {
                    unavailable_json.push(json!({ "source": b.name.as_str(), "reason": reason }));
                }
            }
        }
        crate::readout::emit(
            "find",
            true,
            None,
            json!({
                "query": query,
                "sources": sources,
                "unavailable": unavailable_json,
                "cached": any_cached,
            }),
        );
        return;
    }

    for b in backends {
        match &b.result {
            SourceResult::Hits { count, rows } => {
                if b.cached {
                    any_cached = true;
                }
                println!(
                    "{source}  {count} hit(s){}",
                    if b.cached { "  (cached)" } else { "" },
                    source = b.name.as_str()
                );
                for row in rows {
                    println!("  {}", render_row(b.name.as_str(), row));
                }
                println!();
            }
            SourceResult::Unavailable { reason } => {
                unavailable.push(format!("{}  unavailable ({reason})", b.name));
            }
        }
    }

    for line in &unavailable {
        println!("{line}");
    }
    if any_cached {
        println!("(cached; use --refresh to recompute)");
    }
}

fn render_row(source: &str, row: &Value) -> String {
    let get = |k: &str| {
        row.get(k)
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    };
    match source {
        "hstry" => format!(
            "{} {} - {}",
            get("when"),
            get("session"),
            crate::compact_label(&format!("{} {}", get("title"), get("snippet")), 140)
        ),
        "mmry" => format!(
            "{} {}",
            get("id"),
            crate::compact_label(&get("content"), 140)
        ),
        "trx" => format!(
            "{} [P{}] {} ({})",
            get("id"),
            get("priority"),
            get("title"),
            get("status")
        ),
        "wiki" => format!("{}  {}", get("id"), get("excerpt")),
        "board" => format!(
            "{}  {}  {}",
            crate::compact_label(&get("topic"), 20),
            crate::compact_label(&get("subject"), 40),
            get("id")
        ),
        "git" => {
            if row.get("kind").and_then(|k| k.as_str()) == Some("history") {
                format!("[history] {} {}", get("sha"), get("subject"))
            } else {
                format!("{}:{}  {}", get("path"), get("line"), get("text"))
            }
        }
        _ => row.to_string(),
    }
}
