use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

mod issues;
mod memory;
mod schedule;
mod tools;

use issues::IssuesCommand;
use memory::MemoryCommand;
use schedule::ScheduleCommand;
use tools::ToolsCommand;

#[derive(Parser)]
#[command(name = "agntz")]
#[command(about = "Agent utility toolkit for AI coding agents")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Memory operations
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },

    /// Task tracking
    #[command(alias = "issues")]
    Tasks {
        #[command(subcommand)]
        command: Option<IssuesCommand>,
    },

    /// Show unblocked tasks
    Ready,

    /// Search agent session history
    Search {
        /// Search query
        query: String,
        /// Limit to specific workspace path (defaults to current repo/dir)
        #[arg(short, long, alias = "repo")]
        workspace: Option<String>,
        /// Limit to last N days
        #[arg(long)]
        days: Option<u32>,
        /// Limit to a specific session/conversation ID
        #[arg(long)]
        session: Option<String>,
        /// Maximum results to return
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Search all workspaces (disables default workspace filter)
        #[arg(long)]
        all_workspaces: bool,
        /// Include tool calls/results
        #[arg(long)]
        include_tools: bool,
        /// Include system context (AGENTS.md, etc.)
        #[arg(long)]
        include_system: bool,
        /// Disable result deduplication
        #[arg(long)]
        no_dedup: bool,
        /// Output raw JSON results
        #[arg(long)]
        json: bool,
    },

    /// Manage agent tools
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },

    /// Task scheduling
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommand,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },

    /// Initialize agntz for current repo (mmry store, AGENTS.md)
    Init {
        /// Force re-initialization
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Memory { command } => memory::handle(command).await,
        Commands::Tasks { command } => issues::handle(command).await,
        Commands::Ready => handle_ready().await,
        Commands::Search {
            query,
            workspace,
            days,
            session,
            limit,
            all_workspaces,
            include_tools,
            include_system,
            no_dedup,
            json,
        } => {
            handle_search(
                query,
                workspace,
                days,
                session,
                limit,
                all_workspaces,
                include_tools,
                include_system,
                no_dedup,
                json,
            )
            .await
        }
        Commands::Tools { command } => tools::handle(command).await,
        Commands::Schedule { command } => schedule::handle(command).await,
        Commands::Completions { shell } => handle_completions(shell),
        Commands::Init { force } => handle_init(force).await,
    }
}

async fn handle_ready() -> Result<()> {
    let output = Command::new("trx")
        .arg("ready")
        .output()
        .context("failed to run trx ready - is trx installed?")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

#[derive(serde::Deserialize)]
struct HstryJsonResponse<T> {
    ok: bool,
    result: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct HstrySearchHit {
    message_id: String,
    conversation_id: String,
    message_idx: i32,
    role: String,
    content: String,
    snippet: String,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    conv_created_at: chrono::DateTime<chrono::Utc>,
    conv_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    score: f32,
    source_id: String,
    external_id: Option<String>,
    title: Option<String>,
    workspace: Option<String>,
    source_adapter: String,
    source_path: Option<String>,
    host: Option<String>,
}

async fn handle_search(
    query: String,
    workspace: Option<String>,
    days: Option<u32>,
    session: Option<String>,
    limit: usize,
    all_workspaces: bool,
    include_tools: bool,
    include_system: bool,
    no_dedup: bool,
    json: bool,
) -> Result<()> {
    let mut args = vec!["search".to_string(), query.clone(), "--json".to_string()];

    let workspace_filter = if all_workspaces {
        None
    } else {
        workspace.or_else(resolve_default_workspace)
    };

    if let Some(workspace) = workspace_filter.as_ref() {
        args.push("--workspace".to_string());
        args.push(workspace.clone());
    }

    let dedup = !no_dedup;
    if dedup {
        args.push("--dedup".to_string());
    }
    if !include_tools {
        args.push("--no-tools".to_string());
    }
    if include_system {
        args.push("--include-system".to_string());
    }

    let fetch_limit = if session.is_some() {
        (limit.saturating_mul(10)).clamp(limit.max(20), 1000)
    } else {
        limit
    };
    args.push("--limit".to_string());
    args.push(fetch_limit.to_string());

    let output = Command::new("hstry")
        .args(&args)
        .output()
        .context("failed to run hstry - is hstry installed and the service running?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("hstry search failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: HstryJsonResponse<Vec<HstrySearchHit>> =
        serde_json::from_str(&stdout).context("failed to parse hstry search output")?;

    if !response.ok {
        let error = response
            .error
            .unwrap_or_else(|| "hstry search failed".to_string());
        anyhow::bail!(error);
    }

    let mut hits = response.result.unwrap_or_default();
    hits = filter_hits(hits, session.as_deref(), days);
    hits.truncate(limit);

    if json {
        let payload = serde_json::json!({ "hits": hits });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    print_compact_hits(&hits);
    Ok(())
}

fn resolve_default_workspace() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }

    std::env::current_dir()
        .ok()
        .map(|dir| dir.to_string_lossy().to_string())
}

fn filter_hits(
    hits: Vec<HstrySearchHit>,
    session: Option<&str>,
    days: Option<u32>,
) -> Vec<HstrySearchHit> {
    let mut filtered = Vec::new();
    let cutoff = days.map(|d| chrono::Utc::now() - chrono::Duration::days(i64::from(d)));

    for hit in hits {
        if let Some(session_id) = session {
            let session_match = hit
                .external_id
                .as_deref()
                .map(|id| id == session_id)
                .unwrap_or(false)
                || hit.conversation_id == session_id
                || hit
                    .source_path
                    .as_deref()
                    .map(|path| path.contains(session_id))
                    .unwrap_or(false);
            if !session_match {
                continue;
            }
        }

        if let Some(cutoff) = cutoff {
            let timestamp = hit
                .created_at
                .or(hit.conv_updated_at)
                .unwrap_or(hit.conv_created_at);
            if timestamp < cutoff {
                continue;
            }
        }

        filtered.push(hit);
    }

    filtered
}

fn print_compact_hits(hits: &[HstrySearchHit]) {
    if hits.is_empty() {
        println!("No results found.");
        return;
    }

    for hit in hits {
        let session_id = hit
            .external_id
            .as_deref()
            .unwrap_or(hit.conversation_id.as_str());
        let title = compact_label(hit.title.as_deref().unwrap_or("Untitled"), 40);
        let snippet = compact_snippet(&hit.snippet, 160);
        let workspace = hit
            .workspace
            .as_deref()
            .and_then(|w| w.split('/').last())
            .unwrap_or("-");

        println!(
            "{score:>5.2} {source} {role} {session} #{idx} {workspace} {title} - {snippet}",
            score = hit.score,
            source = hit.source_id,
            role = hit.role,
            session = session_id,
            idx = hit.message_idx,
            workspace = workspace,
            title = title
        );
    }
}

fn compact_snippet(snippet: &str, max_len: usize) -> String {
    let mut collapsed = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.len() > max_len {
        collapsed.truncate(max_len.saturating_sub(3));
        collapsed.push_str("...");
    }
    collapsed
}

fn compact_label(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    let mut trimmed = value.to_string();
    trimmed.truncate(max_len.saturating_sub(3));
    trimmed.push_str("...");
    trimmed
}

fn handle_completions(shell: clap_complete::Shell) -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::generate;
    use std::io;

    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "agntz", &mut io::stdout());
    Ok(())
}

