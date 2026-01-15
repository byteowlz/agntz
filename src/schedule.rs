use anyhow::{Context, Result};
use clap::Subcommand;
use std::process::Command;

#[derive(Subcommand)]
pub enum ScheduleCommand {
    /// Add a new scheduled task
    Add {
        /// Schedule name (identifier)
        name: String,
        /// Cron expression (e.g., "0 8 * * *" for daily at 8am)
        #[arg(short, long)]
        schedule: String,
        /// Command to execute
        #[arg(short, long)]
        command: String,
        /// Working directory
        #[arg(short, long)]
        workdir: Option<String>,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Start disabled
        #[arg(long)]
        disabled: bool,
    },

    /// List all schedules
    List {
        /// Filter by status (enabled/disabled)
        #[arg(long)]
        status: Option<String>,
    },

    /// Show schedule details
    Show {
        /// Schedule name
        name: String,
    },

    /// Edit an existing schedule
    Edit {
        /// Schedule name
        name: String,
        /// New cron expression
        #[arg(short, long)]
        schedule: Option<String>,
        /// New command
        #[arg(short, long)]
        command: Option<String>,
        /// New working directory
        #[arg(short, long)]
        workdir: Option<String>,
        /// New description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Remove a schedule
    Remove {
        /// Schedule name
        name: String,
        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Enable a schedule
    Enable {
        /// Schedule name
        name: String,
    },

    /// Disable a schedule
    Disable {
        /// Schedule name
        name: String,
    },

    /// Trigger an immediate run
    Run {
        /// Schedule name
        name: String,
        /// Show what would run without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// View execution history
    Logs {
        /// Schedule name
        name: String,
        /// Number of runs to show
        #[arg(long, default_value = "10")]
        last: usize,
    },

    /// Show status overview
    Status,

    /// Show upcoming runs
    Next,

    /// Show active backend
    Backend,

    /// Health check
    Doctor,
}

pub async fn handle(command: ScheduleCommand) -> Result<()> {
    match command {
        ScheduleCommand::Add {
            name,
            schedule,
            command,
            workdir,
            description,
            disabled,
        } => handle_add(name, schedule, command, workdir, description, disabled).await,
        ScheduleCommand::List { status } => handle_list(status).await,
        ScheduleCommand::Show { name } => handle_show(name).await,
        ScheduleCommand::Edit {
            name,
            schedule,
            command,
            workdir,
            description,
        } => handle_edit(name, schedule, command, workdir, description).await,
        ScheduleCommand::Remove { name, yes } => handle_remove(name, yes).await,
        ScheduleCommand::Enable { name } => handle_enable(name).await,
        ScheduleCommand::Disable { name } => handle_disable(name).await,
        ScheduleCommand::Run { name, dry_run } => handle_run(name, dry_run).await,
        ScheduleCommand::Logs { name, last } => handle_logs(name, last).await,
        ScheduleCommand::Status => handle_status().await,
        ScheduleCommand::Next => handle_next().await,
        ScheduleCommand::Backend => handle_backend().await,
        ScheduleCommand::Doctor => handle_doctor().await,
    }
}

async fn handle_add(
    name: String,
    schedule: String,
    command: String,
    workdir: Option<String>,
    description: Option<String>,
    disabled: bool,
) -> Result<()> {
    let mut args = vec![
        "add".to_string(),
        name,
        "--schedule".to_string(),
        schedule,
        "--command".to_string(),
        command,
    ];

    if let Some(w) = workdir {
        args.push("--workdir".to_string());
        args.push(w);
    }

    if let Some(d) = description {
        args.push("--description".to_string());
        args.push(d);
    }

    if disabled {
        args.push("--enabled".to_string());
        args.push("false".to_string());
    }

    run_skdlr(&args)
}

async fn handle_list(status: Option<String>) -> Result<()> {
    let mut args = vec!["list".to_string()];

    if let Some(s) = status {
        args.push("--status".to_string());
        args.push(s);
    }

    run_skdlr(&args)
}

async fn handle_show(name: String) -> Result<()> {
    run_skdlr(&["show".to_string(), name])
}

async fn handle_edit(
    name: String,
    schedule: Option<String>,
    command: Option<String>,
    workdir: Option<String>,
    description: Option<String>,
) -> Result<()> {
    let mut args = vec!["edit".to_string(), name];

    if let Some(s) = schedule {
        args.push("--schedule".to_string());
        args.push(s);
    }

    if let Some(c) = command {
        args.push("--command".to_string());
        args.push(c);
    }

    if let Some(w) = workdir {
        args.push("--workdir".to_string());
        args.push(w);
    }

    if let Some(d) = description {
        args.push("--description".to_string());
        args.push(d);
    }

    run_skdlr(&args)
}

async fn handle_remove(name: String, yes: bool) -> Result<()> {
    let mut args = vec!["remove".to_string(), name];

    if yes {
        args.push("--yes".to_string());
    }

    run_skdlr(&args)
}

async fn handle_enable(name: String) -> Result<()> {
    run_skdlr(&["enable".to_string(), name])
}

async fn handle_disable(name: String) -> Result<()> {
    run_skdlr(&["disable".to_string(), name])
}

async fn handle_run(name: String, dry_run: bool) -> Result<()> {
    let mut args = vec!["run".to_string(), name];

    if dry_run {
        args.push("--dry-run".to_string());
    }

    run_skdlr(&args)
}

async fn handle_logs(name: String, last: usize) -> Result<()> {
    run_skdlr(&[
        "logs".to_string(),
        name,
        "--last".to_string(),
        last.to_string(),
    ])
}

async fn handle_status() -> Result<()> {
    run_skdlr(&["status".to_string()])
}

async fn handle_next() -> Result<()> {
    run_skdlr(&["next".to_string()])
}

async fn handle_backend() -> Result<()> {
    run_skdlr(&["backend".to_string()])
}

async fn handle_doctor() -> Result<()> {
    run_skdlr(&["doctor".to_string()])
}

fn run_skdlr(args: &[String]) -> Result<()> {
    let output = Command::new("skdlr")
        .args(args)
        .output()
        .context("failed to run skdlr - is skdlr installed?")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        anyhow::bail!("skdlr command failed");
    }

    Ok(())
}
