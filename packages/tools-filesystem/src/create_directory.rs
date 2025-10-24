use crate::validate_path;
use kodegen_mcp_schema::filesystem::{CreateDirectoryArgs, CreateDirectoryPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use tokio::fs;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct CreateDirectoryTool {
    config_manager: kodegen_tools_config::ConfigManager,
}

impl CreateDirectoryTool {
    #[must_use]
    pub fn new(config_manager: kodegen_tools_config::ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for CreateDirectoryTool {
    type Args = CreateDirectoryArgs;
    type PromptArgs = CreateDirectoryPromptArgs;

    fn name() -> &'static str {
        "create_directory"
    }

    fn description() -> &'static str {
        "Create a new directory or ensure a directory exists. Can create multiple nested \
         directories in one operation. Automatically validates paths."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false // Creates only, doesn't delete
    }

    fn idempotent() -> bool {
        true // Can be called multiple times safely
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager).await?;

        fs::create_dir_all(&valid_path).await?;

        Ok(json!({
            "success": true,
            "path": valid_path.to_string_lossy(),
            "message": "Directory created successfully"
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I create directories?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The create_directory tool creates directories recursively:\n\n\
                     1. Single directory: create_directory({\"path\": \"/path/to/newdir\"})\n\
                     2. Nested directories: create_directory({\"path\": \"/path/to/nested/deep/dir\"})\n\n\
                     The tool automatically:\n\
                     - Creates all parent directories if they don't exist\n\
                     - Succeeds silently if directory already exists (idempotent)\n\
                     - Validates paths are within allowed directories\n\
                     - Normalizes paths and expands ~\n\n\
                     This is safe to call multiple times with the same path.",
                ),
            },
        ])
    }
}
