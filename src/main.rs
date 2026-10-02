//! agntz — Agent utility toolkit for AI coding agents.
//!
//! One front door to an agent's operative memory, also seeing the fleet. Reads
//! `AGENT_CTX` to orient, and delegates to the underlying stores (mmry, trx,
//! hstry, skdlr) behind a stable unified `--json` surface.

mod board;
mod ctx;
mod gitx;
mod gvnr;
mod issues;
mod mcp;
mod memory;
mod readout;
mod schedule;
mod tools;
mod wiki;

use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use log::LevelFilter;

use agntz::config::{AppConfig, AppPaths, load_or_init_config, write_default_config};

use board::BoardCommand;
use issues::IssuesCommand;
use memory::MemoryCommand;
use schedule::ScheduleCommand;
use tools::ToolsCommand;
use wiki::WikiCommand;

#[derive(Parser)]
#[command(
    name = "agntz",
    about = "Agent utility toolkit for AI coding agents",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(flatten)]
    common: CommonOpts,
    #[command(subcommand)]
    command: Commands,
}

/// Common global options shared across all subcommands.
#[derive(Debug, Clone, Args)]
pub struct CommonOpts {
    /// Override the config file path.
    #[arg(
        long,
        value_name = "PATH",
        global = true,
        help_heading = "Global options"
    )]
    pub config: Option<PathBuf>,
    /// Reduce output to only errors.
    #[arg(short, long, action = clap::ArgAction::SetTrue, global = true, help_heading = "Global options")]
    pub quiet: bool,
    /// Increase logging verbosity (stackable).
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true, help_heading = "Global options")]
    pub verbose: u8,
    /// Enable debug logging (equivalent to -vv).
    #[arg(long, global = true, help_heading = "Global options")]
    pub debug: bool,
    /// Enable trace logging (overrides other levels).
    #[arg(long, global = true, help_heading = "Global options")]
    pub trace: bool,
    /// Output machine-readable JSON.
    #[arg(long, global = true, help_heading = "Global options")]
    pub json: bool,
    /// Disable ANSI colors in output.
    #[arg(long = "no-color", global = true, conflicts_with = "color")]
    pub no_color: bool,
    /// Control color output (auto, always, never).
    #[arg(long, value_enum, default_value_t = ColorOption::Auto, global = true, help_heading = "Global options")]
    pub color: ColorOption,
    /// Do not change anything on disk.
    #[arg(long = "dry-run", global = true, help_heading = "Global options")]
    pub dry_run: bool,
    /// Assume "yes" for interactive prompts.
    #[arg(
        short = 'y',
        long = "yes",
        global = true,
        help_heading = "Global options"
    )]
    pub assume_yes: bool,
    /// Never prompt for input; fail if confirmation would be required.
    #[arg(long = "no-input", global = true, help_heading = "Global options")]
    pub no_input: bool,
    /// Maximum seconds to allow an operation to run.
    #[arg(
        long = "timeout",
        value_name = "SECONDS",
        global = true,
        help_heading = "Global options"
    )]
    pub timeout: Option<u64>,
    /// Override the degree of parallelism.
    #[arg(
        long = "parallel",
        value_name = "N",
        global = true,
        help_heading = "Global options"
    )]
    pub parallel: Option<usize>,
    /// Disable progress indicators.
    #[arg(long = "no-progress", global = true, help_heading = "Global options")]
    pub no_progress: bool,
    /// Emit additional diagnostics for troubleshooting.
    #[arg(long = "diagnostics", global = true, help_heading = "Global options")]
    pub diagnostics: bool,
}

/// Color output mode.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorOption {
    /// Detect terminal capabilities automatically.
    Auto,
    /// Always emit ANSI color codes.
    Always,
    /// Never emit ANSI color codes.
    Never,
}

