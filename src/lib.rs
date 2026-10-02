//! agntz library crate.
//!
//! Holds the shared configuration types and the JSON-schema / example-config
//! generation used to keep `examples/config.schema.json` and
//! `examples/config.toml` in sync with the real config structs.

pub mod config;

pub use config::{AppPaths, RepoConfig};

use std::path::Path;

use anyhow::{Context, Result};
use schemars::Schema;
use schemars::generate::SchemaSettings;

/// Generated schema filename.
pub const SCHEMA_FILENAME: &str = "config.schema.json";

/// Generated config filename.
pub const CONFIG_FILENAME: &str = "config.toml";

/// Clip `text` to at most `max` characters, snapping the cut back to the
/// previous word boundary so no word is split in the middle. Appends `...` when
/// clipped. Used for bounded excerpts/titles.
pub fn clip_to_words(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut clipped: String = text.chars().take(max).collect();
    if let Some(pos) = clipped.rfind(char::is_whitespace) {
        clipped.truncate(pos);
    }
    format!("{}...", clipped.trim_end())
}

/// Decode common `\n`/`\t`/`\r` escapes in an inline `--body` string so an
/// agent composing a multi-line body in a single argument gets real newlines
/// (not a literal backslash-n).
pub fn unescape_body(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
}

/// Snap a byte range `[start, end)` inside `text` outward to word boundaries so
/// an excerpt never begins or ends mid-word. Safe on any (possibly non-boundary)
/// `start`/`end`.
pub fn snap_to_word_bounds(text: &str, start: usize, end: usize) -> (usize, usize) {
    let start = if start > 0 {
        text.get(..start)
            .and_then(|p| p.rfind(char::is_whitespace))
            .map(|p| p + 1)
            .unwrap_or(start)
    } else {
        0
    };
    let end = if end < text.len() {
        text.get(end..)
            .and_then(|t| t.find(char::is_whitespace))
            .map(|p| end + p)
            .unwrap_or(text.len())
    } else {
        text.len()
    };
    (start, end.max(start))
}

/// Generate the JSON schema for `config::AppConfig`.
///
/// # Errors
///
/// Returns an error if schema serialization fails.
pub fn generate_schema() -> Result<String> {
    let settings = SchemaSettings::draft07();
    let generator = settings.into_generator();
    let mut schema: Schema = generator.into_root_schema_for::<config::AppConfig>();

    schema.insert(
        "$id".to_string(),
        serde_json::json!(format!("{}/schemas/config.schema.json", config::REPO_URL)),
    );
    schema.insert(
        "title".to_string(),
        serde_json::json!(format!("{} configuration", config::APP_NAME)),
    );
    schema.insert(
        "description".to_string(),
        serde_json::json!(format!("Configuration schema for {}", config::APP_NAME)),
    );

    if let Some(props) = schema.get_mut("properties")
        && let Some(props_obj) = props.as_object_mut()
    {
        props_obj.insert(
            "$schema".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "JSON Schema reference for editor support"
            }),
        );
    }

    serde_json::to_string_pretty(&schema).context("serializing JSON schema")
}

/// Generate the example TOML config from the default `AppConfig`.
///
/// # Errors
///
/// Returns an error if TOML serialization fails.
pub fn generate_example_config() -> Result<String> {
    let schema_url = format!(
        "https://raw.githubusercontent.com/byteowlz/schemas/refs/heads/main/{}/config.schema.json",
        config::APP_NAME
    );

    let cfg = config::AppConfig::default();
    let toml_body = toml::to_string_pretty(&cfg).context("serializing default config to TOML")?;

    Ok(format!(
        "#:schema {schema_url}\n\n# Configuration for {name}.\n# Copy this file to $XDG_CONFIG_HOME/{name}/config.toml and adjust as needed.\n\n{toml_body}",
        name = config::APP_NAME
    ))
}

/// Write the generated schema and example config into `output_dir`.
///
/// # Errors
///
/// Returns an error if the directory cannot be created or files cannot be
/// written.
pub fn write_generated_files(output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("creating output directory: {}", output_dir.display()))?;

    let schema = generate_schema()?;
    let schema_path = output_dir.join(SCHEMA_FILENAME);
    std::fs::write(&schema_path, &schema)
        .with_context(|| format!("writing schema to {}", schema_path.display()))?;

    let config = generate_example_config()?;
    let config_path = output_dir.join(CONFIG_FILENAME);
    std::fs::write(&config_path, &config)
        .with_context(|| format!("writing config to {}", config_path.display()))?;

    Ok(())
}

/// Compare the generated files against the committed examples.
///
/// # Errors
///
/// Returns an error if any example is missing or out of date.
pub fn validate_against_examples(examples_dir: &Path) -> Result<()> {
    let schema = generate_schema()?;
    let config = generate_example_config()?;

    let schema_path = examples_dir.join(SCHEMA_FILENAME);
    let config_path = examples_dir.join(CONFIG_FILENAME);

    let mut errors = Vec::new();

    if schema_path.exists() {
        let existing = std::fs::read_to_string(&schema_path)
            .with_context(|| format!("reading {}", schema_path.display()))?;
        if existing != schema {
            errors.push(format!(
                "{} is out of date. Run `just generate-config` to update.",
                schema_path.display()
            ));
        }
    } else {
        errors.push(format!(
            "{} does not exist. Run `just generate-config` to create.",
            schema_path.display()
        ));
    }

    if config_path.exists() {
        let existing = std::fs::read_to_string(&config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;
        if existing != config {
            errors.push(format!(
                "{} is out of date. Run `just generate-config` to update.",
                config_path.display()
            ));
        }
    } else {
        errors.push(format!(
            "{} does not exist. Run `just generate-config` to create.",
            config_path.display()
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(
            "Generated config/schema validation failed:\n  - {}",
            errors.join("\n  - ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_generation_contains_title() -> Result<()> {
        let schema = generate_schema()?;
        anyhow::ensure!(schema.contains("\"title\""), "schema title missing");
        anyhow::ensure!(
            schema.contains(&format!("{} configuration", config::APP_NAME)),
            "schema description missing"
        );
        anyhow::ensure!(schema.contains("\"$schema\""), "schema metadata missing");
        Ok(())
    }

    #[test]
    fn config_generation_has_sections() -> Result<()> {
        let c = generate_example_config()?;
        anyhow::ensure!(c.contains("[board]"), "board section missing");
        anyhow::ensure!(c.contains("[wiki]"), "wiki section missing");
        anyhow::ensure!(c.contains("#:schema"), "schema reference missing");
        Ok(())
    }
}
