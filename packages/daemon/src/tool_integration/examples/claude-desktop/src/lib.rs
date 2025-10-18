use extism_pdk::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct Metadata {
    name: String,
    version: String,
    author: String,
    description: String,
    supported_platforms: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct DetectedTool {
    name: String,
    version: Option<String>,
    installed: bool,
    config_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ConfigUpdateRequest {
    server_name: String,
    server_config: ServerConfig,
}

#[derive(Serialize, Deserialize)]
struct ServerConfig {
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct ConfigUpdateResult {
    success: bool,
    message: String,
    restart_required: bool,
}

#[derive(Serialize, Deserialize)]
struct McpConfig {
    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, ServerConfig>,
}

#[plugin_fn]
pub fn get_metadata(_: ()) -> FnResult<Json<Metadata>> {
    let metadata = Metadata {
        name: "claude-desktop".to_string(),
        version: "0.1.0".to_string(),
        author: "Kodegen Team".to_string(),
        description: "Auto-configures Claude Desktop for Kodegen".to_string(),
        supported_platforms: vec!["windows".to_string(), "macos".to_string(), "linux".to_string()],
    };
    Ok(Json(metadata))
}

/// Detect Claude Desktop version on macOS by parsing Info.plist
#[cfg(target_os = "macos")]
fn detect_version_macos() -> Option<String> {
    // Standard Claude Desktop installation path
    let info_plist_path = "/Applications/Claude.app/Contents/Info.plist";
    
    // Attempt to read plist file
    let content = match std::fs::read_to_string(info_plist_path) {
        Ok(c) => c,
        Err(_) => return None, // App not installed or inaccessible
    };
    
    // Parse CFBundleShortVersionString using simple string matching
    // Format: <key>CFBundleShortVersionString</key>\n<string>VERSION</string>
    if let Some(key_pos) = content.find("<key>CFBundleShortVersionString</key>") {
        let after_key = &content[key_pos..];
        if let Some(string_start) = after_key.find("<string>") {
            if let Some(string_end) = after_key[string_start..].find("</string>") {
                let version_start = string_start + "<string>".len();
                let version = &after_key[version_start..string_start + string_end];
                let trimmed = version.trim();
                if trimmed.is_empty() {
                    return None;
                }
                return Some(trimmed.to_string());
            }
        }
    }
    
    None // Version key not found or malformed plist
}

/// Detect Claude Desktop version on Windows (stub for future implementation)
#[cfg(target_os = "windows")]
fn detect_version_windows() -> Option<String> {
    // TODO: Windows version detection strategies:
    // 1. Parse app.asar (requires asar extraction library)
    // 2. Read file properties from Claude.exe (requires winapi)
    // 3. Check registry keys (requires winreg crate)
    //
    // For now, return None gracefully
    // Windows users will see version: null in detection response
    None
}

/// Detect Claude Desktop version on Linux (stub for future implementation)
#[cfg(target_os = "linux")]
fn detect_version_linux() -> Option<String> {
    // TODO: Linux version detection strategies:
    // 1. Check dpkg: `dpkg -l claude-desktop`
    // 2. Check rpm: `rpm -q claude-desktop`
    // 3. Parse app.asar from /opt/Claude (if standard install)
    // 4. Query snap: `snap info claude-desktop`
    // 5. Query flatpak: `flatpak info com.anthropic.Claude`
    //
    // For now, return None gracefully
    // Linux users will see version: null in detection response
    None
}

#[plugin_fn]
pub fn detect(_: ()) -> FnResult<Json<DetectedTool>> {
    let config_path = get_config_path_internal();
    let path = std::path::Path::new(&config_path);
    
    // Check if Claude Desktop is installed by looking for its config directory
    let installed = path.parent().map(|p| p.exists()).unwrap_or(false);
    
    // Detect version based on platform
    #[cfg(target_os = "macos")]
    let version = detect_version_macos();
    
    #[cfg(target_os = "windows")]
    let version = detect_version_windows();
    
    #[cfg(target_os = "linux")]
    let version = detect_version_linux();
    
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let version: Option<String> = None;
    
    let tool = DetectedTool {
        name: "Claude Desktop".to_string(),
        version,
        installed,
        config_path: Some(config_path),
    };
    
    Ok(Json(tool))
}

#[plugin_fn]
pub fn get_config_path(_: ()) -> FnResult<String> {
    Ok(get_config_path_internal())
}

fn get_config_path_internal() -> String {
    let os = std::env::consts::OS;
    
    match os {
        "macos" => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{}/Library/Application Support/Claude/claude_desktop_config.json", home)
        }
        "windows" => {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\".to_string());
            format!("{}\\Claude\\claude_desktop_config.json", appdata)
        }
        _ => {
            // Linux and others
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{}/.config/Claude/claude_desktop_config.json", home)
        }
    }
}

#[plugin_fn]
pub fn read_config(_: ()) -> FnResult<Json<McpConfig>> {
    let config_path = get_config_path_internal();
    
    // Try to read existing config
    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            match serde_json::from_str::<McpConfig>(&content) {
                Ok(config) => Ok(Json(config)),
                Err(_) => {
                    // Return empty config if parse fails
                    Ok(Json(McpConfig {
                        mcp_servers: HashMap::new(),
                    }))
                }
            }
        }
        Err(_) => {
            // Return empty config if file doesn't exist
            Ok(Json(McpConfig {
                mcp_servers: HashMap::new(),
            }))
        }
    }
}

