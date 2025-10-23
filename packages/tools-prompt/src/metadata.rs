use serde::{Deserialize, Serialize};

/// Prompt metadata from YAML frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMetadata {
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    #[serde(default)]
    pub secondary_tag: Option<String>,
    pub author: String,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub votes: u32,
    #[serde(default)]
    pub parameters: Vec<ParameterDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    pub name: String,
    pub description: String,
    #[serde(default = "default_param_type")]
    pub param_type: ParameterType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
}

fn default_param_type() -> ParameterType {
    ParameterType::String
}

/// Full prompt template (metadata + content)
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub filename: String,
    pub metadata: PromptMetadata,
    pub content: String,
}
