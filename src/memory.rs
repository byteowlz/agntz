use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::readout;

#[derive(Subcommand)]
pub enum MemoryCommand {
    /// Add a memory
    Add {
        /// Memory content (or - for stdin)
        content: String,
        /// Tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
        /// Why it matters / how to apply it
        #[arg(long)]
        why: Option<String>,
        /// Where it was observed (command, issue, URL)
        #[arg(long)]
        source: Option<String>,
        /// Memory type (semantic/episodic/procedural)
        #[arg(long)]
        memory_type: Option<String>,
        /// Store as a general (cross-repository) memory
        #[arg(long)]
        general: bool,
        /// Expiry: RFC 3339 timestamp or duration (12h, 30d, 8w)
        #[arg(long)]
        expires: Option<String>,
    },

    /// Search memories
    Search {
        /// Search query
        query: String,
        /// Search mode
        #[arg(short, long, default_value = "hybrid")]
        mode: String,
        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Export memories
    Export {
        /// Output file (defaults to .memories/export.json or .memories/export.md)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Output format: json, md
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Export all stores
        #[arg(long)]
        all: bool,
    },

    /// Import memories
    Import {
        /// Input file
        file: PathBuf,
    },

    /// Show memory statistics
    Stats,

    /// List available stores
    Stores,

    /// List memories
    List {
        /// Maximum number of results
        #[arg(short, long)]
        limit: Option<usize>,
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
        /// Include full embeddings in JSON output
        #[arg(long)]
        full: bool,
    },

    /// Remove a memory
    #[command(alias = "rm")]
    Remove {
        /// Memory ID to remove
        id: String,
    },
}

pub async fn handle(command: MemoryCommand, json: bool) -> Result<()> {
    match command {
        MemoryCommand::Add {
            content,
            tags,
            why,
            source,
            memory_type,
            general,
            expires,
        } => handle_add(content, tags, why, source, memory_type, general, expires).await,
        MemoryCommand::Search { query, mode, limit } => {
            handle_search(query, mode, limit, json).await
        }
        MemoryCommand::Export {
            output,
            format,
            all,
        } => handle_export(output, format, all).await,
        MemoryCommand::Import { file } => handle_import(file).await,
        MemoryCommand::Stats => handle_stats(json).await,
        MemoryCommand::Stores => handle_stores(json).await,
        MemoryCommand::List {
            limit,
            category,
            full,
        } => handle_list(limit, category, json, full).await,
        MemoryCommand::Remove { id } => handle_remove(id).await,
    }
}

async fn handle_add(
    content: String,
    tags: Option<String>,
    why: Option<String>,
    source: Option<String>,
    memory_type: Option<String>,
    general: bool,
    expires: Option<String>,
) -> Result<()> {
    let mut args = vec!["add".to_string()];

    // Handle stdin
    let actual_content = if content == "-" {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        content
    };

    args.push(actual_content);

    // mmry 0.14 flags (the old -c/-t/-i short flags are gone).
    if let Some(t) = tags {
        args.push("--tags".to_string());
        args.push(t);
    }
    if let Some(w) = why {
        args.push("--why".to_string());
        args.push(w);
    }
    if let Some(s) = source {
        args.push("--source".to_string());
        args.push(s);
    }
    if let Some(mt) = memory_type {
        args.push("--memory-type".to_string());
        args.push(mt);
    }
    if general {
        args.push("--general".to_string());
    }
    if let Some(e) = expires {
        args.push("--expires".to_string());
        args.push(e);
    }

    run_mmry(&args)
}

async fn handle_search(query: String, _mode: String, limit: usize, json: bool) -> Result<()> {
    // mmry 0.14 dropped search `--mode`; the flag stays accepted for CLI
    // compatibility but is no longer forwarded.
    let args = vec![
        "search".to_string(),
        query,
        "--limit".to_string(),
        limit.to_string(),
        "--json".to_string(),
    ];

    if json {
        emit_mmry("memory/search", args).await
    } else {
        run_mmry(&args)
    }
}

async fn handle_export(output: Option<PathBuf>, format: String, all: bool) -> Result<()> {
    // Determine output path
    let output_path = match output {
        Some(p) => p,
        None => {
            // Default to .memories/ directory
            let memories_dir = PathBuf::from(".memories");
            fs::create_dir_all(&memories_dir)?;

            let filename = match format.as_str() {
                "md" | "markdown" => "export.md",
                _ => "export.json",
            };
            memories_dir.join(filename)
        }
    };

    match format.as_str() {
        "md" | "markdown" => export_markdown(&output_path, all).await,
        _ => export_json(&output_path, all).await,
    }
}

/// Fetch memories via `mmry list --json` (mmry 0.14 dropped its own `export`
/// subcommand, so agntz derives exports from the supported read path).
async fn fetch_memories(all: bool) -> Result<Vec<serde_json::Value>> {
    let mut args = vec!["list".to_string(), "--json".to_string()];
    if all {
        args.push("--all".to_string());
    }
    let (ok, out, err) = readout::run("mmry", &args);
    if !ok {
        anyhow::bail!("mmry list failed: {err}");
    }
    serde_json::from_str::<Vec<serde_json::Value>>(out.trim())
        .map_err(|e| anyhow!("mmry list returned an unparseable payload: {e}"))
}

async fn export_json(output: &std::path::Path, all: bool) -> Result<()> {
    let memories = fetch_memories(all).await?;
    fs::write(
        output,
        serde_json::to_string_pretty(&memories).context("serializing memories")?,
    )
    .with_context(|| format!("writing {}", output.display()))?;
    println!(
        "Exported {} memories to {}",
        memories.len(),
        output.display()
    );
    Ok(())
}

async fn export_markdown(output: &PathBuf, all: bool) -> Result<()> {
    let memories = fetch_memories(all).await?;

    let mut md = String::new();
    md.push_str("# Memories\n\n");

    // Group by memory type (mmry 0.14 replaced `category` with `memory_type`).
    let mut by_type: std::collections::HashMap<String, Vec<&serde_json::Value>> =
        std::collections::HashMap::new();
    for mem in &memories {
        let kind = mem
            .get("memory_type")
            .and_then(|v| v.as_str())
            .unwrap_or("uncategorized")
            .to_string();
        by_type.entry(kind).or_default().push(mem);
    }

    for (kind, mems) in by_type {
        md.push_str(&format!("## {kind}\n\n"));
        for mem in mems {
            let content = mem
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let tags = mem
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .filter(|s| !s.is_empty())
                .map(|s| format!(" [tags: {s}]"))
                .unwrap_or_default();
            md.push_str(&format!("- {content}{tags}\n"));
        }
        md.push('\n');
    }

    fs::write(output, &md).with_context(|| format!("writing {}", output.display()))?;
    println!(
        "Exported {} memories to {}",
        memories.len(),
        output.display()
    );
    Ok(())
}

async fn handle_import(_file: PathBuf) -> Result<()> {
    // mmry 0.14 removed its `import` subcommand. Fail loudly instead of
    // silently doing nothing.
    Err(anyhow!(
        "mmry 0.14 removed the `import` subcommand, so `agntz memory import` has no backend; add memories with `agntz memory add` instead"
    ))
}

async fn handle_stats(json: bool) -> Result<()> {
    // mmry 0.14 removed `stats`; `doctor` is the supported store/ledger health
    // view, so surface that instead (never a silent empty success).
    if json {
        emit_mmry(
            "memory/stats",
            vec!["doctor".to_string(), "--json".to_string()],
        )
        .await
    } else {
        run_mmry(&["doctor".to_string()])
    }
}

async fn handle_stores(json: bool) -> Result<()> {
    // mmry 0.14 replaced `stores list` with `repos` (known ledgers).
    if json {
        emit_mmry(
            "memory/stores",
            vec!["repos".to_string(), "--json".to_string()],
        )
        .await
    } else {
        run_mmry_raw(&["repos"])
    }
}

async fn handle_list(
    limit: Option<usize>,
    category: Option<String>,
    json: bool,
    full: bool,
) -> Result<()> {
    let mut args = vec!["ls".to_string()];

    if let Some(l) = limit {
        args.push("--limit".to_string());
        args.push(l.to_string());
    }

    if let Some(cat) = category {
        args.push("--category".to_string());
        args.push(cat);
    }

    if full {
        args.push("--full".to_string());
    }

    if json {
        let mut jargs = args.clone();
        jargs.push("--json".to_string());
        emit_mmry("memory/list", jargs).await
    } else {
        run_mmry(&args)
    }
}

/// Run mmry and emit the unified read envelope (used for --json reads).
async fn emit_mmry(verb: &str, args: Vec<String>) -> Result<()> {
    let (ok, out, err) = readout::run("mmry", &args);
    readout::emit(
        verb,
        ok,
        (!ok).then(|| format!("mmry failed: {err}")),
        readout::parse_or_text(out),
    );
    Ok(())
}

async fn handle_remove(id: String) -> Result<()> {
    run_mmry(&["rm".to_string(), id])
}

/// Run mmry without auto-store detection
fn run_mmry_raw(args: &[&str]) -> Result<()> {
    let output = Command::new("mmry")
        .args(args)
        .output()
        .context("failed to run mmry - is mmry installed?")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        anyhow::bail!("mmry command failed");
    }

