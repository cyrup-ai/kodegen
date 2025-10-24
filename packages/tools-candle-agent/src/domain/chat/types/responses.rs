use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response type for tool selection stage in multi-stage chat loop
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolSelectionResponse {
    pub reasoning: String,
    pub selected_tools: Vec<String>,
}

/// OpenAI-compatible function call response for tool calling stage
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OpenAIFunctionCallResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Individual tool call in `OpenAI` format
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolCall {
    pub id: String,
    pub function: FunctionCall,
    #[serde(rename = "type")]
    pub call_type: String,
}

/// Function call details within a tool call
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON string containing function arguments
}

/// Final response for interpretation stage
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FinalResponse {
    pub content: String,
}
