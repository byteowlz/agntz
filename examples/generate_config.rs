//! Regenerate `examples/config.toml` and `examples/config.schema.json` from the
//! config structs. Run via `just generate-config`.

use std::path::PathBuf;

use anyhow::Result;

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    agntz::write_generated_files(&root)?;
    println!("Regenerated config files in {}", root.display());
    Ok(())
}
