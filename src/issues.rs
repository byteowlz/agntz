use anyhow::{Context, Result};
use clap::Subcommand;
use std::process::Command;

#[derive(Subcommand)]
pub enum IssuesCommand {
    /// List all issues
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,
        /// Filter by type
        #[arg(short = 'T', long)]
        r#type: Option<String>,
    },

    /// Create a new issue
    Create {
        /// Issue title
        title: String,
        /// Issue type (bug, feature, task, epic, chore)
        #[arg(short = 'T', long, default_value = "task")]
        r#type: String,
        /// Priority (0-4, default 2)
        #[arg(short, long, default_value = "2")]
        priority: u8,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Update an issue
    Update {
        /// Issue ID
        id: String,
        /// New status
        #[arg(long)]
        status: Option<String>,
        /// New priority
        #[arg(long)]
        priority: Option<u8>,
    },

    /// Close an issue
    Close {
        /// Issue ID
        id: String,
        /// Reason for closing
        #[arg(short, long)]
        reason: Option<String>,
    },

    /// Show issue details
    Show {
        /// Issue ID
        id: String,
    },
}

pub async fn handle(command: Option<IssuesCommand>) -> Result<()> {
    match command {
        None => run_trx(&["list"]),
        Some(IssuesCommand::List { status, r#type }) => {
            let mut args = vec!["list"];
            let status_str;
            let type_str;

            if let Some(s) = &status {
                status_str = s.clone();
                args.push("--status");
                args.push(&status_str);
            }
            if let Some(t) = &r#type {
                type_str = t.clone();
                args.push("--issue-type");
                args.push(&type_str);
            }
            run_trx(&args)
        }
        Some(IssuesCommand::Create {
            title,
            r#type,
            priority,
            description,
        }) => {
            let mut args = vec!["create", &title, "-t", &r#type];
            let priority_str = priority.to_string();
            args.push("-p");
            args.push(&priority_str);

            let desc_str;
            if let Some(d) = &description {
                desc_str = d.clone();
                args.push("-d");
                args.push(&desc_str);
            }
            run_trx(&args)
        }
        Some(IssuesCommand::Update {
            id,
            status,
            priority,
        }) => {
            let mut args = vec!["update", &id];
            let status_str;
            let priority_str;

            if let Some(s) = &status {
                status_str = s.clone();
                args.push("--status");
                args.push(&status_str);
            }
            if let Some(p) = priority {
                priority_str = p.to_string();
                args.push("--priority");
                args.push(&priority_str);
            }
            run_trx(&args)
        }
        Some(IssuesCommand::Close { id, reason }) => {
            let mut args = vec!["close", &id];
            let reason_str;

            if let Some(r) = &reason {
                reason_str = r.clone();
                args.push("-r");
                args.push(&reason_str);
            }
            run_trx(&args)
        }
        Some(IssuesCommand::Show { id }) => run_trx(&["show", &id]),
    }
}

fn run_trx(args: &[&str]) -> Result<()> {
    let output = Command::new("trx")
        .args(args)
        .output()
        .context("failed to run trx - is trx installed?")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        // Don't fail on non-zero exit for trx (it might just mean no results)
    }

    Ok(())
}