    Ok(())
}

/// Get the current repo name from git remote or directory name
fn get_repo_name() -> Option<String> {
    // Try to get repo name from git remote
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout);
        // Extract repo name from URL (handles both SSH and HTTPS)
        // e.g., git@github.com:user/repo.git -> repo
        // e.g., https://github.com/user/repo.git -> repo
        let name = url
            .trim()
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .map(|s| s.to_string());
        if name.is_some() {
            return name;
        }
    }

    // Fallback: use current directory name
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
}

/// Agent identity detected from environment.
struct AgentIdentity {
    /// Harness name (e.g. "pi", "opencode")
    harness: String,
    /// Session ID or name if available
    session: Option<String>,
    /// Model string (e.g. "anthropic/claude-sonnet-4")
    model: Option<String>,
}

/// Detect the agent harness from env vars.
///
/// Any harness (pi, opencode, aider, etc.) can set:
///   AGENT_HARNESS=<name>          (e.g. "pi", "opencode")
///   AGENT_SESSION_ID=<uuid>
///   AGENT_SESSION_NAME=<readable> (set after auto-rename)
///   AGENT_SESSION_FILE=<path>
///   AGENT_MODEL=<provider>/<id>
///   AGENT_CWD=<workdir>
fn detect_agent() -> Option<AgentIdentity> {
    let harness = std::env::var("AGENT_HARNESS")
        .ok()
        .filter(|s| !s.is_empty())?;

    // Prefer session name (human-readable) over raw UUID
    let session = std::env::var("AGENT_SESSION_NAME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("AGENT_SESSION_ID")
                .ok()
                .filter(|s| !s.is_empty())
        });
    let model = std::env::var("AGENT_MODEL").ok().filter(|s| !s.is_empty());

    Some(AgentIdentity {
        harness,
        session,
        model,
    })
}

