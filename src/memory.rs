use anyhow::{Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;
use std::fs;

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
        MemoryCommand::Export { output, format, all } => handle_export(output, format, all).await,
        MemoryCommand::Import { file } => handle_import(file).await,
        MemoryCommand::Stats => handle_stats().await,
        MemoryCommand::Stores => handle_stores().await,
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
    let memories: Vec<Memory> = serde_json::from_str(&json_content)
        .unwrap_or_default();

    let mut md = String::new();
    md.push_str("# Memories\n\n");

    // Group by category
    let mut by_category: std::collections::HashMap<String, Vec<&Memory>> = std::collections::HashMap::new();
    for mem in &memories {
        let cat = mem.category.clone().unwrap_or_else(|| "uncategorized".to_string());
        by_category.entry(cat).or_default().push(mem);
    }

    for (category, mems) in by_category {
        md.push_str(&format!("## {}\n\n", category));
        for mem in mems {
            let importance = mem.importance.map(|i| format!(" [i:{}]", i)).unwrap_or_default();
            md.push_str(&format!("- {}{}\n", mem.content.trim(), importance));
        }
        md.push('\n');
    }

    fs::write(output, &md)?;
    fs::remove_file(&temp_json).ok();

    println!("Exported {} memories to {}", memories.len(), output.display());
    Ok(())
}

async fn handle_import(file: PathBuf) -> Result<()> {
    let args = vec![
        "import".to_string(),
        file.to_string_lossy().to_string(),
    ];

    run_mmry(&args)
}

async fn handle_stats() -> Result<()> {
    run_mmry(&["stats".to_string()])
}

async fn handle_stores() -> Result<()> {
    run_mmry(&["stores".to_string()])
}

fn run_mmry(args: &[String]) -> Result<()> {
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
