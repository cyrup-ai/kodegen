//! Browser automation tools for AI agents
//!
//! Provides browser control, page navigation, and content extraction via chromiumoxide.

mod browser;
mod manager;
mod tools;
mod utils;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u64,
    
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    
    #[serde(default = "default_search_engine")]
    pub search_engine: String,
}

fn default_temperature() -> f64 { 0.7 }
fn default_max_tokens() -> u64 { 2048 }
fn default_max_steps() -> usize { 10 }
fn default_search_engine() -> String { "google".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            max_steps: default_max_steps(),
            search_engine: default_search_engine(),
        }
    }
}

/// Load config from config.yaml in package root
pub fn load_yaml_config() -> anyhow::Result<Config> {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("config.yaml");
        
    if config_path.exists() {
        let contents = fs::read_to_string(&config_path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

pub use browser::{
    BrowserContext, BrowserError, BrowserResult, BrowserWrapper, download_managed_browser,
    find_browser_executable, launch_browser,
};
pub use manager::BrowserManager;
pub use tools::{
    BrowserAgentTool, BrowserClickTool, BrowserExtractTextTool, BrowserNavigateTool,
    BrowserScreenshotTool, BrowserScrollTool, BrowserTypeTextTool, BrowserWaitTool,
    BrowserWaitForTool,
};
