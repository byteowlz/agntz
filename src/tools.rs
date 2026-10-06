use anyhow::{Context, Result};
use clap::Subcommand;
use std::process::Command;

#[derive(Subcommand)]
pub enum ToolsCommand {
    /// List available tools
    List,

    /// Install a tool
    Install {
        /// Tool name (mmry, trx, hstry, all)
        tool: String,
    },

    /// Update a tool
    Update {
        /// Tool name (or "all")
        tool: String,
    },

    /// Check tool health
    Doctor,
}

struct ToolInfo {
    name: &'static str,
    description: &'static str,
    binary: &'static str,
    install_cmd: &'static str,
}

const TOOLS: &[ToolInfo] = &[
    ToolInfo {
        name: "mmry",
        description: "Memory storage and search",
        binary: "mmry",
        install_cmd: "cargo install mmry-cli",
    },
    ToolInfo {
        name: "trx",
        description: "Issue tracking",
        binary: "trx",
        install_cmd: "cargo install trx",
    },
    ToolInfo {
        name: "hstry",
        description: "Agent session history search",
        binary: "hstry",
        install_cmd: "cargo install hstry-cli",
    },
];

pub async fn handle(command: ToolsCommand) -> Result<()> {
    match command {
        ToolsCommand::List => handle_list(),
        ToolsCommand::Install { tool } => handle_install(&tool).await,
        ToolsCommand::Update { tool } => handle_update(&tool).await,
        ToolsCommand::Doctor => handle_doctor(),
    }
}

fn handle_list() -> Result<()> {
    println!("Available tools:\n");

    for tool in TOOLS {
        let installed = is_installed(tool.binary);
        let status = if installed {
            "[installed]"
        } else {
            "[not installed]"
        };
        println!("  {} {} - {}", tool.name, status, tool.description);
    }

    println!("\nInstall with: agntz tools install <name>");
    println!("Install all:  agntz tools install all");

    Ok(())
}

async fn handle_install(tool: &str) -> Result<()> {
    if tool == "all" {
        for t in TOOLS {
            install_tool(t).await?;
        }
        return Ok(());
    }

    let tool_info = TOOLS.iter().find(|t| t.name == tool);
    match tool_info {
        Some(t) => install_tool(t).await,
        None => {
            println!("Unknown tool: {}", tool);
            println!(
                "Available: {}",
                TOOLS.iter().map(|t| t.name).collect::<Vec<_>>().join(", ")
            );
            Ok(())
        }
    }
}

async fn install_tool(tool: &ToolInfo) -> Result<()> {
    if is_installed(tool.binary) {
        println!("{} is already installed", tool.name);
        return Ok(());
    }

    println!("Installing {}...", tool.name);

    let parts: Vec<&str> = tool.install_cmd.split_whitespace().collect();
    let output = Command::new(parts[0])
        .args(&parts[1..])
        .status()
        .context(format!("failed to run: {}", tool.install_cmd))?;

    if output.success() {
        println!("{} installed successfully", tool.name);
    } else {
        println!("{} installation failed", tool.name);
    }

    Ok(())
}

async fn handle_update(tool: &str) -> Result<()> {
    if tool == "all" {
        for t in TOOLS {
            if is_installed(t.binary) {
                update_tool(t).await?;
            }
        }
        return Ok(());
    }

    let tool_info = TOOLS.iter().find(|t| t.name == tool);
    match tool_info {
        Some(t) => update_tool(t).await,
        None => {
            println!("Unknown tool: {}", tool);
            Ok(())
        }
    }
}

async fn update_tool(tool: &ToolInfo) -> Result<()> {
    println!("Updating {}...", tool.name);

    // For cargo-installed tools, reinstall with --force
    let parts: Vec<&str> = tool.install_cmd.split_whitespace().collect();
    let mut args: Vec<&str> = parts[1..].to_vec();
    args.push("--force");

    let output = Command::new(parts[0])
        .args(&args)
        .status()
        .context(format!("failed to update {}", tool.name))?;

    if output.success() {
        println!("{} updated successfully", tool.name);
    } else {
        println!("{} update failed", tool.name);
    }

    Ok(())
}

/// A real (cheap) invocation per tool, so doctor reports *protocol* health —
/// binary presence alone once reported all-green while both integrations were
/// dead against the installed tool versions. The contract is: the tool runs and
/// returns parseable JSON when we ask for `--json`.
fn probe_tool(binary: &str) -> Result<(), String> {
    let args: &[&str] = match binary {
        // Store/ledger health view (mmry 0.14).
        "mmry" => &["doctor", "--json"],
        // Issue listing (works in and out of a repo; --json proves the contract).
        "trx" => &["list", "--json", "--limit", "1"],
        // History search: an absent service is an availability problem, but a
        // retired flag/shape fails here.
        "hstry" => &["search", "", "--limit", "1", "--json"],
        _ => &["--help"],
    };

    let output = Command::new(binary)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run {binary}: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let detail = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(detail);
    }

    // Success + JSON we asked for = the installed version speaks our contract.
    if args.contains(&"--json") {
        return serde_json::from_str::<serde_json::Value>(stdout.trim())
            .map(|_| ())
            .map_err(|_| {
                format!(
                    "did not return JSON (protocol drift): {}",
                    stdout.lines().next().unwrap_or("").trim()
                )
            });
    }

    Ok(())
}

fn handle_doctor() -> Result<()> {
    println!("Checking tool health...\n");

    let mut all_ok = true;

    for tool in TOOLS {
        let installed = is_installed(tool.binary);
        if !installed {
            println!("  [x] {}: MISSING", tool.name);
            all_ok = false;
            continue;
        }

        match probe_tool(tool.binary) {
            Ok(()) => println!("  [+] {}: OK", tool.name),
            Err(detail) => {
                println!(
                    "  [x] {}: FAILED (installed, but the integration is broken)",
                    tool.name
                );
                for line in detail.lines().take(3) {
                    println!("        {line}");
                }
                all_ok = false;
            }
        }
    }

    println!();

    if all_ok {
        println!("All tools are installed and healthy.");
    } else {
        println!(
            "Some tools are missing or incompatible. Install with: agntz tools install all\n\
             A tool that is installed but FAILED means the version on PATH no longer \
             matches what agntz expects — update it (agntz tools update <tool>)."
        );
    }

    Ok(())
}

fn is_installed(binary: &str) -> bool {
    Command::new("which")
        .arg(binary)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