/// Get the current repo name from git remote or directory name
fn get_repo_name() -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout);
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

    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
}

async fn handle_init(force: bool) -> Result<()> {
    let repo_name = get_repo_name().context("could not determine repo name")?;
    println!("Initializing agntz for repo: {}", repo_name);

    // 1. Initialize mmry with repo-specific store
    println!("\n[1/3] Initializing mmry store...");
    let mut mmry_args = vec!["init", "--store", &repo_name];
    if force {
        mmry_args.push("--force");
    }
    let mmry_output = Command::new("mmry")
        .args(&mmry_args)
        .output()
        .context("failed to run mmry init - is mmry installed?")?;

    if !mmry_output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&mmry_output.stdout));
    }
    if !mmry_output.stderr.is_empty() && !mmry_output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&mmry_output.stderr));
    }

    // 2. Initialize trx
    println!("[2/3] Initializing trx...");
    let trx_args = vec!["init", "--prefix", &repo_name];
    let trx_output = Command::new("trx")
        .args(&trx_args)
        .output()
        .context("failed to run trx init - is trx installed?")?;

    if !trx_output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&trx_output.stdout));
    }
    if !trx_output.stderr.is_empty() && !trx_output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&trx_output.stderr));
    }

    // 3. Append to AGENTS.md
    println!("[3/3] Updating AGENTS.md...");
    let agents_md = PathBuf::from("AGENTS.md");
    let agntz_section = format!(
        r#"
## agntz

Use agntz for memory:

```bash
agntz memory search "topic"    # Find relevant context
agntz memory add "insight" -c category
agntz memory list
```
"#
    );

    if agents_md.exists() {
        let content = fs::read_to_string(&agents_md)?;
        if content.contains("## agntz") {
            if force {
                // Remove existing section and re-add
                let new_content = remove_agntz_section(&content);
                fs::write(
                    &agents_md,
                    format!("{}{}", new_content.trim_end(), agntz_section),
                )?;
                println!("  Updated existing agntz section in AGENTS.md");
            } else {
                println!("  AGENTS.md already contains agntz section (use --force to update)");
            }
        } else {
            fs::write(
                &agents_md,
                format!("{}{}", content.trim_end(), agntz_section),
            )?;
            println!("  Appended agntz section to AGENTS.md");
        }
    } else {
        fs::write(
            &agents_md,
            format!("# Agent Instructions\n{}", agntz_section),
        )?;
        println!("  Created AGENTS.md with agntz section");
    }

    println!("\nDone! agntz initialized for '{}'", repo_name);
    Ok(())
}

fn remove_agntz_section(content: &str) -> String {
    let mut result = String::new();
    let mut in_agntz_section = false;

    for line in content.lines() {
        if line.starts_with("## agntz") {
            in_agntz_section = true;
            continue;
        }
        if in_agntz_section && line.starts_with("## ") {
            in_agntz_section = false;
        }
        if !in_agntz_section {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}
