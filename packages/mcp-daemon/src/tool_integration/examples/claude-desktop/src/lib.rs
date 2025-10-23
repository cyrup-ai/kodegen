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

/// Detect Claude Desktop version on Windows using PowerShell
#[cfg(target_os = "windows")]
fn detect_version_windows() -> Option<String> {
    // Strategy 1: Query file version using PowerShell (most reliable)
    if let Some(version) = detect_version_windows_powershell() {
        return Some(version);
    }
    
    // Strategy 2: Check LOCALAPPDATA path directly
    if let Some(version) = detect_version_windows_file_path() {
        return Some(version);
    }
    
    None
}

/// Query Claude.exe file version using PowerShell Get-Item
#[cfg(target_os = "windows")]
fn detect_version_windows_powershell() -> Option<String> {
    // Build path to Claude.exe
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let claude_exe_path = format!("{}\\Programs\\Claude\\Claude.exe", local_app_data);
    
    // PowerShell command to get file version
    let ps_command = format!(
        "(Get-Item '{}').VersionInfo.FileVersion",
        claude_exe_path
    );
    
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_command])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.trim();
    
    if version.is_empty() {
        return None;
    }
    
    Some(version.to_string())
}

/// Fallback: Check if Claude.exe exists and try to read package.json
#[cfg(target_os = "windows")]
fn detect_version_windows_file_path() -> Option<String> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let claude_path = format!("{}\\Programs\\Claude", local_app_data);
    
    // Try to read package.json if it exists (some electron apps expose this)
    let package_json_path = format!("{}\\resources\\app\\package.json", claude_path);
    
    if let Ok(content) = std::fs::read_to_string(&package_json_path) {
        // Simple JSON parsing for "version": "X.Y.Z"
        if let Some(version_pos) = content.find("\"version\"") {
            let after_version = &content[version_pos..];
            if let Some(colon_pos) = after_version.find(':') {
                let after_colon = &after_version[colon_pos + 1..];
                if let Some(quote_start) = after_colon.find('"') {
                    let after_quote = &after_colon[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        let version = &after_quote[..quote_end];
                        return Some(version.to_string());
                    }
                }
            }
        }
    }
    
    None
}

/// Detect Claude Desktop version on Linux using multiple package managers
#[cfg(target_os = "linux")]
fn detect_version_linux() -> Option<String> {
    // Strategy 1: Check dpkg (Debian/Ubuntu)
    if let Some(version) = detect_version_linux_dpkg() {
        return Some(version);
    }
    
    // Strategy 2: Check rpm (Fedora/RHEL)
    if let Some(version) = detect_version_linux_rpm() {
        return Some(version);
    }
    
    // Strategy 3: Check snap
    if let Some(version) = detect_version_linux_snap() {
        return Some(version);
    }
    
    // Strategy 4: Check flatpak
    if let Some(version) = detect_version_linux_flatpak() {
        return Some(version);
    }
    
    // Strategy 5: Check direct install path
    if let Some(version) = detect_version_linux_direct() {
        return Some(version);
    }
    
    None
}

/// Check dpkg for claude-desktop package
#[cfg(target_os = "linux")]
fn detect_version_linux_dpkg() -> Option<String> {
    let output = std::process::Command::new("dpkg")
        .args(["-l", "claude-desktop"])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // dpkg -l output format:
    // ii  claude-desktop  1.2.3  amd64  Claude Desktop Application
    // Extract version from 3rd column
    for line in stdout.lines() {
        if line.contains("claude-desktop") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let version = parts[2];
                if !version.is_empty() && version != "<none>" {
                    return Some(version.to_string());
                }
            }
        }
    }
    
    None
}

/// Check rpm for claude-desktop package
#[cfg(target_os = "linux")]
fn detect_version_linux_rpm() -> Option<String> {
    let output = std::process::Command::new("rpm")
        .args(["-q", "claude-desktop", "--queryformat", "%{VERSION}"])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.trim();
    
    if version.is_empty() || version.contains("not installed") {
        return None;
    }
    
    Some(version.to_string())
}

/// Check snap for claude-desktop
#[cfg(target_os = "linux")]
fn detect_version_linux_snap() -> Option<String> {
    let output = std::process::Command::new("snap")
        .args(["info", "claude-desktop"])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // snap info output:
    // name:      claude-desktop
    // version:   1.2.3
    // ...
    for line in stdout.lines() {
        if line.starts_with("version:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let version = parts[1].trim();
                if !version.is_empty() {
                    return Some(version.to_string());
                }
            }
        }
    }
    
    None
}

/// Check flatpak for Claude Desktop
#[cfg(target_os = "linux")]
fn detect_version_linux_flatpak() -> Option<String> {
    let output = std::process::Command::new("flatpak")
        .args(["info", "com.anthropic.Claude"])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // flatpak info output:
    //          ID: com.anthropic.Claude
    //     Version: 1.2.3
    // ...
    for line in stdout.lines() {
        if line.contains("Version:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let version = parts[1].trim();
                if !version.is_empty() {
                    return Some(version.to_string());
                }
            }
        }
    }
    
    None
}

/// Check direct installation in /opt/Claude
#[cfg(target_os = "linux")]
fn detect_version_linux_direct() -> Option<String> {
    // Try to read package.json from standard install location
    let package_json_paths = [
        "/opt/Claude/resources/app/package.json",
        "/opt/claude-desktop/resources/app/package.json",
    ];
    
    for path in &package_json_paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            // Simple JSON parsing for "version": "X.Y.Z"
            if let Some(version_pos) = content.find("\"version\"") {
                let after_version = &content[version_pos..];
                if let Some(colon_pos) = after_version.find(':') {
                    let after_colon = &after_version[colon_pos + 1..];
                    if let Some(quote_start) = after_colon.find('"') {
                        let after_quote = &after_colon[quote_start + 1..];
                        if let Some(quote_end) = after_quote.find('"') {
                            let version = &after_quote[..quote_end];
                            return Some(version.to_string());
                        }
                    }
                }
            }
        }
    }
    
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
            Ok(std::process::Command::new("osascript")
                .args(&[
                    "-e", "tell application \"Claude\" to quit",
                    "-e", "delay 2",
                    "-e", "tell application \"Claude\" to activate",
                ])
                .output()
                .map(|_| "Claude Desktop restarted".to_string())
                .map_err(|e| Error::msg(format!("Failed to restart: {}", e)))?)
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
            Ok(std::process::Command::new("cmd")
                .args(&["/C", "start", "", "Claude"])
                .output()
                .map(|_| "Claude Desktop restarted".to_string())
                .map_err(|e| Error::msg(format!("Failed to restart: {}", e)))?)
        }
        _ => {
            // Linux - harder to restart generically
            Ok("Please restart Claude Desktop manually".to_string())
        }
    }
}