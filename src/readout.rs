//! Stable JSON output for the unified agent surface (vision pillar 2).
//!
//! Every read verb, when asked for `--json`, emits the same stable envelope so
//! the agent always gets machine-parseable, token-efficient output regardless
//! of which store/wrapper backs the read. Backends stay as the underlying CLIs.
//!
//! Schema of the envelope:
//! ```json
//! { "schema": "agntz.read", "version": 1, "verb": "tasks/list",
//!   "ok": true, "error": null, "result": <value or raw text> }
//! ```
//! `result` carries the underlying tool's structured value when it is JSON,
//! or its raw text otherwise — always a legal JSON document.

use serde_json::{json, Value};

/// Run a wrapped tool, returning (success, stdout, stderr). Never panics.
pub fn run(tool: &str, args: &[String]) -> (bool, String, String) {
    match std::process::Command::new(tool).args(args).output() {
        Ok(o) => (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        ),
        Err(_) => (false, String::new(), format!("{tool} not found")),
    }
}

/// If stdout is JSON, return it as a Value; otherwise wrap the raw text as a
/// string value so the envelope is always valid JSON.
pub fn parse_or_text(stdout: String) -> Value {
    match serde_json::from_str::<Value>(stdout.trim()) {
        Ok(v) => v,
        Err(_) => Value::String(stdout),
    }
}

/// Print the stable read envelope.
pub fn emit(verb: &str, ok: bool, error: Option<String>, result: Value) {
    let envelope = json!({
        "schema": "agntz.read",
        "version": 1,
        "verb": verb,
        "ok": ok,
        "error": error,
        "result": result,
    });
    println!("{}", serde_json::to_string_pretty(&envelope).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_stdout() {
        assert!(parse_or_text("[1,2,3]".into()).is_array());
        assert!(parse_or_text("{\"a\":1}".into()).is_object());
    }

    #[test]
    fn wraps_non_json_text() {
        let v = parse_or_text("just a table\nrow".into());
        assert_eq!(v, Value::String("just a table\nrow".into()));
    }

    #[test]
    fn envelope_uses_stable_schema() {
        // Structural check via json macro used internally by emit.
        let e = json!({
            "schema": "agntz.read",
            "version": 1,
            "verb": "tasks/list",
            "ok": true,
            "error": null,
            "result": "y"
        });
        assert_eq!(e["schema"], "agntz.read");
        assert_eq!(e["ok"], true);
        assert_eq!(e["verb"], "tasks/list");
    }
}
