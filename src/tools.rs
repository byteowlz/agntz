use anyhow::{Context, Result};
use clap::Subcommand;
use std::process::Command;

#[derive(Subcommand)]
pub enum ToolsCommand {
    /// List available tools
    List,

    /// Install a tool
    Install {
        /// Tool name (mmry, mailz, bd, bv, cass, all)
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
        name: "mailz",
        description: "Agent coordination and messaging",
        binary: "mailz-cli",
        install_cmd: "cargo install mailz",
    },
    ToolInfo {
        name: "bd",
        description: "Issue tracking (beads)",
        binary: "bd",
        install_cmd: "cargo install beads",
    },
    ToolInfo {
        name: "bv",
        description: "Issue triage and analytics",
        binary: "bv",
        install_cmd: "cargo install beads-viewer",
    },
    ToolInfo {
        name: "cass",
        description: "Agent session history search",
        binary: "cass",
        install_cmd: "cargo install coding-agent-session-search",
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
        let status = if installed { "[installed]" } else { "[not installed]" };
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
            println!("Available: {}", TOOLS.iter().map(|t| t.name).collect::<Vec<_>>().join(", "));
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

fn handle_doctor() -> Result<()> {
    println!("Checking tool health...\n");
    
    let mut all_ok = true;
    
    for tool in TOOLS {
        let installed = is_installed(tool.binary);
        let status = if installed { "OK" } else { "MISSING" };
        let icon = if installed { "+" } else { "x" };
        
        println!("  [{}] {}: {}", icon, tool.name, status);
        
        if !installed {
            all_ok = false;
        }
    }
    
    println!();
    
    if all_ok {
        println!("All tools are installed and ready.");
    } else {
        println!("Some tools are missing. Install with: agntz tools install all");
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
