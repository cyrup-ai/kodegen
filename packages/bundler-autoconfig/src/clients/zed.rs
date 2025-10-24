use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{ClientConfigPlugin, ConfigFormat, ConfigPath, Platform};

pub struct ZedPlugin;

impl ClientConfigPlugin for ZedPlugin {
    fn client_id(&self) -> &'static str {
        "zed"
    }

    fn client_name(&self) -> &'static str {
        "Zed"
    }

    fn watch_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        match Platform::current() {
            Platform::MacOS => {
                if let Some(base_dirs) = directories::BaseDirs::new() {
                    paths.push(base_dirs.home_dir().join(".config").join("zed"));
                    // Also check macOS-specific location
                    paths.push(base_dirs.home_dir().join("Library/Application Support/Zed"));
                }
            }
            Platform::Linux => {
                if let Some(base_dirs) = directories::BaseDirs::new() {
                    paths.push(base_dirs.config_dir().join("zed"));
                }
            }
            _ => {
                // Zed doesn't support Windows yet
            }
        }

        paths
    }

    fn config_paths(&self) -> Vec<ConfigPath> {
        let mut configs = Vec::new();

        match Platform::current() {
            Platform::MacOS => {
                if let Some(base_dirs) = directories::BaseDirs::new() {
                    configs.push(ConfigPath {
                        path: base_dirs
                            .home_dir()
                            .join(".config")
                            .join("zed")
                            .join("settings.json"),
                        format: ConfigFormat::Json,
                        platform: Platform::MacOS,
                    });

                    configs.push(ConfigPath {
                        path: base_dirs
                            .home_dir()
                            .join("Library/Application Support/Zed")
                            .join("settings.json"),
                        format: ConfigFormat::Json,
                        platform: Platform::MacOS,
                    });
                }
            }
            Platform::Linux => {
                if let Some(base_dirs) = directories::BaseDirs::new() {
                    configs.push(ConfigPath {
                        path: base_dirs.config_dir().join("zed").join("settings.json"),
                        format: ConfigFormat::Json,
                        platform: Platform::Linux,
                    });
                }
            }
            _ => {}
        }

        configs
    }

    fn is_installed(&self, path: &Path) -> bool {
        path.exists() && path.is_dir()
    }

    fn inject_kodegen(&self, config_content: &str, _format: ConfigFormat) -> Result<String> {
        use anyhow::Context;

        let mut config: serde_json::Value = if config_content.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(config_content).context("Failed to parse Zed config")?
        };

        // Fast path: already configured?
        if let Some(servers) = config.get("context_servers")
            && servers.get("kodegen").is_some()
        {
            return Ok(config_content.to_string());
        }

        // Inject Zed format: uses context_servers not mcpServers
        if let Some(obj) = config.as_object_mut() {
            if !obj.contains_key("context_servers") {
                obj.insert("context_servers".to_string(), serde_json::json!({}));
            }

            if let Some(servers) = obj
                .get_mut("context_servers")
                .and_then(|v| v.as_object_mut())
            {
                servers.insert(
                    "kodegen".to_string(),
                    serde_json::json!({
                        "command": {
                            "path": "kodegen",
                            "args": []
                        },
                        "settings": {}
                    }),
                );
            }
        }

        serde_json::to_string_pretty(&config).context("Failed to serialize Zed config")
    }

    fn config_format(&self) -> ConfigFormat {
        ConfigFormat::Json
    }
}
