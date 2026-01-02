use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;

mod memory;
mod mail;
mod issues;
mod tools;

use memory::MemoryCommand;
use mail::MailCommand;
use issues::IssuesCommand;
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

    /// List issues (wraps bd list)
    Issues {
        #[command(subcommand)]
        command: Option<IssuesCommand>,
    },

    /// Show unblocked issues (wraps bd ready)
    Ready,

    /// Cross-repo issue triage (wraps bv)
    Triage {
        /// Show next recommended item
        #[arg(long)]
        next: bool,
        /// Refresh workspace.yaml first
        #[arg(long)]
        refresh: bool,
    },

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

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
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
        Commands::Triage { next, refresh } => handle_triage(next, refresh).await,
        Commands::Search { query, repo, days } => handle_search(query, repo, days).await,
        Commands::Tools { command } => tools::handle(command).await,
        Commands::Completions { shell } => handle_completions(shell),
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
    let output = Command::new("bd")
        .arg("ready")
        .output()
        .context("failed to run bd ready - is beads installed?")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

async fn handle_triage(next: bool, refresh: bool) -> Result<()> {
    let mut args = vec![];

    if refresh {
        args.push("--refresh");
    }

    if next {
        args.push("-robot-next");
    } else {
        args.push("-robot-triage");
    }

    let output = Command::new("bv")
        .args(&args)
        .output()
        .context("failed to run bv - is beads_viewer installed?")?;

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
