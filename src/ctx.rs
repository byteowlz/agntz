//! `agntz ctx` — the operative-memory orientation command (vision pillar 1).
//!
//! One snapshot answering "what is relevant to me here, right now". Reads the
//! AGENT_CTX v2 producer-map (schemas/agent-context-env) to know who/where the
//! agent is, then surfaces the operative memory it can see (mmry/trx/hstry)
//! and — optionally via `--gvnr` — the fleet view.
//!
//! Design rules honoured here (see DESIGN.md):
//! * AGENT_CTX bags are independent — reflect whatever is present, never
//!   require a specific setup, degrade in bare-pi / no-multiplexer / etc.
//! * gvnr is fleet-ONLY (resolve/list). agntz never schedules/queues the fleet.
//! * No bespoke correlation — the snapshot is filtered by the present lineage
//!   ids (session/workspace/machine), inherited from AGENT_CTX.
//! * Snapshot capture stays an *open problem* — surfaced, not half-built.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Command;

use crate::gvnr;

/// Effective AGENT_CTX read from the environment (producer-map, v2).
///
/// Every field is optional except the contract floor (VERSION/HARNESS/RUN_MODE)
/// meaning "AGENT_CTX is enabled". Missing = this layer is absent; we tolerate.
#[derive(Debug, Default, Serialize)]
pub struct AgentCtx {
    /// Contract version (AGENT_CTX_VERSION), when AGENT_CTX is enabled.
    pub version: Option<String>,
    /// Producer-map bags, keyed by layer. Present bags only.
    pub bags: BTreeMap<String, BTreeMap<String, Option<String>>>,
    /// The stable lineage ids available for filtering operative memory.
    pub lineage: Lineage,
}

#[derive(Debug, Default, Serialize)]
pub struct Lineage {
    pub platform_session_id: Option<String>,
    pub harness_session_id: Option<String>,
    pub agent_id: Option<String>,
    pub workspace_id: Option<String>,
    pub machine_id: Option<String>,
}

const BAG_MEMBERS: &[(&str, &[&str])] = &[
    (
        "platform",
        &[
            "PLATFORM_NAME",
            "PLATFORM_VERSION",
            "PLATFORM_SESSION_ID",
            "WORKSPACE_ID",
            "WORKSPACE_PATH",
            "USER_ID",
        ],
    ),
    (
        "multiplexer",
        &["MULTIPLEXER", "AGENT_ID", "AGENT_ADDRESS", "AGENT_LABEL"],
    ),
    (
        "harness",
        &[
            "HARNESS",
            "HARNESS_SESSION_ID",
            "SESSION_NAME",
            "READABLE_ID",
            "MODEL",
        ],
    ),
    ("host", &["MACHINE_ID", "NODE_HOSTNAME", "OS_ARCH"]),
    (
        "tracing",
        &["REQUEST_ID", "CORRELATION_ID", "SANDBOX_PROFILE"],
    ),
];

fn get(var: &str) -> Option<String> {
    std::env::var(var)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read the effective AGENT_CTX from the environment.
///
/// Tolerant: works with any combination of bags (bare pi, herdr+pi, full, …).
pub fn read_agent_ctx() -> AgentCtx {
    let mut ctx = AgentCtx {
        version: get("AGENT_CTX_VERSION"),
        ..AgentCtx::default()
    };

    for (bag, members) in BAG_MEMBERS {
        let mut map = BTreeMap::new();
        for m in *members {
            if let Some(v) = get(&format!("AGENT_CTX_{m}")) {
                map.insert((*m).to_string(), Some(v));
            }
        }
        if !map.is_empty() {
            ctx.bags.insert((*bag).to_string(), map);
        }
    }

    ctx.lineage = Lineage {
        platform_session_id: get("AGENT_CTX_PLATFORM_SESSION_ID"),
        harness_session_id: get("AGENT_CTX_HARNESS_SESSION_ID"),
        agent_id: get("AGENT_CTX_AGENT_ID"),
        workspace_id: get("AGENT_CTX_WORKSPACE_ID"),
        machine_id: get("AGENT_CTX_MACHINE_ID"),
    };

    ctx
}

impl AgentCtx {
    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.version.is_some()
    }

    fn bag(&self, name: &str) -> Option<&BTreeMap<String, Option<String>>> {
        self.bags.get(name)
    }

    fn val(&self, bag: &str, key: &str) -> Option<String> {
        self.bag(bag)
            .and_then(|m| m.get(key))
            .and_then(|v| v.clone())
    }

    /// A short human identity: "harness (agent label/session name)".
    pub fn display_identity(&self) -> String {
        let harness = self
            .val("harness", "HARNESS")
            .unwrap_or_else(|| "unknown-environment".to_string());
        let label = self
            .val("multiplexer", "AGENT_LABEL")
            .or_else(|| self.val("harness", "SESSION_NAME"));
        match label {
            Some(l) => format!("{harness} ({l})"),
            None => harness,
        }
    }
}

