pub mod clients;
pub mod config;
pub mod install;
pub mod watcher;

// Re-export commonly used types
use std::path::{Path, PathBuf};

use anyhow::Result;
pub use config::ConfigMerger;
pub use install::{InstallResult, install_all_clients};
use serde::{Deserialize, Serialize};

/// Core trait for MCP client configuration plugins
pub trait ClientConfigPlugin: Send + Sync {
    /// Unique identifier (e.g., "claude-desktop", "windsurf", "cursor")
    fn client_id(&self) -> &str;

    /// Human-readable name (e.g., "Claude Desktop")
    fn client_name(&self) -> &str;

    /// Get all directories to watch for this client
    fn watch_paths(&self) -> Vec<PathBuf>;

    /// Get the config file path(s) for this client
    fn config_paths(&self) -> Vec<ConfigPath>;

    /// Check if config indicates client is installed
    fn is_installed(&self, path: &Path) -> bool;

    /// Inject KODEGEN.ᴀɪ into existing config
    ///
    /// # Errors
    ///
    /// Returns an error if the config cannot be parsed or serialized for the given format.
    fn inject_kodegen(&self, config_content: &str, format: ConfigFormat) -> Result<String>;

    /// Get the default config format for this client
    fn config_format(&self) -> ConfigFormat;
}

#[derive(Debug, Clone)]
pub struct ConfigPath {
    pub path: PathBuf,
    pub format: ConfigFormat,
    pub platform: Platform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Toml,
    Yaml,
    Plist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    All,
}

impl Platform {
    #[must_use]
    pub const fn current() -> Self {
        #[cfg(target_os = "windows")]
        return Self::Windows;

        #[cfg(target_os = "macos")]
        return Self::MacOS;

        #[cfg(target_os = "linux")]
        return Self::Linux;

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return Self::All;
    }
}

/// Standard KODEGEN server configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KodegenConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<serde_json::Value>,
}

impl Default for KodegenConfig {
    fn default() -> Self {
        Self {
            command: "kodegen".to_string(),
            args: vec!["--stdio".to_string()],
            env: None,
        }
    }
}

/// Alternative HTTP-based config for clients that support it
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KodegenHttpConfig {
    #[serde(rename = "type")]
    pub transport_type: String,
    pub url: String,
}

impl Default for KodegenHttpConfig {
    fn default() -> Self {
        Self {
            transport_type: "streamable-http".to_string(),
            url: "https://kodegen.kodegen.dev:8443".to_string(),
        }
    }
}
