use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::ConfigMerger;
use crate::{ClientConfigPlugin, ConfigFormat, ConfigPath, Platform};

pub struct RooCodePlugin;

impl ClientConfigPlugin for RooCodePlugin {
    fn client_id(&self) -> &'static str {
        "roo-code"
    }

    fn client_name(&self) -> &'static str {
        "Roo Code"
    }

    fn watch_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Roo Code is a VSCode extension, so we watch VSCode config directories
        match Platform::current() {
            Platform::Windows => {
                if let Ok(appdata) = std::env::var("APPDATA") {
                    paths.push(PathBuf::from(appdata).join("Code"));
                }
            }
            Platform::MacOS => {
                if let Some(base_dirs) = directories::BaseDirs::new() {
                    paths.push(
                        base_dirs
                            .home_dir()
                            .join("Library/Application Support/Code"),
                    );
                }
            }
            Platform::Linux => {
                if let Some(base_dirs) = directories::BaseDirs::new() {
                    paths.push(base_dirs.config_dir().join("Code"));
                }
            }
            Platform::All => {}
        }

        paths
    }

    fn config_paths(&self) -> Vec<ConfigPath> {
        let mut configs = Vec::new();

        // Roo Code stores its MCP config in VSCode's settings
        match Platform::current() {
            Platform::Windows => {
                if let Ok(appdata) = std::env::var("APPDATA") {
                    configs.push(ConfigPath {
                        path: PathBuf::from(appdata)
                            .join("Code")
                            .join("User")
                            .join("settings.json"),
                        format: ConfigFormat::Json,
                        platform: Platform::Windows,
                    });
                }
            }
            Platform::MacOS => {
                if let Some(base_dirs) = directories::BaseDirs::new() {
                    configs.push(ConfigPath {
                        path: base_dirs
                            .home_dir()
                            .join("Library/Application Support/Code")
                            .join("User")
                            .join("settings.json"),
                        format: ConfigFormat::Json,
                        platform: Platform::MacOS,
                    });
                }
            }
            Platform::Linux => {
                if let Some(base_dirs) = directories::BaseDirs::new() {
                    configs.push(ConfigPath {
                        path: base_dirs
                            .config_dir()
                            .join("Code")
                            .join("User")
                            .join("settings.json"),
                        format: ConfigFormat::Json,
                        platform: Platform::Linux,
                    });
                }
            }
            Platform::All => {}
        }

        configs
    }

    fn is_installed(&self, path: &Path) -> bool {
        // Check if VSCode config directory exists
        if !path.exists() || !path.is_dir() {
            return false;
        }

        // Check for Roo Code extension's globalStorage directory
        // This directory only exists if the extension has been installed and run
        let global_storage = path
            .join("User")
            .join("globalStorage")
            .join("rooveterinaryinc.roo-cline");

        global_storage.exists() && global_storage.is_dir()
    }

    fn inject_kodegen(&self, config_content: &str, format: ConfigFormat) -> Result<String> {
        let merger = ConfigMerger::new();
        merger.merge(config_content, format)
    }

    fn config_format(&self) -> ConfigFormat {
        ConfigFormat::Json
    }
}