// ---------------------------------------------------------------------------
// Operative memory reads (mmry/trx/hstry) — the front-door surface.
// Each is best-effort: a missing tool makes that section unavailable, never
// an error that aborts the whole snapshot.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize)]
pub struct TaskSummary {
    pub available: bool,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub open: Vec<Task>,
    /// How many open tasks name this agent's session / workspace (lineage).
    pub involving_self: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<u8>,
    pub issue_type: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub sessions: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Surface open tasks (trx list --json), and count how many name this agent's
/// lineage (harness session id / workspace id / agent id) — inherited
/// correlation, no bespoke graph.
pub fn read_tasks(ctx: &AgentCtx) -> TaskSummary {
    let mut out = TaskSummary::default();
    let resp = match run_json("trx", &["list", "--json"]) {
        Some(v) => v,
        None => {
            out.available = false;
            out.error = Some("trx unavailable (not installed or no trx store)".into());
            return out;
        }
    };

    let tasks: Vec<Task> = match serde_json::from_value(resp) {
        Ok(t) => t,
        Err(e) => {
            out.available = false;
            out.error = Some(format!("trx list --json unparseable: {e}"));
            return out;
        }
    };

    let self_ids: Vec<String> = [
        ctx.lineage.harness_session_id.clone(),
        ctx.lineage.workspace_id.clone(),
        ctx.lineage.agent_id.clone(),
    ]
    .into_iter()
    .flatten()
    .collect();

    out.available = true;
    out.open = tasks.iter().filter(|t| t.status == "open").cloned().collect();
    out.involving_self = out
        .open
        .iter()
        .filter(|t| {
            t.sessions.iter().any(|s| self_ids.contains(s))
                || t.description.as_deref().is_some_and(|d| {
                    self_ids.iter().any(|id| d.contains(id))
                })
        })
        .count();
    out
}

#[derive(Debug, Default, Serialize)]
pub struct MemorySummary {
    pub available: bool,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub top: Vec<Memory>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Memory {
    pub id: Option<String>,
    pub content: String,
    pub category: Option<String>,
    pub importance: Option<u8>,
}

/// Top memories for this repo (mmry list --json --limit N).
pub fn read_memories(limit: usize) -> MemorySummary {
    let mut out = MemorySummary::default();
    let resp = match run_json(
        "mmry",
        &["ls", "--json", "--limit", &limit.to_string()],
    ) {
        Some(v) => v,
        None => {
            out.available = false;
            out.error = Some("mmry unavailable (not installed)".into());
            return out;
        }
    };

    // mmry returns either an array of items or an object.
    let items: Vec<Memory> = if let Ok(arr) = serde_json::from_value::<Vec<Memory>>(resp.clone())
    {
        arr
    } else if let Some(obj) = resp.as_object() {
        // { items: [...] } or { memories: [...] }
        for key in ["items", "memories", "results", "data"] {
            if let Some(v) = obj.get(key) {
                if let Ok(arr) = serde_json::from_value::<Vec<Memory>>(v.clone()) {
                    return finish_memories(arr, out);
                }
            }
        }
        match obj.get("content").and_then(|c| c.as_str()) {
            Some(content) => vec![Memory {
                id: None,
                content: content.to_string(),
                category: None,
                importance: None,
            }],
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };

    finish_memories(items, out)
}

fn finish_memories(items: Vec<Memory>, mut out: MemorySummary) -> MemorySummary {
    out.available = true;
    out.top = items;
    out
}

#[derive(Debug, Default, Serialize)]
pub struct HistorySummary {
    pub available: bool,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent: Vec<HistoryHit>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryHit {
    pub session_id: String,
    pub role: String,
    pub title: Option<String>,
    pub snippet: String,
    pub created_at: Option<String>,
    pub workspace: Option<String>,
}

#[derive(Deserialize)]
struct HstryEnvelope<T> {
    ok: bool,
    result: Option<T>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct RawHit {
    conversation_id: String,
    external_id: Option<String>,
    role: String,
    title: Option<String>,
    snippet: String,
    created_at: Option<String>,
    workspace: Option<String>,
}

/// Most recent session history touching this workspace (hstry search, service
/// or local). Best-effort: an unreachable service → unavailable, not fatal.
pub fn read_history(limit: usize, workspace: &str) -> HistorySummary {
    let mut out = HistorySummary::default();

    let mut args = vec!["search".to_string(), "".to_string(), "--json".to_string()];
    args.push("--limit".to_string());
    args.push(limit.to_string());
    args.push("--workspace".to_string());
    args.push(workspace.to_string());

    let output = Command::new("hstry")
        .args(&args)
        .output()
        .map_err(|e| e.to_string())
        .ok();

    let raw = match output {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        Some(_) => {
            out.available = false;
            out.error = Some("hstry search failed".into());
            return out;
        }
        None => {
            out.available = false;
            out.error = Some("hstry unavailable (not installed or service down)".into());
            return out;
        }
    };

    let envelope: HstryEnvelope<Vec<RawHit>> = match serde_json::from_str(&raw) {
        Ok(e) => e,
        Err(_) => {
            // Some hstry modes return the array directly.
            out.available = false;
            out.error = Some("hstry search returned an unsupported shape".into());
            return out;
        }
    };

    if !envelope.ok {
        out.available = false;
        out.error = envelope.error.or(Some("hstry search failed".into()));
        return out;
    }

    out.available = true;
    out.recent = envelope
        .result
        .unwrap_or_default()
        .into_iter()
        .map(|h| HistoryHit {
            session_id: h.external_id.unwrap_or(h.conversation_id),
            role: h.role,
            title: h.title,
            snippet: h.snippet,
            created_at: h.created_at,
            workspace: h.workspace,
        })
        .collect();
    out
}

// ---------------------------------------------------------------------------
// Snapshot assembly
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CtxSnapshot {
    pub schema: &'static str,
    pub version: u32,
    pub captured_at: String,
    pub agent: AgentCtx,
    pub workspace: Workspace,
    pub operative_memory: OperativeMemory,
    pub fleet: gvnr::FleetView,
    pub open_problems: OpenProblems,
}

#[derive(Debug, Serialize)]
pub struct Workspace {
    pub id: Option<String>,
    pub path: Option<String>,
    pub cwd: String,
    pub repo: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OperativeMemory {
    pub tasks: TaskSummary,
    pub memories: MemorySummary,
    pub history: HistorySummary,
}

#[derive(Debug, Serialize)]
pub struct OpenProblems {
    pub snapshot_capture: SnapshotProblem,
}

#[derive(Debug, Serialize)]
pub struct SnapshotProblem {
    pub status: &'static str,
    pub note: &'static str,
    pub next_diagnostic: &'static str,
}

fn workspace(ctx: &AgentCtx) -> Workspace {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    Workspace {
        id: ctx.lineage.workspace_id.clone(),
        path: ctx.val("platform", "WORKSPACE_PATH").or_else(|| {
            std::env::var("AGENT_CTX_WORKSPACE_PATH").ok()
        }),
        cwd,
        repo: repo_name(),
    }
}

fn repo_name() -> Option<String> {
    let out = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !out.status.success() {
        return std::env::current_dir()
            .ok()
            .and_then(|d| d.file_name().map(|s| s.to_string_lossy().to_string()));
    }
    let url = String::from_utf8_lossy(&out.stdout);
    let name = url.trim().trim_end_matches(".git").rsplit('/').next()?;
    Some(name.to_string())
}

/// Build the full orientation snapshot.
pub fn snapshot(include_fleet: bool) -> CtxSnapshot {
    let ctx = read_agent_ctx();
    let cwd = workspace(&ctx);

    let memories = read_memories(5);
    let tasks = read_tasks(&ctx);
    let history = read_history(5, cwd.cwd.as_str());
    let fleet = gvnr::fleet_view(include_fleet);

    CtxSnapshot {
        schema: "agntz.ctx",
        version: 1,
        captured_at: Utc::now().to_rfc3339(),
        agent: ctx,
        workspace: cwd,
        operative_memory: OperativeMemory {
            tasks,
            memories,
            history,
        },
        fleet,
        open_problems: OpenProblems {
            snapshot_capture: SnapshotProblem {
                status: "open",
                note: "snapshot capture is an open design problem; not shipped",
                next_diagnostic: "instrument AGENT_CTX lineage (session/workspace/machine) on the three stores, then diff two ctx captures to see which fields survive an agent lifetime",
            },
        },
    }
}

/// Public entry point for `agntz ctx`. Prints text (or JSON with --json).
pub fn handle(include_fleet: bool, json: bool) -> Result<()> {
    let snap = snapshot(include_fleet);
    if json {
        println!("{}", serde_json::to_string_pretty(&snap)?);
    } else {
        print_text(&snap);
    }
    Ok(())
}

fn print_text(s: &CtxSnapshot) {
    println!("agntz ctx — operative memory orientation");
    println!("{}", "─".repeat(60));

    // Who am I
    println!("\nagent: {}", s.agent.display_identity());
    if let Some(m) = s.agent.val("harness", "MODEL") {
        println!("  model:      {m}");
    }
    if let Some(r) = s.agent.val("harness", "HARNESS") {
        println!("  harness:    {r}");
    }
    if let Some(m) = s.agent.val("multiplexer", "MULTIPLEXER") {
        println!("  multiplexer:{m}");
    }
    if let Some(a) = s.agent.val("multiplexer", "AGENT_ID") {
        println!("  agent_id:   {a}");
    }
    if let Some(h) = s.agent.val("host", "MACHINE_ID") {
        println!("  machine:    {h}{}",
            s.agent.val("host", "OS_ARCH").map(|a| format!(" ({a})")).unwrap_or_default());
    }

    // Workspace
    println!("\nworkspace:{}", s.workspace.repo.as_deref().unwrap_or("-"));
    if let Some(id) = &s.workspace.id {
        println!("  id:    {id}");
    }
    if let Some(p) = &s.workspace.path {
        println!("  path:  {p}");
    } else {
        println!("  cwd:   {}", s.workspace.cwd);
    }

    // Operative memory
    println!("\noperative memory:");
    if s.operative_memory.tasks.available {
        println!(
            "  tasks:   {} open ({} naming this session/workspace)",
            s.operative_memory.tasks.open.len(),
            s.operative_memory.tasks.involving_self
        );
        for t in &s.operative_memory.tasks.open {
            let pri = t.priority.map(|p| format!(" [P{p}]")).unwrap_or_default();
            println!("    {} {}{pri} {}", t.id, t.status, t.title);
        }
    } else {
        println!(
            "  tasks:   unavailable ({})",
            s.operative_memory.tasks.error.as_deref().unwrap_or("?")
        );
    }
    if s.operative_memory.memories.available {
        println!("  memories: {} top", s.operative_memory.memories.top.len());
        for m in &s.operative_memory.memories.top {
            let cat = m
                .category
                .as_deref()
                .map(|c| format!("[{c}] "))
                .unwrap_or_default();
            let content = m.content.lines().next().unwrap_or("").trim();
            println!("    {}{}", cat, clip(content, 90));
        }
    } else {
        println!("  memories: unavailable ({})", s.operative_memory.memories.error.as_deref().unwrap_or("?"));
    }
    if s.operative_memory.history.available {
        println!("  history:  {} recent", s.operative_memory.history.recent.len());
        for h in &s.operative_memory.history.recent {
            println!(
                "    {:<36} {} - {}",
                h.session_id,
                h.role,
                clip(h.title.as_deref().unwrap_or(&h.snippet), 60)
            );
        }
    } else {
        println!("  history:  unavailable ({})", s.operative_memory.history.error.as_deref().unwrap_or("?"));
    }

    // Fleet
    if s.fleet.available {
        println!("\nfleet (gvnr):");
        if let Some(runners) = &s.fleet.runners {
            for r in runners {
                println!("  {}  {}  last_beat={}", r.runner_id, r.mesh_ip, r.last_beat);
            }
        }
    } else {
        println!("\nfleet (gvnr): unavailable — {}", s.fleet.reason.as_deref().unwrap_or("not configured"));
        println!("  (start/point agntz at gvnr; fleet view is optional)");
    }

    println!("\nopen problems:");
    println!("  snapshot_capture: {} — {}", s.open_problems.snapshot_capture.status, s.open_problems.snapshot_capture.note);
}

fn clip(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n.saturating_sub(3)).collect();
        t.push_str("...");
        t
    }
}

/// Run a tool expecting JSON on stdout; returns its stdout parsed as JSON, or
/// None if the tool is missing or fails. Never panics.
fn run_json(tool: &str, args: &[&str]) -> Option<serde_json::Value> {
    let out = Command::new(tool).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(stdout.trim()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    // Tests mutate the process-global env, so serialize them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn clear_all() {
        for (k, _) in std::env::vars() {
            if k.starts_with("AGENT_CTX_") {
                std::env::remove_var(k);
            }
        }
    }

    fn with_agent_ctx(vars: &[(&str, &str)]) {
        clear_all();
        for (k, v) in vars {
            std::env::set_var(format!("AGENT_CTX_{}", k), *v);
        }
    }

    #[test]
    fn tolerates_absent_bags() {
        let _g = lock_env();
        clear_all();
        let ctx = read_agent_ctx();
        assert!(!ctx.is_enabled());
        assert!(ctx.bags.is_empty());
    }

    #[test]
    fn reads_floor_only() {
        let _g = lock_env();
        with_agent_ctx(&[("VERSION", "2"), ("HARNESS", "pi"), ("RUN_MODE", "local")]);
        let ctx = read_agent_ctx();
        assert!(ctx.is_enabled());
        assert_eq!(ctx.bags["harness"]["HARNESS"].as_deref(), Some("pi"));
        assert!(!ctx.bags.contains_key("multiplexer"));
        assert!(!ctx.bags.contains_key("host"));
    }

    #[test]
    fn reads_full_producer_map() {
        let _g = lock_env();
        with_agent_ctx(&[
            ("VERSION", "2"),
            ("HARNESS", "pi"),
            ("RUN_MODE", "runner"),
            ("PLATFORM_NAME", "oqto"),
            ("PLATFORM_SESSION_ID", "sess_1"),
            ("WORKSPACE_ID", "ws_1"),
            ("WORKSPACE_PATH", "/ws"),
            ("USER_ID", "u_1"),
            ("MULTIPLEXER", "herdr"),
            ("AGENT_ID", "agent_1"),
            ("AGENT_ADDRESS", "tab_1"),
            ("HARNESS_SESSION_ID", "pi_1"),
            ("MACHINE_ID", "node-1"),
            ("OS_ARCH", "linux/amd64"),
        ]);
        let ctx = read_agent_ctx();
        assert!(ctx.bags.contains_key("platform"));
        assert!(ctx.bags.contains_key("multiplexer"));
        assert!(ctx.bags.contains_key("host"));
        assert_eq!(ctx.lineage.workspace_id.as_deref(), Some("ws_1"));
        assert_eq!(ctx.lineage.agent_id.as_deref(), Some("agent_1"));
        assert_eq!(ctx.val("host", "MACHINE_ID").as_deref(), Some("node-1"));
        assert!(ctx.display_identity().starts_with("pi"));
    }

    #[test]
    fn reads_floor_only_remaining() {
        let _g = lock_env();
        with_agent_ctx(&[("VERSION", "2"), ("HARNESS", "pi"), ("RUN_MODE", "")]);
        let ctx = read_agent_ctx();
        assert_eq!(ctx.bags.get("harness").and_then(|m| m.get("RUN_MODE")), None);
    }
}
