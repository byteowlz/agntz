//! Fleet view for `agntz ctx --gvnr` — resolves + lists ONLY.
//!
//! gvnr is the fleet control plane: it resolves and lists, never schedules,
//! queues, or owns agents. agntz stays a normal-agent operative-memory tool;
//! the fleet view is an optional window, strictly read-only via the wire
//! protocol in `schemas/gvnr-dpty-wire` (`/resolve/{id}` + `/list_runners`).
//!
//! Degradation contract: if gvnr is not configured / not reachable / the token
//! is missing, `fleet_view` returns `available: false` with a reason. It never
//! blocks the orientation snapshot, and it never invents fleet truth.

use serde::Serialize;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// A fleet runner as declared by a registered heartbeat (registry fact).
#[derive(Debug, Clone, Serialize)]
pub struct Runner {
    pub runner_id: String,
    pub mesh_ip: String,
    pub last_beat: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_vram_gb: Option<f64>,
}

/// Result of resolving this agent's own id on the fleet, if any.
#[derive(Debug, Clone, Serialize)]
pub struct SelfResolved {
    pub id: String,
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

/// Read-only fleet view.
#[derive(Debug, Default, Serialize)]
pub struct FleetView {
    pub available: bool,
    /// Human reason when the fleet is not reachable / not configured.
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runners: Option<Vec<Runner>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_resolved: Option<SelfResolved>,
}

/// Resolve fleet config from the environment. Returns (endpoint, token).
///
/// Precedence: `AGNTZ_GVNR_URL` (+ `AGNTZ_GVNR_TOKEN`) → gvnr CLI when
/// `AGNTZ_GVNR_BIN` is set (treats the binary as a thin over-the-wire shell
/// with `--json`). If none, the fleet view is simply unavailable.
fn config() -> Option<(String, String)> {
    let url = std::env::var("AGNTZ_GVNR_URL").ok().filter(|s| !s.is_empty())?;
    let token = std::env::var("AGNTZ_GVNR_TOKEN")
        .ok()
        .filter(|s| !s.is_empty())?;
    Some((url, token))
}

/// Request the fleet view over the wire. `include` gates whether we even try
/// (default `ctx` keeps the fleet optional behind `--gvnr`).
pub fn fleet_view(include: bool) -> FleetView {
    if !include {
        return FleetView {
            available: false,
            reason: Some("omit (pass --gvnr to include the fleet view)".into()),
            ..Default::default()
        };
    }

    let (url, token) = match config() {
        Some(c) => c,
        None => {
            return FleetView {
                available: false,
                reason: Some(
                    "gvnr not configured (set AGNTZ_GVNR_URL + AGNTZ_GVNR_TOKEN)".into(),
                ),
                ..Default::default()
            }
        }
    };

    if let Some(cli) = std::env::var("AGNTZ_GVNR_BIN").ok().filter(|s| !s.is_empty()) {
        return via_cli(&cli);
    }

    via_wire(&url, &token)
}

fn via_cli(cli: &str) -> FleetView {
    let mut view = FleetView::default();
    let list = Command::new(cli)
        .args(["list", "--json"])
        .output()
        .ok();
    match list {
        Some(o) if o.status.success() => match serde_json::from_slice::<serde_json::Value>(&o.stdout)
        {
            Ok(v) => {
                view.available = true;
                view.runners = parse_runners(&v).or(Some(Vec::new()));
            }
            Err(_) => {
                view.available = false;
                view.reason = Some("gvnr CLI output unparseable".into());
            }
        },
        _ => {
            view.available = false;
            view.reason = Some("gvnr CLI unavailable/failed".into());
        }
    }
    view
}

fn via_wire(url: &str, token: &str) -> FleetView {
    let mut view = FleetView::default();

    // GET /list_runners (wire ListRunnersResp)
    match http_get(url, "/list_runners", token) {
        Ok(body) => match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(v) => {
                view.runners = parse_runners(&v).or(Some(Vec::new()));
                view.available = true;
            }
            Err(e) => {
                view.available = false;
                view.reason = Some(format!("list_runners unparseable: {e}"));
                return view;
            }
        },
        Err(e) => {
            view.available = false;
            view.reason = Some(format!("gvnr unreachable: {e}"));
            return view;
        }
    }

    // GET /resolve/{self} (wire ResolveResp) — best-effort; never fails the view.
    if let Ok(agent_id) = std::env::var("AGENT_CTX_AGENT_ID") {
        if !agent_id.is_empty() {
            let path = format!("/resolve/{}", urlencode(&agent_id));
            if let Ok(body) = http_get(url, &path, token) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    view.self_resolved = parse_resolve(&v, &agent_id);
                }
            }
        }
    }

    view
}

