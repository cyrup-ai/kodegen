use anyhow::{Context, Result};
use log::debug;
use std::path::Path;
use tokio::fs;

/// Default prompts embedded at compile time
pub const DEFAULT_PROMPTS: &[(&str, &str)] = &[
    (
        "onb_001",
        include_str!("../../../data/default_prompts/onb_001.j2.md"),
    ),
    (
        "code_001",
        include_str!("../../../data/default_prompts/code_001.j2.md"),
    ),
    (
        "refactor_example",
        include_str!("../../../data/default_prompts/refactor_example.j2.md"),
    ),
    (
        "env_demo",
        include_str!("../../../data/default_prompts/env_demo.j2.md"),
    ),
];

/// Write default prompts to the prompts directory
///
/// Only writes prompts that don't already exist (preserves user modifications)
pub async fn write_default_prompts(prompts_dir: &Path) -> Result<()> {
    for (name, content) in DEFAULT_PROMPTS {
        let path = prompts_dir.join(format!("{name}.j2.md"));

        // Only write if file doesn't exist (don't overwrite user changes)
        if !fs::try_exists(&path).await.unwrap_or(false) {
            fs::write(&path, content)
                .await
                .with_context(|| format!("Failed to write default prompt: {name}"))?;

            debug!("Wrote default prompt: {name}");
        }
    }

    Ok(())
}
