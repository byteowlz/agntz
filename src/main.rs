use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

mod issues;
mod mail;
mod memory;
mod schedule;
mod tools;

use issues::IssuesCommand;
use mail::MailCommand;
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
    /// Memory operations (wraps mmry)
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },

    /// Mail and messaging (wraps mailz)
    Mail {
        #[command(subcommand)]
        command: MailCommand,
    },

    /// File reservations
    Reserve {
        /// Files to reserve
        files: Vec<PathBuf>,
        /// Reason for reservation
        #[arg(short, long)]
        reason: Option<String>,
        /// TTL in seconds
        #[arg(long, default_value = "1800")]
        ttl: u32,
    },

    /// List active reservations
    Reservations,

    /// Release file reservations
    Release {
        /// Files to release (or --all)
        files: Vec<PathBuf>,
        /// Release all reservations
        #[arg(long)]
        all: bool,
    },

    /// List issues (wraps trx list)
    Issues {
        #[command(subcommand)]
        command: Option<IssuesCommand>,
    },

    /// Show unblocked issues (wraps trx ready)
    Ready,

    /// Search agent session history (wraps cass)
    Search {
        /// Search query
        query: String,
        /// Limit to specific repo context
        #[arg(short, long)]
        repo: Option<String>,
        /// Limit to last N days
        #[arg(long)]
        days: Option<u32>,
    },

    /// Manage agent tools
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },

    /// Task scheduling (wraps skdlr)
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommand,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },

    /// Initialize agntz for current repo (mmry store, mailz, AGENTS.md)
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
        Commands::Mail { command } => mail::handle(command).await,
        Commands::Reserve { files, reason, ttl } => handle_reserve(files, reason, ttl).await,
        Commands::Reservations => handle_reservations().await,
        Commands::Release { files, all } => handle_release(files, all).await,
        Commands::Issues { command } => issues::handle(command).await,
        Commands::Ready => handle_ready().await,
        Commands::Search { query, repo, days } => handle_search(query, repo, days).await,
        Commands::Tools { command } => tools::handle(command).await,
        Commands::Schedule { command } => schedule::handle(command).await,
        Commands::Completions { shell } => handle_completions(shell),
        Commands::Init { force } => handle_init(force).await,
    }
}

async fn handle_reserve(files: Vec<PathBuf>, reason: Option<String>, ttl: u32) -> Result<()> {
    let mut args = vec!["reserve".to_string()];

    for file in &files {
        args.push(file.to_string_lossy().to_string());
    }

    args.push("--ttl".to_string());
    args.push(ttl.to_string());

    if let Some(reason) = reason {
        args.push("--reason".to_string());
        args.push(reason);
    }

    run_mailz_cli(&args)
}

async fn handle_reservations() -> Result<()> {
    run_mailz_cli(&["reservations".to_string()])
}

async fn handle_release(files: Vec<PathBuf>, all: bool) -> Result<()> {
    let mut args = vec!["release".to_string()];

    if all {
        args.push("--all".to_string());
    } else {
        for file in &files {
            args.push(file.to_string_lossy().to_string());
        }
    }

    run_mailz_cli(&args)
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

async fn handle_search(query: String, repo: Option<String>, days: Option<u32>) -> Result<()> {
    let mut args = vec!["search".to_string(), query];

    if let Some(repo) = repo {
        args.push("--workspace".to_string());
        args.push(repo);
    }

    if let Some(days) = days {
        args.push("--days".to_string());
        args.push(days.to_string());
    }

    let output = Command::new("cass")
        .args(&args)
        .output()
        .context("failed to run cass - is coding_agent_session_search installed?")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

fn handle_completions(shell: clap_complete::Shell) -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::generate;
    use std::io;

    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "agntz", &mut io::stdout());
    Ok(())
}

fn run_mailz_cli(args: &[String]) -> Result<()> {
    let output = Command::new("mailz-cli")
        .args(args)
        .output()
        .context("failed to run mailz-cli - is mailz installed?")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        anyhow::bail!("mailz-cli failed");
    }

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
    println!("\n[1/4] Initializing mmry store...");
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

    // 2. Initialize mailz
    println!("[2/4] Initializing mailz...");
    let mut mailz_args = vec!["init"];
    if force {
        mailz_args.push("--force");
    }
    let mailz_output = Command::new("mailz-cli")
        .args(&mailz_args)
        .output()
        .context("failed to run mailz-cli init - is mailz installed?")?;

    if !mailz_output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&mailz_output.stdout));
    }
    if !mailz_output.stderr.is_empty() && !mailz_output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&mailz_output.stderr));
    }

    // 3. Initialize trx
    println!("[3/4] Initializing trx...");
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

    // 4. Append to AGENTS.md
    println!("[4/4] Updating AGENTS.md...");
    let agents_md = PathBuf::from("AGENTS.md");
    let agntz_section = format!(
        r#"
## agntz

Use agntz for memory and coordination:

```bash
agntz memory search "topic"    # Find relevant context
agntz memory add "insight" -c category
agntz memory list
agntz mail inbox               # Check messages
agntz reserve <file>           # Before editing shared files
agntz release <file>           # When done
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
