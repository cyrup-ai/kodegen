use anyhow::{Context, Result, anyhow};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use toml::Value as TomlValue;

#[cfg(target_os = "macos")]
use plist::Value as PlistValue;

use crate::ConfigFormat;

/// Zero-allocation config merger for different formats
pub struct ConfigMerger {
    /// Pre-allocated KODEGEN.ᴀɪ config template
    kodegen_config: KodegenConfig,
}

#[derive(Clone)]
struct KodegenConfig {
    json: JsonValue,
    toml: TomlValue,
    yaml: YamlValue,
    #[cfg(target_os = "macos")]
    plist: PlistValue,
}

impl ConfigMerger {
    /// Create a new config merger with pre-allocated templates
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        let kodegen_config = KodegenConfig {
            json: serde_json::json!({
                "mcpServers": {
                    "kodegen": {
                        "command": "kodegen",
                        "args": ["--proxy-sse", "http://localhost:8081/sse"],
                        "env": {}
                    }
                }
            }),
            toml: TomlValue::Table({
                let mut map = toml::map::Map::new();
                let mut mcp_servers = toml::map::Map::new();
                let mut kodegen = toml::map::Map::new();
                kodegen.insert(
                    "command".to_string(),
                    TomlValue::String("kodegen".to_string()),
                );
                kodegen.insert(
                    "args".to_string(),
                    TomlValue::Array(vec![
                        TomlValue::String("--proxy-sse".to_string()),
                        TomlValue::String("http://localhost:8081/sse".to_string()),
                    ]),
                );
                mcp_servers.insert("kodegen".to_string(), TomlValue::Table(kodegen));
                map.insert("mcpServers".to_string(), TomlValue::Table(mcp_servers));
                map
            }),
            yaml: {
                let yaml_str = r"
mcpServers:
  kodegen:
    command: kodegen
    args:
      - --proxy-sse
      - http://localhost:8081/sse
    env: {}
";
                serde_yaml::from_str(yaml_str)
                    .ok()
                    .unwrap_or(YamlValue::Null)
            },
            #[cfg(target_os = "macos")]
            plist: {
                use plist::Value;

                let mut kodegen = plist::Dictionary::new();
                kodegen.insert("command".to_string(), Value::String("kodegen".to_string()));
                kodegen.insert(
                    "args".to_string(),
                    Value::Array(vec![
                        Value::String("--proxy-sse".to_string()),
                        Value::String("http://localhost:8081/sse".to_string()),
                    ]),
                );
                kodegen.insert(
                    "env".to_string(),
                    Value::Dictionary(plist::Dictionary::new()),
                );

                let mut servers = plist::Dictionary::new();
                servers.insert("kodegen".to_string(), Value::Dictionary(kodegen));

                let mut root = plist::Dictionary::new();
                root.insert("mcpServers".to_string(), Value::Dictionary(servers));

                Value::Dictionary(root)
            },
        };