fn parse_runners(v: &serde_json::Value) -> Option<Vec<Runner>> {
    let runners = if let Some(r) = v.get("runners") {
        r
    } else if let Some(r) = v.get("payload").and_then(|p| p.get("runners")) {
        r
    } else {
        v
    };
    let arr = runners.as_array()?;
    Some(arr.iter().map(|r| Runner {
        runner_id: r.get("runner_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        mesh_ip: r.get("mesh_ip").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        last_beat: r.get("last_beat").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        rank: r.get("rank").and_then(|x| x.as_f64()),
        free_vram_gb: r.get("free_vram_gb").and_then(|x| x.as_f64()),
    }).collect())
}

fn parse_resolve(v: &serde_json::Value, id: &str) -> Option<SelfResolved> {
    let payload = v.get("payload").unwrap_or(v);
    let found = payload.get("found").and_then(|x| x.as_bool()).unwrap_or(false);
    Some(SelfResolved {
        id: payload.get("id").and_then(|x| x.as_str()).unwrap_or(id).to_string(),
        found,
        address: payload.get("address").and_then(|x| x.as_str()).map(|s| s.to_string()),
    })
}

// ---------------------------------------------------------------------------
// Minimal HTTP GET over std TcpStream (no extra deps). Good enough for the
// gvnr wire over the overlay mesh (http). HTTPS is out of scope for the
// default mesh; gvnr clients terminate auth at the registry layer.
// ---------------------------------------------------------------------------

fn http_get(base: &str, path: &str, token: &str) -> std::io::Result<String> {
    let (host, port) = parse_url(base)?;
    let addr = format!("{host}:{port}");
    let mut stream = TcpStream::connect(&addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(4)))?;
    stream.set_write_timeout(Some(Duration::from_secs(4)))?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes())?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf).to_string();

    // Split headers from body.
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("");
    Ok(body.to_string())
}

/// Parse "host:port" or "scheme://host:port/path" → (host, port). Defaults 80.
fn parse_url(url: &str) -> std::io::Result<(String, u16)> {
    let trimmed = url.trim();
    let rest = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);
    let hostport = rest.split('/').next().unwrap_or(rest);
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) => {
            (h.to_string(), p.parse().unwrap_or(80))
        }
        _ => (hostport.to_string(), 80),
    };
    if host.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty host"));
    }
    Ok((host, port))
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => {
                let mut b = [0u8; 4];
                let s = c.encode_utf8(&mut b).as_bytes();
                s.iter()
                    .map(|&x| format!("%{:02X}", x))
                    .collect::<String>()
            }
        })
        .collect()
}

use std::process::Command;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_defaults() {
        assert_eq!(parse_url("http://100.64.0.5:8080").unwrap(), ("100.64.0.5".into(), 8080));
        assert_eq!(parse_url("100.64.0.5").unwrap(), ("100.64.0.5".into(), 80));
        assert_eq!(
            parse_url("https://gvnr.byteowlz.dev").unwrap(),
            ("gvnr.byteowlz.dev".into(), 80)
        );
    }

    #[test]
    fn urlencode_encodes_special_chars() {
        assert_eq!(urlencode("agent_9e2f:tab"), "agent_9e2f%3Atab");
        assert_eq!(urlencode("w2:p3W"), "w2%3Ap3W");
        assert_eq!(urlencode("plain"), "plain");
    }

    #[test]
    fn not_configured_is_unavailable() {
        std::env::remove_var("AGNTZ_GVNR_URL");
        std::env::remove_var("AGNTZ_GVNR_TOKEN");
        std::env::remove_var("AGNTZ_GVNR_BIN");
        // Include=true but no config → unavailable without panicking.
        let v = fleet_view(true);
        assert!(!v.available);
        assert!(v.reason.is_some());
    }

    #[test]
    fn parse_list_runners_handles_envelope_and_bare() {
        let bare = serde_json::json!({"runners": [{"runner_id": "r1", "mesh_ip": "100.64.0.1", "last_beat": "2026-09-16T00:00:00Z"}]});
        let runners = parse_runners(&bare).unwrap();
        assert_eq!(runners.len(), 1);
        assert_eq!(runners[0].runner_id, "r1");

        let env = serde_json::json!({"proto": "gvnr-dpty", "kind": "list_runners", "payload": {"runners": []}});
        assert_eq!(parse_runners(&env).unwrap().len(), 0);
    }
}
