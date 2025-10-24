//! Prompt tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// GET PROMPT
// ============================================================================

/// Action enum for get_prompt tool
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

/// Arguments for `get_prompt` tool
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

/// Prompt arguments for `get_prompt` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPromptPromptArgs {}

// ============================================================================
// ADD PROMPT
// ============================================================================

/// Arguments for `add_prompt` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddPromptArgs {
    /// Filename for the prompt (without .j2.md extension)
    /// Must contain only alphanumeric characters, hyphens, and underscores
    pub name: String,

    /// Full prompt content including YAML frontmatter
    pub content: String,
}

/// Prompt arguments for `add_prompt` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddPromptPromptArgs {}

// ============================================================================
// EDIT PROMPT
// ============================================================================

/// Arguments for `edit_prompt` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditPromptArgs {
    /// Name of the prompt to edit
    pub name: String,

    /// New content (including frontmatter)
    pub content: String,
}

/// Prompt arguments for `edit_prompt` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditPromptPromptArgs {}

// ============================================================================
// DELETE PROMPT
// ============================================================================

/// Arguments for `delete_prompt` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeletePromptArgs {
    /// Name of the prompt to delete
    pub name: String,

    /// Confirmation flag (must be true)
    #[serde(default)]
    pub confirm: bool,
}

/// Prompt arguments for `delete_prompt` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeletePromptPromptArgs {}
