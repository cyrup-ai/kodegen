//! Config tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// CONFIG VALUE TYPE
// ============================================================================

/// Configuration value that can be string, number, boolean, or array of strings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Array(Vec<String>),
}

impl ConfigValue {
    /// Convert to string value
    pub fn into_string(self) -> Result<String, String> {
        match self {
            ConfigValue::String(s) => Ok(s),
            _ => Err("Expected string".to_string()),
        }
    }

    /// Convert to number value
    pub fn into_number(self) -> Result<i64, String> {
        match self {
            ConfigValue::Number(n) => Ok(n),
            _ => Err("Expected number".to_string()),
        }
    }

    /// Convert to boolean value
    pub fn into_boolean(self) -> Result<bool, String> {
        match self {
            ConfigValue::Boolean(b) => Ok(b),
            _ => Err("Expected boolean".to_string()),
        }
    }

    /// Convert to array value
    pub fn into_array(self) -> Result<Vec<String>, String> {
        match self {
            ConfigValue::Array(arr) => Ok(arr),
            ConfigValue::String(s) if s.starts_with('[') => {
                serde_json::from_str(&s).map_err(|_| "Invalid array format".to_string())
            }
            _ => Err("Expected array".to_string()),
        }
    }
}

// ============================================================================
// GET CONFIG
// ============================================================================

/// Arguments for `get_config` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GetConfigArgs {}

/// Prompt arguments for `get_config` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GetConfigPromptArgs {}

// ============================================================================
// SET CONFIG VALUE
// ============================================================================

/// Arguments for `set_config_value` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SetConfigValueArgs {
    /// Configuration key to update
    pub key: String,

    /// New value (string, number, boolean, or array)
    pub value: ConfigValue,
}

/// Prompt arguments for `set_config_value` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SetConfigValuePromptArgs {}
