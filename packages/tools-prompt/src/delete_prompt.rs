use super::manager::PromptManager;
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::prompt::{DeletePromptArgs, DeletePromptPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct DeletePromptTool {
    manager: PromptManager,
}

impl DeletePromptTool {
    pub async fn new() -> Result<Self, McpError> {
        let manager = PromptManager::new();
        manager.init().await?;
        Ok(Self { manager })
    }
}

impl Tool for DeletePromptTool {
    type Args = DeletePromptArgs;
    type PromptArgs = DeletePromptPromptArgs;

    fn name() -> &'static str {
        "delete_prompt"
    }

    fn description() -> &'static str {
        "Delete a prompt template. Requires confirm=true for safety. This action cannot be undone. \
         Default prompts can be deleted but will be recreated on next initialization."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true // Deletes file
    }

    fn idempotent() -> bool {
        false // Second deletion will fail (file gone)
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        if !args.confirm {
            return Err(McpError::InvalidArguments(
                "Must set confirm=true to delete a prompt".into(),
            ));
        }

        self.manager
            .delete_prompt(&args.name)
            .await
            .map_err(McpError::Other)?;

        Ok(json!({
            "success": true,
            "name": args.name,
            "message": format!("Prompt '{}' deleted successfully", args.name)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I delete a prompt?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use delete_prompt to remove a prompt template:\n\n\
                     Example:\n\
                     ```\n\
                     delete_prompt({\n\
                       \"name\": \"my_prompt\",\n\
                       \"confirm\": true\n\
                     })\n\
                     ```\n\n\
                     IMPORTANT:\n\
                     - You must set confirm=true for safety\n\
                     - This action is permanent and cannot be undone\n\
                     - Default prompts will be recreated on next initialization if deleted\n\n\
                     The deletion will fail if:\n\
                     - confirm is not true\n\
                     - The prompt does not exist",
                ),
            },
        ])
    }
}
