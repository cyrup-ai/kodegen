//! Prompt input types for Claude agents
//!
//! Supports both plain string prompts and template-based prompts with parameters.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input for agent prompts - can be plain string or template
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "value")]
pub enum PromptInput {
    /// Plain text prompt
    #[serde(rename = "string")]
    String(String),

    /// Template-based prompt with parameters
    #[serde(rename = "template")]
    Template(PromptTemplateInput),
}

/// Template reference with parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptTemplateInput {
    /// Template name (e.g., "`code_review`", "`bug_fix`")
    pub name: String,

    /// Parameters to pass to template rendering
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
}

impl PromptInput {
    /// Convert to plain string, resolving templates if needed
    pub async fn resolve(
        &self,
        prompt_manager: &kodegen_tools_prompt::PromptManager,
    ) -> Result<String, crate::error::ClaudeError> {
        match self {
            PromptInput::String(s) => Ok(s.clone()),
            PromptInput::Template(template) => prompt_manager
                .render_prompt(&template.name, Some(template.parameters.clone()))
                .await
                .map_err(|e| crate::error::ClaudeError::PromptTemplateError {
                    template: template.name.clone(),
                    message: e.to_string(),
                }),
        }
    }
}