        Self { kodegen_config }
    }

    /// Merge KODEGEN.ᴀɪ config into existing config with zero allocation where possible
    ///
    /// # Errors
    ///
    /// Returns an error if the config parsing or serialization fails for the given format.
    #[inline]
    pub fn merge(&self, existing: &str, format: ConfigFormat) -> Result<String> {
        match format {
            ConfigFormat::Json => self.merge_json(existing),
            ConfigFormat::Toml => self.merge_toml(existing),
            ConfigFormat::Yaml => self.merge_yaml(existing),
            ConfigFormat::Plist => self.merge_plist(existing),
        }
    }

    /// Merge JSON config with optimal performance
    #[inline]
    fn merge_json(&self, existing: &str) -> Result<String> {
        let mut config: JsonValue = if existing.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(existing)?
        };

        // Fast path: check if already configured
        if let Some(servers) = config.get("mcpServers")
            && servers.get("kodegen").is_some()
        {
            return Ok(existing.to_string());
        }

        // Merge efficiently
        if let Some(obj) = config.as_object_mut() {
            if !obj.contains_key("mcpServers") {
                obj.insert("mcpServers".to_string(), serde_json::json!({}));
            }

            if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                servers.insert(
                    "kodegen".to_string(),
                    self.kodegen_config.json["mcpServers"]["kodegen"].clone(),
                );
            }
        }

        Ok(serde_json::to_string_pretty(&config)?)
    }

    /// Merge TOML config with optimal performance
    #[inline]
    fn merge_toml(&self, existing: &str) -> Result<String> {
        let mut config: TomlValue = if existing.trim().is_empty() {
            toml::Value::Table(toml::map::Map::new())
        } else {
            toml::from_str(existing)?
        };

        // Fast path: check if already configured
        if let Some(table) = config.as_table()
            && let Some(servers) = table.get("mcpServers").and_then(|v| v.as_table())
            && servers.contains_key("kodegen")
        {
            return Ok(existing.to_string());
        }

        // Merge efficiently
        if let Some(table) = config.as_table_mut() {
            if !table.contains_key("mcpServers") {
                table.insert(
                    "mcpServers".to_string(),
                    TomlValue::Table(toml::map::Map::new()),
                );
            }

            if let Some(servers) = table.get_mut("mcpServers").and_then(|v| v.as_table_mut()) {
                servers.insert(
                    "kodegen".to_string(),
                    self.kodegen_config.toml["mcpServers"]["kodegen"].clone(),
                );
            }
        }

        Ok(toml::to_string_pretty(&config)?)
    }

    /// Merge YAML config with proper YAML parsing and serialization
    #[inline]
    fn merge_yaml(&self, existing: &str) -> Result<String> {
        let mut config: YamlValue = if existing.trim().is_empty() {
            YamlValue::Mapping(serde_yaml::Mapping::new())
        } else {
            serde_yaml::from_str(existing)
                .map_err(|e| anyhow!("Failed to parse existing YAML: {e}"))?
        };

        // Fast path: check if already configured
        if let YamlValue::Mapping(ref map) = config
            && let Some(YamlValue::Mapping(servers)) =
                map.get(YamlValue::String("mcpServers".to_string()))
            && servers.contains_key(YamlValue::String("kodegen".to_string()))
        {
            return Ok(existing.to_string());
        }

        // Merge efficiently
        if let YamlValue::Mapping(ref mut map) = config {
            if !map.contains_key(YamlValue::String("mcpServers".to_string())) {
                map.insert(
                    YamlValue::String("mcpServers".to_string()),
                    YamlValue::Mapping(serde_yaml::Mapping::new()),
                );
            }

            if let Some(YamlValue::Mapping(servers)) =
                map.get_mut(YamlValue::String("mcpServers".to_string()))
                && let YamlValue::Mapping(ref template_servers) = self.kodegen_config.yaml
                && let Some(YamlValue::Mapping(kodegen_map)) =
                    template_servers.get(YamlValue::String("mcpServers".to_string()))
                && let Some(kodegen_entry) =
                    kodegen_map.get(YamlValue::String("kodegen".to_string()))
            {
                servers.insert(
                    YamlValue::String("kodegen".to_string()),
                    kodegen_entry.clone(),
                );
            }
        }

        serde_yaml::to_string(&config).map_err(|e| anyhow!("Failed to serialize YAML: {e}"))
    }

    /// Merge Plist config with proper plist parsing and serialization (macOS only)
    #[cfg(target_os = "macos")]
    #[inline]
    fn merge_plist(&self, existing: &str) -> Result<String> {
        use plist::Value;

        let mut config: Value = if existing.trim().is_empty() {
            Value::Dictionary(plist::Dictionary::new())
        } else {
            plist::from_reader(std::io::Cursor::new(existing.as_bytes()))
                .context("Failed to parse existing plist")?
        };

        // Fast path: check if already configured
        if let Value::Dictionary(ref dict) = config
            && let Some(Value::Dictionary(servers)) = dict.get("mcpServers")
            && servers.contains_key("kodegen")
        {
            return Ok(existing.to_string());
        }

        // Merge efficiently
        if let Value::Dictionary(ref mut dict) = config {
            // Ensure mcpServers exists
            if !dict.contains_key("mcpServers") {
                dict.insert(
                    "mcpServers".to_string(),
                    Value::Dictionary(plist::Dictionary::new()),
                );
            }

            // Insert kodegen config
            if let Some(Value::Dictionary(servers)) = dict.get_mut("mcpServers")
                && let Value::Dictionary(ref template_root) = self.kodegen_config.plist
                && let Some(Value::Dictionary(template_servers)) = template_root.get("mcpServers")
                && let Some(kodegen_config) = template_servers.get("kodegen")
            {
                servers.insert("kodegen".to_string(), kodegen_config.clone());
            }
        }

        // Serialize to XML plist format
        let mut output = Vec::new();
        plist::to_writer_xml(&mut output, &config).context("Failed to serialize plist")?;

        String::from_utf8(output).context("Failed to convert plist to UTF-8")
    }

    /// Plist format not supported on non-macOS platforms
    #[cfg(not(target_os = "macos"))]
    #[inline]
    fn merge_plist(&self, _existing: &str) -> Result<String> {
        Err(anyhow!("Plist format only supported on macOS"))
    }
}

impl Default for ConfigMerger {
    fn default() -> Self {
        Self::new()
    }
}
