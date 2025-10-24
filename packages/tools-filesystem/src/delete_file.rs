use crate::validate_path;
use kodegen_mcp_schema::filesystem::{DeleteFileArgs, DeleteFilePromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use tokio::fs;

#[derive(Clone)]
pub struct DeleteFileTool {
    config_manager: kodegen_tools_config::ConfigManager,
}

impl DeleteFileTool {
    #[must_use]
    pub fn new(config_manager: kodegen_tools_config::ConfigManager) -> Self {
        Self { config_manager }
    }
}

impl Tool for DeleteFileTool {
    type Args = DeleteFileArgs;
    type PromptArgs = DeleteFilePromptArgs;

    fn name() -> &'static str {
        "delete_file"
    }

    fn description() -> &'static str {
        "Delete a file from the filesystem. This operation is permanent and cannot be undone. \
         Only deletes files, not directories. Automatically validates paths."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true // Permanently deletes data
    }

    fn idempotent() -> bool {
        false // Deleting twice will fail (file no longer exists)
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager).await?;

        // Check if path exists and is a file
        let metadata = tokio::fs::metadata(&valid_path).await.map_err(|_| {
            McpError::ResourceNotFound(format!("File does not exist: {}", args.path))
        })?;

        if !metadata.is_file() {
            return Err(McpError::InvalidArguments(
                "Path is not a file. Use delete_directory to remove directories.".to_string(),
            ));
        }

        fs::remove_file(&valid_path).await?;

        Ok(json!({
            "success": true,
            "path": valid_path.to_string_lossy(),
            "message": "File deleted successfully"
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I safely delete files?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The delete_file tool permanently deletes files:\n\n\
                     Usage: delete_file({\"path\": \"/path/to/file.txt\"})\n\n\
                     Safety features:\n\
                     - Only deletes files, not directories (prevents accidental recursive deletion)\n\
                     - Validates path exists before attempting deletion\n\
                     - Validates path is within allowed directories\n\
                     - Returns clear error if file doesn't exist\n\n\
                     IMPORTANT: This operation is permanent and cannot be undone!\n\n\
                     To delete directories, use delete_directory instead.",
                ),
            },
        ])
    }
}
