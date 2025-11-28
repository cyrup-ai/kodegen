//! Configuration management tools

use kodegen_mcp_schema::config::{CONFIG_GET, CONFIG_SET};
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn config_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: CONFIG_GET,
            category: "config",
            description: "Get complete server configuration including security settings (blocked commands, allowed directories), shell preferences, resource limits, and live...",
            schema: build_schema::<kodegen_mcp_schema::config::GetConfigArgs>(),
        },
        ToolMetadata {
            name: CONFIG_SET,
            category: "config",
            description: "Set a specific configuration value by key.nn WARNING: Should be used in a separate chat from file operations and n command execution to prevent sec...",
            schema: build_schema::<kodegen_mcp_schema::config::SetConfigValueArgs>(),
        },
    ]
}
