use crate::{ConfigManager, get_system_info};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::config::{GetConfigArgs, GetConfigPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct GetConfigTool {
    config_manager: ConfigManager,
}

impl GetConfigTool {
    #[must_use]
    pub fn new(config_manager: ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for GetConfigTool {
    type Args = GetConfigArgs;
    type PromptArgs = GetConfigPromptArgs;

    fn name() -> &'static str {
        "get_config"
    }

    fn description() -> &'static str {
        "Get complete server configuration including security settings (blocked commands, \
         allowed directories), shell preferences, resource limits, and live system diagnostics \
         (platform, architecture, OS version, kernel version, hostname, CPU count, memory usage)."
    }

    fn read_only() -> bool {
        true
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![] // No arguments needed
    }

    async fn execute(&self, _args: Self::Args) -> Result<Value, McpError> {
        let mut config = self.config_manager.get_config();

        // Refresh system info with current values (especially memory usage)
        config.system_info = get_system_info();

        Ok(json!(config))
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I check server configuration?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use get_config to retrieve the current server configuration. \
                     This shows blocked commands, allowed directories, shell settings, \
                     and line limits.",
                ),
            },
        ])
    }
}
