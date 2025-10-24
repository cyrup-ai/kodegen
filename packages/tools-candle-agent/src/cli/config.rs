//! Persistent configuration management for CLI
//!
//! This module handles loading and saving CLI configuration to disk using JSON format.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// CLI configuration stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Last used model
    pub last_model: Option<String>,

    /// Default system prompt
    pub default_system_prompt: Option<String>,

    /// Default temperature
    pub default_temperature: f64,

    /// Default max tokens
    pub default_max_tokens: u64,

    /// Chat history (recent messages)
    pub history: Vec<String>,

    /// Maximum history size
    pub max_history: usize,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            last_model: None,
            default_system_prompt: Some("You are a helpful AI assistant.".to_string()),
            default_temperature: 0.0, // Greedy sampling for CLI - deterministic output
            default_max_tokens: 2000,
            history: Vec::new(),
            max_history: 100,
        }
    }
}

impl CliConfig {
    /// Create new config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Get default config file path
    pub fn default_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            let app_config = config_dir.join("cyrup");
            fs::create_dir_all(&app_config).ok();
            app_config.join("candle-chat.json")
        } else {
            PathBuf::from(".candle-chat.json")
        }
    }

    /// Load config from file
    pub fn load(path: Option<&Path>) -> Result<Self, String> {
        let config_path = path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(Self::default_path);

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse config file: {}", e))
    }

    /// Save config to file
    pub fn save(&self, path: Option<&Path>) -> Result<(), String> {
        let config_path = path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(Self::default_path);

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&config_path, contents).map_err(|e| format!("Failed to write config file: {}", e))
    }

    /// Add message to history
    pub fn add_to_history(&mut self, message: String) {
        // Avoid duplicates
        if !self.history.contains(&message) {
            self.history.push(message);

            // Trim history if too long
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }
    }

    /// Get history
    pub fn get_history(&self) -> &[String] {
        &self.history
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Update last used model
    pub fn set_last_model(&mut self, model: String) {
        self.last_model = Some(model);
    }

    /// Get last used model
    pub fn get_last_model(&self) -> Option<&str> {
        self.last_model.as_deref()
    }
}