#[plugin_fn]
pub fn update_config(Json(request): Json<ConfigUpdateRequest>) -> FnResult<Json<ConfigUpdateResult>> {
    let config_path = get_config_path_internal();
    
    // Read current config
    let mut config = match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            serde_json::from_str::<McpConfig>(&content).unwrap_or_else(|_| McpConfig {
                mcp_servers: HashMap::new(),
            })
        }
        Err(_) => McpConfig {
            mcp_servers: HashMap::new(),
        },
    };
    
    // Add or update the Kodegen server
    config.mcp_servers.insert(request.server_name.clone(), request.server_config);
    
    // Ensure directory exists
    if let Some(parent) = std::path::Path::new(&config_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    
    // Write updated config
    match serde_json::to_string_pretty(&config) {
        Ok(json) => {
            match std::fs::write(&config_path, json) {
                Ok(_) => {
                    Ok(Json(ConfigUpdateResult {
                        success: true,
                        message: format!("Successfully added {} to Claude Desktop", request.server_name),
                        restart_required: true,
                    }))
                }
                Err(e) => {
                    Ok(Json(ConfigUpdateResult {
                        success: false,
                        message: format!("Failed to write config: {}", e),
                        restart_required: false,
                    }))
                }
            }
        }
        Err(e) => {
            Ok(Json(ConfigUpdateResult {
                success: false,
                message: format!("Failed to serialize config: {}", e),
                restart_required: false,
            }))
        }
    }
}

#[plugin_fn]
pub fn restart_tool(_: ()) -> FnResult<String> {
    let os = std::env::consts::OS;
    
    match os {
        "macos" => {
            // Try to restart Claude Desktop on macOS
            std::process::Command::new("osascript")
                .args(&[
                    "-e", "tell application \"Claude\" to quit",
                    "-e", "delay 2",
                    "-e", "tell application \"Claude\" to activate",
                ])
                .output()
                .map(|_| "Claude Desktop restarted".to_string())
                .map_err(|e| Error::msg(format!("Failed to restart: {}", e)))
        }
        "windows" => {
            // Try to restart Claude Desktop on Windows
            // First, try to close it
            std::process::Command::new("taskkill")
                .args(&["/IM", "Claude.exe", "/F"])
                .output()
                .ok();
            
            // Wait a moment
            std::thread::sleep(std::time::Duration::from_secs(2));
            
            // Try to start it again
            std::process::Command::new("cmd")
                .args(&["/C", "start", "", "Claude"])
                .output()
                .map(|_| "Claude Desktop restarted".to_string())
                .map_err(|e| Error::msg(format!("Failed to restart: {}", e)))
        }
        _ => {
            // Linux - harder to restart generically
            Ok("Please restart Claude Desktop manually".to_string())
        }
    }
}