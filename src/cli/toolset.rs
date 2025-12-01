//! Shared toolset resolution logic for all kodegen commands
//!
//! Provides standardized handling of toolset specifications:
//! - Toolset names: "core", "browser", "dev" → search standard locations
//! - File paths: "/path/to/file.json", "./config.json" → use directly
//!
//! Standard search locations (in order):
//! 1. {git_root}/.kodegen/toolset/{name}.json
//! 2. {config_dir}/kodegen/toolset/{name}.json
//!
//! Cross-platform config directory resolution:
//! - Linux: ~/.config/kodegen/toolset/
//! - macOS: ~/Library/Application Support/kodegen/toolset/
//! - Windows: %APPDATA%\kodegen\toolset\

use anyhow::{bail, Context, Result};
use kodegen_config::KodegenConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

/// Toolset configuration loaded from JSON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsetConfig {
    /// List of individual tool names to enable
    pub tools: Vec<String>,
}


/// Load toolset from embedded assets
fn load_embedded_toolset(name: &str) -> Result<ToolsetConfig> {
    let path = format!("toolset/{}.json", name);
    let content = crate::embedded::get_file(&path)
        .ok_or_else(|| anyhow::anyhow!("Embedded toolset '{}' not found", name))?;
    
    serde_json::from_str(content)
        .with_context(|| format!("Failed to parse embedded toolset '{}'", name))
}

/// Resolve toolset specification to absolute file path
///
/// Auto-detects whether spec is a toolset name or file path:
/// - "core", "browser", "dev" → searches standard locations
/// - "/abs/path.json" → uses absolute path
/// - "./rel.json" → uses relative path
/// - "config.json" → uses file in current directory
///
/// # Examples
/// ```
/// // Toolset name
/// resolve_toolset_path("core").await?;
/// // → searches .kodegen/toolset/core.json and XDG config
///
/// // Absolute path
/// resolve_toolset_path("/etc/kodegen/toolset.json").await?;
/// // → uses /etc/kodegen/toolset.json
///
/// // Relative path
/// resolve_toolset_path("./my-toolset.json").await?;
/// // → resolves from current directory
/// ```
pub async fn resolve_toolset_path(spec: &str) -> Result<PathBuf> {
    let path = Path::new(spec);
    
    // Determine if this is a path or a toolset name (cross-platform)
    let is_path = path.is_absolute()                    // C:\ on Windows, / on Unix
                  || spec.contains('/')                 // Forward slash (Unix or Windows)
                  || spec.contains(MAIN_SEPARATOR)      // Backslash on Windows
                  || spec.starts_with('.')              // Relative paths like ./file
                  || path.extension().is_some();        // Has file extension like .json
    
    if is_path {
        // Treat as file path - use directly
        let path_buf = path.to_path_buf();
        if !path_buf.exists() {
            bail!("Toolset file not found: {}", path_buf.display());
        }
        return Ok(path_buf);
    }
    
    // Not a path - resolve as toolset name
    match KodegenConfig::resolve_toolset(spec) {
        Ok(path) => Ok(path),
        Err(e) => {
            // Filesystem search failed - try embedded toolsets
            let embedded_path = format!("toolset/{}.json", spec);
            if crate::embedded::get_file(&embedded_path).is_some() {
                // Create a temporary marker path that load_toolset_file() will recognize
                Ok(PathBuf::from(format!("embedded:{}", spec)))
            } else {
                // Not found anywhere - return original error with additional context
                Err(e).with_context(|| format!("Toolset '{}' not found in filesystem or embedded toolsets", spec))
            }
        }
    }
}

/// Load toolset JSON file and extract tool names
pub async fn load_toolset_file(path: &Path) -> Result<Vec<String>> {
    let path_str = path.to_string_lossy();
    
    // Check for embedded toolset marker
    if let Some(name) = path_str.strip_prefix("embedded:") {
        let config = load_embedded_toolset(name)?;
        return Ok(config.tools);
    }
    
    let content = tokio::fs::read_to_string(path).await
        .with_context(|| format!("Failed to read toolset file: {}", path.display()))?;

    let toolset: ToolsetConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse toolset file: {}", path.display()))?;

    Ok(toolset.tools)
}

/// Load and merge multiple toolsets
///
/// Each toolset specification is resolved and loaded, then all tool names
/// are merged into a single deduplicated list.
///
/// # Examples
/// ```
/// // Load multiple toolsets
/// let tools = load_and_merge_toolsets(&[
///     "core".to_string(),
///     "browser".to_string(),
///     "./custom.json".to_string()
/// ]).await?;
/// ```
pub async fn load_and_merge_toolsets(specs: &[String]) -> Result<Vec<String>> {
    let mut all_tools = std::collections::HashSet::new();

    for toolset_spec in specs {
        let path = resolve_toolset_path(toolset_spec).await?;
        let tools = load_toolset_file(&path).await?;
        all_tools.extend(tools);
    }

    Ok(all_tools.into_iter().collect())
}
