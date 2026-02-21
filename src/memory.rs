use anyhow::{Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Subcommand)]
pub enum MemoryCommand {
    /// Add a memory
    Add {
        /// Memory content (or - for stdin)
        content: String,
        /// Category
        #[arg(short, long)]
        category: Option<String>,
        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
        /// Importance (1-10)
        #[arg(short, long)]
        importance: Option<u8>,
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
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
        /// Output as JSON
        #[arg(long)]
        json: bool,
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

pub async fn handle(command: MemoryCommand) -> Result<()> {
    match command {
        MemoryCommand::Add {
            content,
            category,
            tags,
            importance,
        } => handle_add(content, category, tags, importance).await,
        MemoryCommand::Search {
            query,
            mode,
            limit,
            json,
        } => handle_search(query, mode, limit, json).await,
        MemoryCommand::Export {
            output,
            format,
            all,
        } => handle_export(output, format, all).await,
        MemoryCommand::Import { file } => handle_import(file).await,
        MemoryCommand::Stats => handle_stats().await,
        MemoryCommand::Stores => handle_stores().await,
        MemoryCommand::List {
            limit,
            category,
            json,
            full,
        } => handle_list(limit, category, json, full).await,
        MemoryCommand::Remove { id } => handle_remove(id).await,
    }
}

async fn handle_add(
    content: String,
    category: Option<String>,
    tags: Option<String>,
    importance: Option<u8>,
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

    if let Some(cat) = category {
        args.push("-c".to_string());
        args.push(cat);
    }

    if let Some(t) = tags {
        args.push("-t".to_string());
        args.push(t);
    }

    if let Some(i) = importance {
        args.push("-i".to_string());
        args.push(i.to_string());
    }

    run_mmry(&args)
}

async fn handle_search(query: String, mode: String, limit: usize, json: bool) -> Result<()> {
    let mut args = vec![
        "search".to_string(),
        query,
        "--mode".to_string(),
        mode,
        "--limit".to_string(),
        limit.to_string(),
    ];

    if json {
        args.push("--json".to_string());
    }

    run_mmry(&args)
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

async fn export_json(output: &PathBuf, all: bool) -> Result<()> {
    let mut args = vec![
        "export".to_string(),
        "-o".to_string(),
        output.to_string_lossy().to_string(),
    ];

    if all {
        args.push("--all".to_string());
    }

    run_mmry(&args)?;
    println!("Exported to {}", output.display());
    Ok(())
}

#[derive(Deserialize)]
struct Memory {
    content: String,
    category: Option<String>,
    importance: Option<u8>,
    #[serde(default)]
    #[allow(dead_code)]
    created_at: Option<String>,
}

async fn export_markdown(output: &PathBuf, all: bool) -> Result<()> {
    // First export to JSON, then convert
    let temp_json = std::env::temp_dir().join("agnt_export_temp.json");

    let mut args = vec![
        "export".to_string(),
        "-o".to_string(),
        temp_json.to_string_lossy().to_string(),
    ];

    if all {
        args.push("--all".to_string());
    }

    run_mmry(&args)?;

    // Read and convert to markdown
    let json_content = fs::read_to_string(&temp_json)?;
    let memories: Vec<Memory> = serde_json::from_str(&json_content).unwrap_or_default();

    let mut md = String::new();
    md.push_str("# Memories\n\n");

    // Group by category
    let mut by_category: std::collections::HashMap<String, Vec<&Memory>> =
        std::collections::HashMap::new();
    for mem in &memories {
        let cat = mem
            .category
            .clone()
            .unwrap_or_else(|| "uncategorized".to_string());
        by_category.entry(cat).or_default().push(mem);
    }

    for (category, mems) in by_category {
        md.push_str(&format!("## {}\n\n", category));
        for mem in mems {
            let importance = mem
                .importance
                .map(|i| format!(" [i:{}]", i))
                .unwrap_or_default();
            md.push_str(&format!("- {}{}\n", mem.content.trim(), importance));
        }
        md.push('\n');
    }

    fs::write(output, &md)?;
    fs::remove_file(&temp_json).ok();

    println!(
        "Exported {} memories to {}",
        memories.len(),
        output.display()
    );
    Ok(())
}

async fn handle_import(file: PathBuf) -> Result<()> {
    let args = vec!["import".to_string(), file.to_string_lossy().to_string()];

    run_mmry(&args)
}

async fn handle_stats() -> Result<()> {
    run_mmry(&["stats".to_string()])
}

async fn handle_stores() -> Result<()> {
    // Don't use auto-store for listing stores
    run_mmry_raw(&["stores", "list"])
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

    if json {
        args.push("--json".to_string());
    }

    if full {
        args.push("--full".to_string());
    }

    run_mmry(&args)
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

/// Detect the agent harness from env vars set by the harness itself.
///
/// Pi's oqto-bridge extension sets:
///   PI_HARNESS=pi
///   PI_SESSION_ID=<uuid>
///   PI_SESSION_FILE=<path>
///   PI_MODEL=<provider>/<model>
///   PI_CWD=<workdir>
///
/// Other harnesses can follow a similar pattern with their own prefix
/// or a shared AGENT_HARNESS env var.
fn detect_agent() -> Option<AgentIdentity> {
    // Pi (via oqto-bridge extension)
    if let Ok(harness) = std::env::var("PI_HARNESS") {
        let session = std::env::var("PI_SESSION_ID").ok().filter(|s| !s.is_empty());
        let model = std::env::var("PI_MODEL").ok().filter(|s| !s.is_empty());
        return Some(AgentIdentity {
            harness,
            session,
            model,
        });
    }

    // opencode
    if std::env::var("OPENCODE").is_ok() {
        return Some(AgentIdentity {
            harness: "opencode".to_string(),
            session: None,
            model: None,
        });
    }

    // Generic fallback: AGENT_HARNESS env var
    if let Ok(harness) = std::env::var("AGENT_HARNESS") {
        return Some(AgentIdentity {
            harness,
            session: std::env::var("AGENT_SESSION_ID").ok(),
            model: std::env::var("AGENT_MODEL").ok(),
        });
    }

    None
}

fn run_mmry(args: &[String]) -> Result<()> {
    let mut full_args = Vec::new();

    // Auto-detect store from repo name
    if let Some(repo) = get_repo_name() {
        full_args.push("--store".to_string());
        full_args.push(repo);
    }

    full_args.extend(args.iter().cloned());

    let mut cmd = Command::new("mmry");
    cmd.args(&full_args);

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
            meta.insert("session".to_string(), serde_json::Value::String(session.clone()));
        }
        if let Some(ref model) = identity.model {
            meta.insert("model".to_string(), serde_json::Value::String(model.clone()));
        }
        if !meta.is_empty() {
            if let Ok(meta_json) = serde_json::to_string(&meta) {
                cmd.env("MMRY_AGENT_META", meta_json);
            }
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
