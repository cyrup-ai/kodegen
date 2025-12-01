//! Auto-configure KODEGEN plugin for Claude Code

use serde_json::{json, Value};
use std::path::PathBuf;

const GITHUB_REPO: &str = "cyrup-ai/kodegen-claude-plugin";
const MARKETPLACE_KEY: &str = "cyrup-ai";
const PLUGIN_KEY: &str = "kodegen@cyrup-ai";

/// Get Claude settings path for current platform
/// - macOS/Linux: ~/.claude/settings.json
/// - Windows: %USERPROFILE%\.claude\settings.json
fn settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("settings.json")
}

/// Read existing settings or return empty object
fn read_settings(path: &PathBuf) -> Value {
    if !path.exists() {
        return json!({});
    }
    
    let Ok(content) = std::fs::read_to_string(path) else {
        return json!({});
    };
    
    serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
}

/// Write settings back to file, creating directories if needed
fn write_settings(path: &PathBuf, settings: &Value) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    let content = serde_json::to_string_pretty(settings).unwrap();
    let _ = std::fs::write(path, content);
}

/// Check if KODEGEN plugin is already enabled
fn is_plugin_enabled(settings: &Value) -> bool {
    settings
        .get("enabledPlugins")
        .and_then(|p| p.get(PLUGIN_KEY))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Ensure KODEGEN plugin is configured in Claude Code settings.
/// 
/// Returns `true` if plugin was just installed, `false` if already configured.
/// This function NEVER fails.
pub fn ensure_plugin_configured() -> bool {
    let path = settings_path();
    let mut settings = read_settings(&path);

    if is_plugin_enabled(&settings) {
        return false;
    }

    let settings_obj = match settings.as_object_mut() {
        Some(obj) => obj,
        None => {
            settings = json!({});
            settings.as_object_mut().unwrap()
        }
    };

    // Add marketplace entry
    let marketplaces = settings_obj
        .entry("extraKnownMarketplaces")
        .or_insert(json!({}));
    
    if let Some(m) = marketplaces.as_object_mut() {
        m.insert(
            MARKETPLACE_KEY.to_string(),
            json!({
                "source": {
                    "source": "github",
                    "repo": GITHUB_REPO
                }
            }),
        );
    }

    // Enable plugin
    let plugins = settings_obj
        .entry("enabledPlugins")
        .or_insert(json!({}));
    
    if let Some(p) = plugins.as_object_mut() {
        p.insert(PLUGIN_KEY.to_string(), json!(true));
    }

    write_settings(&path, &settings);
    true
}
