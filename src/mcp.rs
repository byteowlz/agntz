//! ONE-tool MCP server (vision pillar 2 — unified agent surface).
//!
//! The agent never loads five tool defs / five help texts: it loads a single
//! MCP tool, `agntz_orientation`, exposing the operative-memory orientation
//! surface (the same code path as `agntz ctx`). Wrappers (mmry/trx/hstry/
//! skdlr) stay as backends; gvnr stays fleet-only.
//!
//! A small, dependency-light JSON-RPC 2.0 server over stdio implementing just
//! the handful of MCP methods a one-tool server needs.

use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::ctx;

pub const TOOL_NAME: &str = "agntz_orientation";

const TOOL_DESCRIPTION: &str = "Operative-memory orientation for the running agent: \
who/where it is (AGENT_CTX producer-map: platform/multiplexer/harness/host bags), \
its workspace, its operative memory front door (open tasks, top memories, recent \
session history), and optionally the fleet view it can see (gvnr resolve/list). \
Best-effort: missing bags or unreachable backends are tolerated and reported, \
never fatal. Returns structured JSON.";

const SERVER_NAME: &str = "agntz-mcp";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the MCP stdio server until stdin closes.
pub async fn run() -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => {
                eprintln!("[agntz-mcp] dropped non-JSON-RPC line");
                continue;
            }
        };
        if let Some(resp) = dispatch(&msg) {
            if let Ok(out) = serde_json::to_string(&resp) {
                println!("{out}");
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
        }
    }
    Ok(())
}

fn id_of(msg: &Value) -> Option<Value> {
    // Literal JSON-RPC ids include 0; only an absent id marks a notification.
    msg.get("id").cloned()
}

fn dispatch(msg: &Value) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = id_of(msg);
    let params = msg.get("params").cloned().unwrap_or(Value::Null);

    match method {
        "initialize" => Some(make_response(id, init_result())),
        "notifications/initialized" | "notifications/cancelled" => None,
        "ping" => Some(make_response(id, json!({}))),
        "tools/list" => Some(make_response(id, tools_list())),
        "tools/call" => Some(handle_tool_call(id, params)),
        // Legacy/newer handshake shape: some clients send a top-level object.
        m if id.is_some() => Some(make_error(
            id,
            -32601,
            format!("Method not found: {m}"),
        )),
        _ => None,
    }
}

fn init_result() -> Value {
    json!({
        "protocolVersion": "2025-06-18",
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
        "instructions": TOOL_DESCRIPTION
    })
}

fn tools_list() -> Value {
    json!({
        "tools": [{
            "name": TOOL_NAME,
            "description": TOOL_DESCRIPTION,
            "inputSchema": {
                "type": "object",
                "properties": {
                    "fleet": {
                        "type": "boolean",
                        "description": "include the fleet view (gvnr resolve/list). Default false."
                    }
                }
            }
        }]
    })
}

fn handle_tool_call(id: Option<Value>, params: Value) -> Value {
    // params: { name, arguments? }
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    if name != TOOL_NAME {
        return make_error(id, -32602, format!("Unknown tool: {name}"));
    }

    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    let fleet = args
        .get("fleet")
        .and_then(|f| f.as_bool())
        .unwrap_or(false);

    // Run the orientation snapshot synchronously (lightweight reads).
    let snapshot = ctx::snapshot(fleet);
    let text = serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string());

    make_response(id, json!({
        "content": [ { "type": "text", "text": text } ],
        "isError": false
    }))
}

fn make_response(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn make_error(id: Option<Value>, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}
