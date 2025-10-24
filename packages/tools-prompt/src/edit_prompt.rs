use super::manager::PromptManager;
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::prompt::{EditPromptArgs, EditPromptPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct EditPromptTool {
    manager: PromptManager,
}

impl EditPromptTool {
    pub async fn new() -> Result<Self, McpError> {
        let manager = PromptManager::new();
        manager.init().await?;
        Ok(Self { manager })
    }
}

impl Tool for EditPromptTool {
    type Args = EditPromptArgs;
    type PromptArgs = EditPromptPromptArgs;

    fn name() -> &'static str {
        "edit_prompt"
    }

    fn description() -> &'static str {
        "Edit an existing prompt template. Provide the prompt name and complete new content \
         (including YAML frontmatter). The content is validated before saving. Use get_prompt \
         to retrieve current content before editing."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true // Modifies existing file
    }

    fn idempotent() -> bool {
        true // Same content produces same result
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Edit prompt (validates syntax automatically, async)
        self.manager
            .edit_prompt(&args.name, &args.content)
            .await
            .map_err(McpError::Other)?;

        Ok(json!({
            "success": true,
            "name": args.name,
            "message": format!("Prompt '{}' updated successfully", args.name)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I edit an existing prompt?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use edit_prompt to update an existing prompt template:\n\n\
                     1. First, get the current content:\n\
                     ```\n\
                     get_prompt({\"action\": \"get\", \"name\": \"my_prompt\"})\n\
                     ```\n\n\
                     2. Then edit it:\n\
                     ```\n\
                     edit_prompt({\n\
                       \"name\": \"my_prompt\",\n\
                       \"content\": \"---\\n\
                     title: \\\"Updated Title\\\"\\n\
                     description: \\\"Updated description\\\"\\n\
                     categories: [\\\"custom\\\"]\\n\
                     author: \\\"your-name\\\"\\n\
                     ---\\n\
                     \\n\
                     Updated template content here\\n\
                     \\\"\n\
                     })\n\
                     ```\n\n\
                     The new content completely replaces the old content. \
                     Template syntax is validated before saving.",
                ),
            },
        ])
    }
}
