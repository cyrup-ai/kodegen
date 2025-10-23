use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_tool::Tool;
use super::manager::PromptManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPromptArgs {
    /// Action to perform
    pub action: GetPromptAction,
    
    /// Prompt name (for get/render actions)
    #[serde(default)]
    pub name: Option<String>,
    
    /// Category filter (for `list_prompts` action)
    #[serde(default)]
    pub category: Option<String>,
    
    /// Parameters for rendering (for render action)
    #[serde(default)]
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GetPromptAction {
    /// List all categories with counts
    ListCategories,
    /// List all prompts (optionally filtered by category)
    ListPrompts,
    /// Get prompt metadata and raw template
    Get,
    /// Render prompt with parameters
    Render,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPromptPromptArgs {}

#[derive(Clone)]
pub struct GetPromptTool {
    manager: PromptManager,
}

impl GetPromptTool {
    pub async fn new() -> Result<Self, McpError> {
        let manager = PromptManager::new();
        manager.init().await?;
        Ok(Self { manager })
    }
}

impl Tool for GetPromptTool {
    type Args = GetPromptArgs;
    type PromptArgs = GetPromptPromptArgs;

    fn name() -> &'static str {
        "get_prompt"
    }

    fn description() -> &'static str {
        "Browse and retrieve prompt templates. \n\n\
         Actions:\n\
         - list_categories: Show all prompt categories\n\
         - list_prompts: List all prompts (optionally filtered by category)\n\
         - get: Get prompt metadata and raw template content\n\
         - render: Render prompt with parameters\n\n\
         Examples:\n\
         - get_prompt({\"action\": \"list_categories\"})\n\
         - get_prompt({\"action\": \"list_prompts\", \"category\": \"onboarding\"})\n\
         - get_prompt({\"action\": \"get\", \"name\": \"getting_started\"})\n\
         - get_prompt({\"action\": \"render\", \"name\": \"analyze_project\", \"parameters\": {\"project_path\": \"/path\"}})"
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        match args.action {
            GetPromptAction::ListCategories => self.list_categories().await,
            GetPromptAction::ListPrompts => self.list_prompts(args.category.as_deref()).await,
            GetPromptAction::Get => {
                let name = args.name.ok_or_else(|| {
                    McpError::InvalidArguments("name required for get action".into())
                })?;
                self.get_prompt(&name).await
            }
            GetPromptAction::Render => {
                let name = args.name.ok_or_else(|| {
                    McpError::InvalidArguments("name required for render action".into())
                })?;
                self.render_prompt(&name, args.parameters).await
            }
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I browse and use prompts?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use get_prompt to browse and retrieve prompt templates:\n\n\
                     1. List all categories:\n\
                     ```\n\
                     get_prompt({\"action\": \"list_categories\"})\n\
                     ```\n\n\
                     2. List prompts (all or by category):\n\
                     ```\n\
                     get_prompt({\"action\": \"list_prompts\"})\n\
                     get_prompt({\"action\": \"list_prompts\", \"category\": \"onboarding\"})\n\
                     ```\n\n\
                     3. Get raw prompt content:\n\
                     ```\n\
                     get_prompt({\"action\": \"get\", \"name\": \"getting_started\"})\n\
                     ```\n\n\
                     4. Render prompt with parameters:\n\
                     ```\n\
                     get_prompt({\n\
                       \"action\": \"render\",\n\
                       \"name\": \"analyze_project\",\n\
                       \"parameters\": {\"project_path\": \"/my/project\"}\n\
                     })\n\
                     ```"
                ),
            },
        ])
    }
}

impl GetPromptTool {
    async fn list_categories(&self) -> Result<Value, McpError> {
        let prompts = self.manager.list_prompts()
            .await
            .map_err(McpError::Other)?;
        
        // Group by category and count
        let mut category_map: HashMap<String, usize> = HashMap::new();
        for prompt in prompts {
            for cat in prompt.metadata.categories {
                *category_map.entry(cat).or_insert(0) += 1;
            }
        }
        
        let categories: Vec<_> = category_map.into_iter()
            .map(|(name, count)| json!({"name": name, "count": count}))
            .collect();
        
        Ok(json!({
            "categories": categories,
            "total": categories.len()
        }))
    }
    
    async fn list_prompts(&self, category: Option<&str>) -> Result<Value, McpError> {
        let mut prompts = self.manager.list_prompts()
            .await
            .map_err(McpError::Other)?;
        
        // Filter by category if specified
        if let Some(cat) = category {
            prompts.retain(|p| p.metadata.categories.contains(&cat.to_string()));
        }
        
        let prompts_json: Vec<_> = prompts.iter()
            .map(|p| json!({
                "name": p.filename,
                "title": p.metadata.title,
                "description": p.metadata.description,
                "categories": p.metadata.categories,
                "author": p.metadata.author,
                "verified": p.metadata.verified,
                "parameters": p.metadata.parameters,
            }))
            .collect();
        
        Ok(json!({
            "prompts": prompts_json,
            "count": prompts_json.len(),
            "category": category
        }))
    }
    
    async fn get_prompt(&self, name: &str) -> Result<Value, McpError> {
        let template = self.manager.load_prompt(name)
            .await
            .map_err(McpError::Other)?;
        
        Ok(json!({
            "name": name,
            "metadata": template.metadata,
            "content": template.content,
            "rendered": false
        }))
    }
    
    async fn render_prompt(
        &self,
        name: &str,
        parameters: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Value, McpError> {
        let rendered = self.manager.render_prompt(name, parameters)
            .await
            .map_err(McpError::Other)?;
        
        Ok(json!({
            "name": name,
            "content": rendered,
            "rendered": true
        }))
    }
}
