use super::manager::PromptManager;
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddPromptArgs {
    /// Filename for the prompt (without .j2.md extension)
    /// Must contain only alphanumeric characters, hyphens, and underscores
    pub name: String,

    /// Full prompt content including YAML frontmatter
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddPromptPromptArgs {}

#[derive(Clone)]
pub struct AddPromptTool {
    manager: PromptManager,
}

impl AddPromptTool {
    pub async fn new() -> Result<Self, McpError> {
        let manager = PromptManager::new();
        manager.init().await?;
        Ok(Self { manager })
    }
}

impl Tool for AddPromptTool {
    type Args = AddPromptArgs;
    type PromptArgs = AddPromptPromptArgs;

    fn name() -> &'static str {
        "add_prompt"
    }

    fn description() -> &'static str {
        "Create a new prompt template. The content must include YAML frontmatter with metadata \
         (title, description, categories, author) followed by the template body. Template syntax \
         is validated before saving. Environment variables are accessible via {{ env.VAR }}. \
         Parameters can be defined in frontmatter and used via {{ param_name }}."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false // Creates new file, doesn't modify existing
    }

    fn idempotent() -> bool {
        false // Will fail if prompt already exists
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Add prompt (validates syntax automatically, async)
        self.manager
            .add_prompt(&args.name, &args.content)
            .await
            .map_err(McpError::Other)?;

        let path = format!("~/.kodegen/prompts/{}.j2.md", args.name);

        Ok(json!({
            "success": true,
            "name": args.name,
            "path": path,
            "message": format!("Prompt '{}' created successfully", args.name)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I create a custom prompt?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use add_prompt to create custom prompt templates:\n\n\
                     Example:\n\
                     ```\n\
                     add_prompt({\n\
                       \"name\": \"my_workflow\",\n\
                       \"content\": \"---\\n\
                     title: \\\"My Custom Workflow\\\"\\n\
                     description: \\\"Description here\\\"\\n\
                     categories: [\\\"custom\\\"]\\n\
                     author: \\\"your-name\\\"\\n\
                     parameters:\\n\
                       - name: \\\"project_path\\\"\\n\
                         description: \\\"Project to analyze\\\"\\n\
                         required: false\\n\
                         default: \\\".\\\"\\n\
                     ---\\n\
                     \\n\
                     # My Workflow\\n\
                     \\n\
                     Project: {{ project_path }}\\n\
                     User: {{ env.USER }}\\n\
                     \\\"\n\
                     })\n\
                     ```\n\n\
                     Template features:\n\
                     - {{ variable }} - Variable substitution\n\
                     - {% if condition %} - Conditionals\n\
                     - {% for item in items %} - Loops\n\
                     - {{ env.VAR }} - Environment variables\n\
                     - {{ param | filter }} - Filters\n\n\
                     The content is validated for syntax errors before saving.",
                ),
            },
        ])
    }
}
