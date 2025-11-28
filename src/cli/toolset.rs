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

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

/// Toolset configuration loaded from JSON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsetConfig {
    /// List of individual tool names to enable
    pub tools: Vec<String>,
}

impl ToolsetConfig {
    /// Load toolset config from JSON file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read toolset file: {}", path.display()))?;
        
        let config: ToolsetConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse toolset file as JSON: {}", path.display()))?;
        
        Ok(config)
    }
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
    
    // Treat as toolset name - search standard locations
    let mut searched = Vec::new();
    
    // Try git root
    if let Some(git_root) = find_git_root().await {
        let path = git_root.join(format!(".kodegen/toolset/{}.json", spec));
        searched.push(path.display().to_string());
        if path.exists() {
            return Ok(path);
        }
    }
    
    // Try config dir (cross-platform)
    if let Some(config_dir) = dirs::config_dir() {
        let path = config_dir.join(format!("kodegen/toolset/{}.json", spec));
        searched.push(path.display().to_string());
        if path.exists() {
            return Ok(path);
        }
    }
    
    // Not found - show helpful error
    bail!(
        "Toolset '{}' not found. Searched:\n  {}\n\nCreate one of these files with:\n{{\n  \"tools\": [\"tool1\", \"tool2\"]\n}}",
        spec,
        searched.join("\n  ")
    );
}

/// Load toolset JSON file and extract tool names
pub async fn load_toolset_file(path: &Path) -> Result<Vec<String>> {
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

/// Find git repository root by walking up from current directory
///
/// Returns None if current directory is not inside a git repository.
pub async fn find_git_root() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    
    tokio::task::spawn_blocking(move || {
        match gix::discover(&current_dir) {
            Ok(repo) => {
                let work_dir = repo.work_dir()?;
                Some(work_dir.to_path_buf())
            }
            Err(_) => None,
        }
    })
    .await
    .ok()
    .flatten()
}
