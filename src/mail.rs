use anyhow::{Context, Result};
use clap::Subcommand;
use std::process::Command;

#[derive(Subcommand)]
pub enum MailCommand {
    /// Check inbox
    Inbox,

    /// Send a message
    Send {
        /// Recipient
        to: String,
        /// Subject
        subject: String,
        /// Message body
        #[arg(short, long)]
        body: Option<String>,
    },

    /// Read a message
    Read {
        /// Message ID
        id: String,
    },

    /// Acknowledge a message
    Ack {
        /// Message ID
        id: String,
    },

    /// Search messages
    Search {
        /// Search query
        query: String,
    },
}

pub async fn handle(command: MailCommand) -> Result<()> {
    match command {
        MailCommand::Inbox => run_mailz(&["inbox"]),
        MailCommand::Send { to, subject, body } => {
            let mut args = vec!["send", &to, &subject];
            let body_str;
            if let Some(b) = &body {
                body_str = b.clone();
                args.push("--body");
                args.push(&body_str);
            }
            run_mailz(&args)
        }
        MailCommand::Read { id } => run_mailz(&["read", &id]),
        MailCommand::Ack { id } => run_mailz(&["ack", &id]),
        MailCommand::Search { query } => run_mailz(&["search", &query]),
    }
}

fn run_mailz(args: &[&str]) -> Result<()> {
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