#[derive(Subcommand)]
enum Commands {
    /// Memory operations.
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },

    /// Task tracking.
    #[command(alias = "issues")]
    Tasks {
        #[command(subcommand)]
        command: Option<IssuesCommand>,
    },

    /// Show unblocked tasks.
    Ready,

    /// Search agent session history.
    Search {
        /// Search query.
        query: String,
        /// Limit to a specific workspace path (defaults to current repo/dir).
        #[arg(short, long, alias = "repo")]
        workspace: Option<String>,
        /// Limit to the last N days.
        #[arg(long)]
        days: Option<u32>,
        /// Limit to a specific session/conversation ID.
        #[arg(long)]
        session: Option<String>,
        /// Maximum results to return.
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Search all workspaces (disables the default workspace filter).
        #[arg(long)]
        all_workspaces: bool,
        /// Include tool calls/results.
        #[arg(long)]
        include_tools: bool,
        /// Include system context (AGENTS.md, etc.).
        #[arg(long)]
        include_system: bool,
        /// Disable result deduplication.
        #[arg(long)]
        no_dedup: bool,
    },

    /// Manage agent tools.
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },

    /// Task scheduling.
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommand,
    },

    /// Orient: operative-memory snapshot (who/where you are + what's relevant now).
    Ctx {
        /// Include the fleet view (gvnr resolve/list over the wire).
        #[arg(long)]
        gvnr: bool,
    },

    /// Run the ONE-tool MCP server (unified agent surface).
    Mcp,

    /// Generate shell completions.
    Completions {
        /// Shell to generate completions for.
        shell: clap_complete::Shell,
    },

    /// Initialize agntz for the current repo (mmry store, AGENTS.md).
    Init {
        /// Force re-initialization.
        #[arg(long)]
        force: bool,
    },

    /// Git-backed agent messageboard (coordination).
    Board {
        #[command(subcommand)]
        command: BoardCommand,
    },

    /// Git-backed Markdown knowledge wiki (durable synthesis).
    Wiki {
        #[command(subcommand)]
        command: WikiCommand,
    },

    /// Inspect and manage the agntz configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

/// Configuration subcommands.
#[derive(Debug, Clone, Copy, Subcommand)]
enum ConfigCommand {
    /// Output the effective configuration.
    Show,
    /// Print the resolved config file path.
    Path,
    /// Print all resolved paths (config, data, state).
    Paths,
    /// Regenerate the default configuration file.
    Reset,
}

#[derive(Debug, Clone)]
struct RuntimeContext {
    common: CommonOpts,
    paths: AppPaths,
    config: AppConfig,
}

impl RuntimeContext {
    fn new(common: CommonOpts) -> Result<Self> {
        let paths = AppPaths::discover(common.config.as_deref())?;
        let config = load_or_init_config(&paths.config_file, common.dry_run)?;
        let ctx = Self {
            common,
            paths,
            config,
        };
        ctx.init_logging()?;
        Ok(ctx)
    }

    fn init_logging(&self) -> Result<()> {
        if self.common.quiet {
            log::set_max_level(LevelFilter::Off);
            return Ok(());
        }

        let mut builder =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error"));

        builder.filter_level(self.effective_log_level());

        let force_color = matches!(self.common.color, ColorOption::Always)
            || env::var_os("FORCE_COLOR").is_some();
        let disable_color = self.common.no_color
            || matches!(self.common.color, ColorOption::Never)
            || env::var_os("NO_COLOR").is_some()
            || (!force_color && !io::stderr().is_terminal());

        if disable_color {
            builder.write_style(env_logger::fmt::WriteStyle::Never);
        } else if force_color {
            builder.write_style(env_logger::fmt::WriteStyle::Always);
        } else {
            builder.write_style(env_logger::fmt::WriteStyle::Auto);
        }

        if self.common.diagnostics {
            builder.format_timestamp_millis();
            builder.format_module_path(true);
            builder.format_target(true);
        }

        // A prior caller (e.g. tests) may already have initialized the logger.
        let _ = builder.try_init();
        Ok(())
    }

