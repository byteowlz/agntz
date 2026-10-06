//! Small on-disk cache for expensive fan-out commands (`find`, `ctx`).
//!
//! Entries live under `$XDG_STATE_HOME/agntz/cache/<namespace>/<hash>.json` and
//! carry a TTL plus an optional *version token* (a cheap-to-recompute staleness
//! signal such as a git HEAD). A hit requires: not expired AND the version token
//! unchanged. Version tokens make invalidation exact for git-backed sources
//! (wiki, board, the git working tree); TTL covers stores without a cheap
//! version vector (mmry, hstry).
//!
//! Keys are hashed with `DefaultHasher` — not cryptographically stable across
//! rustc versions, but that only ever costs a cache miss.

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
struct Entry {
    cache_version: u32,
    key: String,
    created_at: i64,
    ttl: i64,
    version: Option<String>,
    payload: Value,
}

/// Default TTL for cached fan-out results (seconds).
pub const DEFAULT_TTL: i64 = 300;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn cache_dir(ns: &str) -> PathBuf {
    agntz::config::expand_str_path("$XDG_STATE_HOME/agntz/cache")
        .map(|p| p.join(ns))
        .unwrap_or_else(|_| PathBuf::from("/tmp/agntz-cache").join(ns))
}

fn hash_key(key: &str) -> String {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// A cheap staleness token (git HEAD, file mtime, …) for one source.
pub type VersionToken = Option<String>;

/// Load a fresh entry, or `None` when missing/expired/stale/ignored.
pub fn get(ns: &str, key: &str, ttl: i64, version: &VersionToken, no_cache: bool) -> Option<Value> {
    let _ = ttl; // expiry uses the TTL stored with the entry
    if no_cache {
        return None;
    }
    let path = cache_dir(ns).join(format!("{}.json", hash_key(key)));
    let raw = std::fs::read_to_string(path).ok()?;
    let entry: Entry = serde_json::from_str(&raw).ok()?;
    if entry.cache_version != CACHE_VERSION || entry.key != key {
        return None;
    }
    if entry.version.as_deref() != version.as_deref() {
        return None;
    }
    if now() - entry.created_at > entry.ttl {
        return None;
    }
    Some(entry.payload)
}

/// Store an entry (creating the cache dir; best-effort — cache failures must
/// never fail the command).
pub fn put(ns: &str, key: &str, payload: &Value, ttl: i64, version: &VersionToken) {
    let dir = cache_dir(ns);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let entry = Entry {
        cache_version: CACHE_VERSION,
        key: key.to_string(),
        created_at: now(),
        ttl,
        version: version.clone(),
        payload: payload.clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&entry) {
        let _ = std::fs::write(dir.join(format!("{}.json", hash_key(key))), json);
        prune_ns(&dir, ttl);
    }
}

/// Drop entries in a namespace older than `max_age` seconds (best-effort), so
/// the cache cannot grow unbounded.
fn prune_ns(dir: &PathBuf, ttl: i64) {
    let max_age = (ttl * 8).max(3600);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        let age = now()
            - modified
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
        if age > max_age {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Remove every entry in a namespace (used by `--refresh`-style flows that want
/// a hard reset rather than a per-key bypass).
#[allow(dead_code)]
pub fn clear(ns: &str) -> Result<()> {
    let dir = cache_dir(ns);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).context("clearing cache directory")?;
    }
    Ok(())
}
