//! Application configuration: XDG paths, the board/wiki repo registry, schema
//! generation, and environment/flag-over-config precedence.
//!
//! agntz keeps a single TOML config file at `$XDG_CONFIG_HOME/agntz/config.toml`
//! (platform equivalents on Windows). It registers named board and wiki Git
//! repositories so commands never need hard-coded per-user paths. Selection
//! precedence is `flag > env > config default > first registered`.
//!
//! Machine output and config are JSON or TOML only — never YAML.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Application name, used for the config path and the env override prefix.
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Repository URL, stamped into generated schemas.
pub const REPO_URL: &str = "https://github.com/byteowlz/agntz";

/// The complete agntz configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AppConfig {
    /// Named board repository configuration.
    pub board: BoardConfig,
    /// Named wiki repository configuration.
    pub wiki: WikiConfig,
}

impl AppConfig {
    /// Register or update a named board repository and return globals to it.
    pub fn upsert_board(&mut self, repo: RepoConfig) {
        upsert_repo(&mut self.board.repos, repo);
    }

    /// Register or update a named wiki repository.
    pub fn upsert_wiki(&mut self, repo: RepoConfig) {
        upsert_repo(&mut self.wiki.repos, repo);
    }

    /// Look up a registered board repo by name.
    #[must_use]
    pub fn board_repo(&self, name: &str) -> Option<&RepoConfig> {
        self.board.repos.iter().find(|r| r.name == name)
    }

    /// Look up a registered wiki repo by name.
    #[must_use]
    pub fn wiki_repo(&self, name: &str) -> Option<&RepoConfig> {
        self.wiki.repos.iter().find(|r| r.name == name)
    }
}

/// Named board repositories and the default selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BoardConfig {
    /// Name of the default board repo.
    pub default: String,
    /// Registered board repositories.
    pub repos: Vec<RepoConfig>,
}

/// Named wiki repositories and the default selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct WikiConfig {
    /// Name of the default wiki repo.
    pub default: String,
    /// Registered wiki repositories.
    pub repos: Vec<RepoConfig>,
}

/// A single registered board or wiki repository.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct RepoConfig {
    /// Unique name used to select this repo via `--name` or config.
    pub name: String,
    /// Absolute (or `~`/env-expanded) path to the local working tree.
    pub path: String,
    /// Optional remote URL (may be empty for local-only repositories).
    pub remote: Option<String>,
    /// Optional default role for board identity (board only).
    pub role: Option<String>,
}

/// Insert a repo by name, replacing any prior entry with the same name.
fn upsert_repo(list: &mut Vec<RepoConfig>, repo: RepoConfig) {
    if let Some(entry) = list.iter_mut().find(|r| r.name == repo.name) {
        *entry = repo;
    } else {
        list.push(repo);
    }
}

/// Resolved filesystem paths for agntz (config, data, state, cache).
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Path to the config file.
    pub config_file: PathBuf,
    /// Data directory (e.g. for derived local indexes/cursors).
    pub data_dir: PathBuf,
    /// State directory (e.g. for transient runtime state).
    pub state_dir: PathBuf,
}

impl AppPaths {
    /// Resolve XDG paths (honoring `--config` override as a file or directory).
    ///
    /// # Errors
    ///
    /// Returns an error if the config path has no parent directory.
    pub fn discover(config_override: Option<&Path>) -> Result<Self> {
        let config_file = match config_override {
            Some(path) => {
                let expanded = expand_path(path)?;
                if expanded.is_dir() {
                    expanded.join("config.toml")
                } else {
                    expanded
                }
            }
            None => default_config_dir()?.join("config.toml"),
        };

        if config_file.parent().is_none() {
            return Err(anyhow!(
                "invalid config file path: {}",
                config_file.display()
            ));
        }

        Ok(Self {
            config_file,
            data_dir: default_data_dir()?,
            state_dir: default_state_dir()?,
        })
    }
}

