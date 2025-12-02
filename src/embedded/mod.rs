//! Embedded .kodegen assets bundled at compile time
//!
//! This module embeds the entire .kodegen directory structure into the binary,
//! making all configuration files and toolsets available without requiring
//! filesystem access at runtime.

use include_dir::{include_dir, Dir};

/// Embedded .kodegen directory with toolset files
///
/// Directory structure:
/// ```text
/// .kodegen/
/// └── toolset/
///     └── core.json
/// ```
///
/// Access files via:
/// - `KODEGEN_ASSETS.get_file("toolset/core.json")`
/// - Helper functions: `get_file()`, `list_toolsets()`
pub static KODEGEN_ASSETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/.kodegen");

/// Get any embedded file by path relative to .kodegen/
///
/// # Examples
///
/// ```no_run
/// use kodegen::embedded;
///
/// // Get core toolset
/// let toolset = embedded::get_file("toolset/core.json");
///
/// // Get slash command
/// let aug_cmd = embedded::get_file("claude/commands/aug.md");
/// ```
pub fn get_file(path: &str) -> Option<&'static str> {
    KODEGEN_ASSETS
        .get_file(path)?
        .contents_utf8()
}

/// List all available bundled toolsets
///
/// Returns toolset names (without .json extension)
pub fn list_toolsets() -> Vec<&'static str> {
    KODEGEN_ASSETS
        .get_dir("toolset")
        .map(|dir| {
            dir.files()
                .filter_map(|file| {
                    let name = file.path().file_name()?.to_str()?;
                    name.strip_suffix(".json")
                })
                .collect()
        })
        .unwrap_or_default()
}
