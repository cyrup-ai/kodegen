//! Embedded .kodegen assets bundled at compile time
//!
//! This module embeds the entire .kodegen directory structure into the binary,
//! making all configuration files, system prompts, and slash commands available
//! without requiring filesystem access at runtime.

use include_dir::{include_dir, Dir};

/// Embedded .kodegen directory with all 13 asset files
///
/// Directory structure:
/// ```text
/// .kodegen/
/// ├── claude/
/// │   ├── .mcp.json
/// │   ├── SYSTEM_PROMPT.md (1,393 lines, ~40KB)
/// │   ├── settings.local.json
/// │   └── commands/ (9 slash command files)
/// └── toolset/
///     └── core.json
/// ```
///
/// Access files via:
/// - `KODEGEN_ASSETS.get_file("claude/SYSTEM_PROMPT.md")`
/// - Helper functions: `system_prompt()`, `get_file()`
pub static KODEGEN_ASSETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/.kodegen");

/// Get the embedded SYSTEM_PROMPT.md content as a string
///
/// This is the default system prompt used by `kodegen claude` when no custom
/// prompt is provided via --system-prompt flag.
///
/// Returns None if the file doesn't exist in embedded assets (build error).
pub fn system_prompt() -> Option<&'static str> {
    KODEGEN_ASSETS
        .get_file("claude/SYSTEM_PROMPT.md")?
        .contents_utf8()
}

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