    const fn effective_log_level(&self) -> LevelFilter {
        if self.common.trace {
            LevelFilter::Trace
        } else if self.common.debug {
            LevelFilter::Debug
        } else {
            match self.common.verbose {
                0 => LevelFilter::Info,
                1 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    try_main().await
}

async fn try_main() -> Result<()> {
    let cli = Cli::parse();
    let ctx = RuntimeContext::new(cli.common.clone())?;

    readout::reset_emitted();
    let result: Result<()> = async { dispatch(cli.command, &ctx).await }.await;

    if let Err(e) = result {
        // In --json mode always emit the stable envelope on failure too, so a
        // model never has to parse non-JSON stderr. Skip it if the command
        // already emitted its own ok:false envelope (e.g. wiki validate), so
        // stdout stays a single document while the exit code is still non-zero.
        if ctx.common.json && !readout::emitted_error() {
            readout::emit(
                "error",
                false,
                Some(format!("{e:#}")),
                serde_json::Value::Null,
            );
        }
        return Err(e);
    }
    Ok(())
}

async fn dispatch(command: Commands, ctx: &RuntimeContext) -> Result<()> {
    match command {
        Commands::Memory { command } => memory::handle(command, ctx.common.json).await,
        Commands::Tasks { command } => issues::handle(command, ctx.common.json).await,
        Commands::Ready => handle_ready(ctx).await,
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
                ctx,
            )
            .await
        }
        Commands::Tools { command } => tools::handle(command).await,
        Commands::Schedule { command } => {
            schedule::handle(command, ctx.common.json, ctx.common.assume_yes).await
        }
        Commands::Ctx { gvnr } => ctx::handle(gvnr, ctx.common.json),
        Commands::Mcp => {
            mcp::run().await?;
            Ok(())
        }
        Commands::Completions { shell } => handle_completions(shell),
        Commands::Init { force } => handle_init(force, ctx).await,
        Commands::Board { command } => board::handle(command, ctx),
        Commands::Wiki { command } => wiki::handle(command, ctx),
        Commands::Config { command } => handle_config(ctx, command),
    }
}

async fn handle_ready(ctx: &RuntimeContext) -> Result<()> {
    if ctx.common.json {
        let (ok, out, err) = readout::run("trx", &["ready".to_string(), "--json".to_string()]);
        readout::emit(
            "ready",
            ok,
            (!ok).then(|| format!("trx failed: {err}")),
            readout::parse_or_text(out),
        );
        return Ok(());
    }

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
    ctx: &RuntimeContext,
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

    if ctx.common.json {
        let payload = serde_json::json!({ "hits": hits });
        readout::emit("search", true, None, payload);
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
            .and_then(|w| w.rsplit('/').next())
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
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "agntz", &mut io::stdout());
    Ok(())
}

fn handle_config(ctx: &RuntimeContext, command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Show => {
            if ctx.common.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&ctx.config)
                        .context("serializing config to JSON")?
                );
            } else {
                println!("{:#?}", ctx.config);
            }
            Ok(())
        }
        ConfigCommand::Path => {
            println!("{}", ctx.paths.config_file.display());
            Ok(())
        }
        ConfigCommand::Paths => {
            let data = ctx.paths.data_dir.display();
            let state = ctx.paths.state_dir.display();
            if ctx.common.json {
                let paths = serde_json::json!({
                    "config": ctx.paths.config_file,
                    "data": ctx.paths.data_dir,
                    "state": ctx.paths.state_dir,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&paths).context("serializing paths to JSON")?
                );
            } else {
                println!("config: {}", ctx.paths.config_file.display());
                println!("data:   {}", data);
                println!("state:  {}", state);
            }
            Ok(())
        }
        ConfigCommand::Reset => {
            if ctx.common.dry_run {
                log::info!(
                    "dry-run: would reset config at {}",
                    ctx.paths.config_file.display()
                );
                return Ok(());
            }
            write_default_config(&ctx.paths.config_file)
        }
    }
}

/// Get the current repo name from git remote or directory name.
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
            .map(str::to_string);
        if name.is_some() {
            return name;
        }
    }

    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
}

async fn handle_init(force: bool, ctx: &RuntimeContext) -> Result<()> {
    let repo_name = get_repo_name().context("could not determine repo name")?;
    println!("Initializing agntz for repo: {repo_name}");

    println!("\n[1/3] Initializing mmry store...");
    let mut mmry_args = vec!["init".to_string(), "--store".to_string(), repo_name.clone()];
    if force {
        mmry_args.push("--force".to_string());
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

    println!("[2/3] Initializing trx...");
    let trx_args = vec![
        "init".to_string(),
        "--prefix".to_string(),
        repo_name.clone(),
    ];
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

    println!("[3/3] Updating AGENTS.md...");
    let agents_md = PathBuf::from("AGENTS.md");
    let agntz_section = r#"
## agntz

Use agntz for memory:

```bash
agntz memory search "topic"    # Find relevant context
agntz memory add "insight" -c category
agntz memory list
```
"#
    .to_string();

    if agents_md.exists() {
        let content = fs::read_to_string(&agents_md)?;
        if content.contains("## agntz") {
            if force {
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
        fs::write(&agents_md, format!("# Agent Instructions\n{agntz_section}"))?;
        println!("  Created AGENTS.md with agntz section");
    }

    println!("\nDone! agntz initialized for '{repo_name}'");
    let _ = ctx.common.dry_run;
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
