use crate::ConfigManager;
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::config::{SetConfigValueArgs, SetConfigValuePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct SetConfigValueTool {
    config_manager: ConfigManager,
}

impl SetConfigValueTool {
    #[must_use]
    pub fn new(config_manager: ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for SetConfigValueTool {
    type Args = SetConfigValueArgs;
    type PromptArgs = SetConfigValuePromptArgs;

    fn name() -> &'static str {
        "set_config_value"
    }

    fn description() -> &'static str {
        "Set a specific configuration value by key.\n\n\
         WARNING: Should be used in a separate chat from file operations and \n\
         command execution to prevent security issues.\n\n\
         Config keys include:\n\
         - blocked_commands (array)\n\
         - default_shell (string)\n\
         - allowed_directories (array of paths)\n\
         - file_read_line_limit (number, max lines for read_file)\n\
         - file_write_line_limit (number, max lines per write_file call)\n\n\
         IMPORTANT: Setting allowed_directories to an empty array ([]) allows full access \n\
         to the entire file system."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![] // No prompt arguments needed
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Set the value
        self.config_manager
            .set_value(&args.key, args.value.clone())
            .await?;

        // Get updated config
        let updated_config = self.config_manager.get_config();

        Ok(json!({
            "message": format!("Successfully set {} to {:?}", args.key, args.value),
            "updated_config": updated_config
        }))
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I update server configuration?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use set_config_value to update configuration. Examples:\n\n\
                     Block additional commands:\n\
                     {\"key\": \"blocked_commands\", \"value\": [\"rm\", \"sudo\", \"wget\"]}\n\n\
                     Change shell:\n\
                     {\"key\": \"default_shell\", \"value\": \"/bin/bash\"}\n\n\
                     Restrict directories:\n\
                     {\"key\": \"allowed_directories\", \"value\": [\"/home/user/projects\"]}\n\n\
                     Adjust line limits:\n\
                     {\"key\": \"file_read_line_limit\", \"value\": 2000}",
                ),
            },
        ])
    }
}
