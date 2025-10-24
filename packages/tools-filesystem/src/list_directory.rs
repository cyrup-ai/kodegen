use crate::validate_path;
use kodegen_mcp_schema::filesystem::{ListDirectoryArgs, ListDirectoryPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use tokio::fs;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct ListDirectoryTool {
    config_manager: kodegen_tools_config::ConfigManager,
}

impl ListDirectoryTool {
    #[must_use]
    pub fn new(config_manager: kodegen_tools_config::ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for ListDirectoryTool {
    type Args = ListDirectoryArgs;
    type PromptArgs = ListDirectoryPromptArgs;

    fn name() -> &'static str {
        "list_directory"
    }

    fn description() -> &'static str {
        "List all files and directories in a specified path. Returns entries prefixed with \
         [DIR] or [FILE] to distinguish types. Supports filtering hidden files. \
         Automatically validates paths."
    }

    fn read_only() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager).await?;

        let mut entries = fs::read_dir(&valid_path).await?;
        let mut items = Vec::new();
        let mut dir_count = 0;
        let mut file_count = 0;

        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files if requested
            if !args.include_hidden && name.starts_with('.') {
                continue;
            }

            let is_dir = entry.file_type().await?.is_dir();

            if is_dir {
                items.push(format!("[DIR] {name}"));
                dir_count += 1;
            } else {
                items.push(format!("[FILE] {name}"));
                file_count += 1;
            }
        }

        // Sort for consistent output
        items.sort();

        Ok(json!({
            "path": valid_path.to_string_lossy(),
            "entries": items,
            "total_items": items.len(),
            "directory_count": dir_count,
            "file_count": file_count
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "show_advanced".to_string(),
            title: None,
            description: Some("Show advanced filtering options".to_string()),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I list directory contents?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The list_directory tool shows all files and directories:\n\n\
                     1. Basic usage: list_directory({\"path\": \"/path/to/dir\"})\n\
                     2. Include hidden files: list_directory({\"path\": \"/path/to/dir\", \"include_hidden\": true})\n\n\
                     Output format:\n\
                     - Directories are prefixed with [DIR]\n\
                     - Files are prefixed with [FILE]\n\
                     - Results are sorted alphabetically\n\n\
                     The tool automatically:\n\
                     - Validates the directory path exists\n\
                     - Filters hidden files by default (unless include_hidden=true)\n\
                     - Provides counts of directories and files\n\
                     - Handles permission errors gracefully",
                ),
            },
        ])
    }
}
