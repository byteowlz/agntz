use anyhow::Result;
use clap::Subcommand;

use crate::readout;

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

pub async fn handle(command: Option<IssuesCommand>, json: bool) -> Result<()> {
    match command {
        None => handle_list(None, None, json),
        Some(IssuesCommand::List { status, r#type }) => {
            handle_list(status.as_deref(), r#type.as_deref(), json)
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
            run_trx(&args, json, "tasks/create")
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
            run_trx(&args, json, "tasks/update")
        }
        Some(IssuesCommand::Close { id, reason }) => {
            let mut args = vec!["close", &id];
            let reason_str;

            if let Some(r) = &reason {
                reason_str = r.clone();
                args.push("-r");
                args.push(&reason_str);
            }
            run_trx(&args, json, "tasks/close")
        }
        Some(IssuesCommand::Show { id }) => run_trx(&["show", &id], json, "tasks/show"),
    }
}

fn handle_list(status: Option<&str>, issue_type: Option<&str>, json: bool) -> Result<()> {
    let mut args = vec!["list".to_string(), "--json".to_string()];
    if let Some(s) = status {
        args.push("--status".to_string());
        args.push(s.to_string());
    }
    if let Some(t) = issue_type {
        args.push("--issue-type".to_string());
        args.push(t.to_string());
    }

    let (ok, out, err) = readout::run("trx", &args);
    if json {
        readout::emit(
            "tasks/list",
            ok,
            (!ok).then(|| format!("trx failed: {err}")),
            readout::parse_or_text(out),
        );
        return Ok(());
    }
    print_plain(&out, &err);
    Ok(())
}

fn run_trx(args: &[&str], json: bool, verb: &str) -> Result<()> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (ok, out, err) = readout::run("trx", &args);
    if json {
        readout::emit(
            verb,
            ok,
            (!ok).then(|| format!("trx failed: {err}")),
            readout::parse_or_text(out),
        );
        return Ok(());
    }
    print_plain(&out, &err);
    Ok(())
}

fn print_plain(out: &str, err: &str) {
    print!("{out}");
    if !err.is_empty() {
        eprint!("{err}");
    }
}

// Keep the lane open for write verbs that want to look at the shape.