/// The env prefix used for overrides, e.g. `AGNTZ__BOARD__DEFAULT`.
///
/// # Panics
///
/// Will not panic; the derive uses a length upper bound on the crate name.
#[must_use]
pub fn env_prefix() -> String {
    APP_NAME
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Load the config, creating a default one on first run.
///
/// # Errors
///
/// Returns an error if the config cannot be read or deserialized.
pub fn load_or_init_config(path: &Path, dry_run: bool) -> Result<AppConfig> {
    if !path.exists() {
        if dry_run {
            log::info!("dry-run: would create default config at {}", path.display());
        } else {
            write_default_config(path)?;
        }
    }

    let prefix = env_prefix();
    let config = config::Config::builder()
        .set_default("board.default", "")?
        .set_default("wiki.default", "")?
        .set_default("board.repos", Vec::<String>::new())?
        .set_default("wiki.repos", Vec::<String>::new())?
        .add_source(
            config::File::from(path)
                .format(config::FileFormat::Toml)
                .required(false),
        )
        .add_source(config::Environment::with_prefix(&prefix).separator("__"))
        .build()?;

    config.try_deserialize().map_err(Into::into)
}

/// Write the default config file to `path`.
///
/// # Errors
///
/// Returns an error if the parent directory cannot be created or the file
/// cannot be written.
pub fn write_default_config(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating config directory {}", parent.display()))?;
    }

    let cfg = AppConfig::default();
    let toml = toml::to_string_pretty(&cfg).context("serializing default config to TOML")?;
    let mut body = String::new();
    body.push_str("# Configuration for agntz\n");
    body.push_str("# File: ");
    body.push_str(&path.display().to_string());
    body.push('\n');
    body.push_str(
        "# Named board/wiki Git repositories. Select with --name, AGNTZ_BOARD / AGNTZ_WIKI,\n",
    );
    body.push_str(
        "# or the default in this file. Precedence: flag > env > config default > first.\n\n",
    );
    body.push_str(&toml);
    fs::write(path, body).with_context(|| format!("writing config file to {}", path.display()))
}

/// Expand `~` and environment variables in a path.
///
/// # Errors
///
/// Returns an error if expansion fails.
pub fn expand_path(path: &Path) -> Result<PathBuf> {
    path.to_str()
        .map_or_else(|| Ok(path.to_path_buf()), expand_str_path)
}

/// Expand `~` and environment variables in a path string.
///
/// # Errors
///
/// Returns an error if expansion fails.
pub fn expand_str_path(text: &str) -> Result<PathBuf> {
    let expanded = shellexpand::full(text).context("expanding path")?;
    // Store/resolve an absolute, normalized path so a board/wiki registered from
    // one cwd is always found from another (relative paths in config are a
    // cwd-coupling footgun that can silently open the wrong repo).
    Ok(std::path::absolute(PathBuf::from(expanded.to_string()))?)
}

/// Resolve a base dir with the given precedence.
fn base_dir(xdg_var: &str, unix_rel: &str, win_var: &str) -> Result<PathBuf> {
    let xdg = env::var_os(xdg_var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute());
    let home = env::var_os("HOME").map(PathBuf::from);
    let win = env::var_os(win_var).map(PathBuf::from);

    if let Some(path) = xdg {
        return Ok(path);
    }
    if cfg!(windows) {
        win.ok_or_else(|| anyhow!("unable to determine base directory ({win_var})"))
    } else {
        home.map(|h| h.join(unix_rel))
            .ok_or_else(|| anyhow!("unable to determine base directory ({xdg_var})"))
    }
}

fn default_config_dir() -> Result<PathBuf> {
    Ok(base_dir("XDG_CONFIG_HOME", ".config", "APPDATA")?.join(APP_NAME))
}

fn default_data_dir() -> Result<PathBuf> {
    Ok(base_dir("XDG_DATA_HOME", ".local/share", "APPDATA")?.join(APP_NAME))
}

fn default_state_dir() -> Result<PathBuf> {
    Ok(base_dir("XDG_STATE_HOME", ".local/state", "LOCALAPPDATA")?.join(APP_NAME))
}

/// Resolve the named repo that a command should operate on, applying
/// `flag > env > config default > first` precedence.
///
/// # Errors
///
/// Returns an error if no repo is configured or the requested name is unknown.
pub fn select_repo<'a>(
    repos: &'a [RepoConfig],
    default_name: &str,
    flag_name: Option<&str>,
    env_name: Option<&str>,
) -> Result<&'a RepoConfig> {
    let chosen = flag_name
        .or(env_name)
        .filter(|s| !s.is_empty())
        .or_else(|| (!default_name.is_empty()).then_some(default_name))
        .or_else(|| repos.first().map(|r| r.name.as_str()));

    let chosen = chosen.ok_or_else(|| {
        anyhow!("no repository configured; run `agntz board init` / `agntz wiki init` or set a config default")
    })?;

    repos
        .iter()
        .find(|r| r.name == chosen)
        .ok_or_else(|| anyhow!("repository '{chosen}' is not configured"))
}