fn run_mmry(args: &[String]) -> Result<()> {
    // mmry 0.14 dropped `--store <name>`: repository scoping is now cwd-based
    // (the current repository identity), with an explicit `--repo <NAME>` on the
    // read verbs. Pass args through untouched — injecting the old flag made
    // every invocation fail at mmry's clap level.
    let full_args: &[String] = args;

    let mut cmd = Command::new("mmry");
    cmd.args(full_args);

    // Auto-identify the agent for memory attribution via env vars.
    // mmry reads MMRY_AGENT, MMRY_AGENT_KIND, and MMRY_AGENT_META.
    if let Some(identity) = detect_agent() {
        cmd.env("MMRY_AGENT", &identity.harness);
        cmd.env("MMRY_AGENT_KIND", "coding_agent");

        // Build metadata with repo, session, and model context
        let mut meta = serde_json::Map::new();
        if let Some(repo) = get_repo_name() {
            meta.insert("repo".to_string(), serde_json::Value::String(repo));
        }
        if let Some(ref session) = identity.session {
            meta.insert(
                "session".to_string(),
                serde_json::Value::String(session.clone()),
            );
        }
        if let Some(ref model) = identity.model {
            meta.insert(
                "model".to_string(),
                serde_json::Value::String(model.clone()),
            );
        }
        if !meta.is_empty()
            && let Ok(meta_json) = serde_json::to_string(&meta)
        {
            cmd.env("MMRY_AGENT_META", meta_json);
        }
    }

    let output = cmd
        .output()
        .context("failed to run mmry - is mmry installed?")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        anyhow::bail!("mmry command failed");
    }

    Ok(())
}
