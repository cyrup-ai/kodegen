//! Qwen3 Hermes-style tool schema formatting
//!
//! Converts `rmcp::model::Tool` → Qwen3 JSON Schema with <tools> wrapping.
//!
//! # Format
//! Tools are formatted as JSON Schema following `OpenAI` function calling format,
//! wrapped in Hermes-style XML tags for Qwen3 recognition.
//!
//! # References
//! - Qwen3 function calling: <https://qwen.readthedocs.io/en/latest/framework/function_call.html>
//! - Hermes format uses XML tags: `<tools>`, `<tool_call>`, `<tool_response>`
//! - Existing `OpenAI` format: [`orchestration.rs:37-54`](../../chat/orchestration.rs)

use rmcp::model::Tool as ToolInfo;
use serde_json::{Value, json};

/// Format tools for Qwen3 function calling
///
/// Converts MCP tool definitions into Qwen3-compatible Hermes format.
/// This is similar to `OpenAI` function calling format but wrapped in `<tools>` XML tags.
///
/// # Arguments
/// * `tools` - Slice of MCP tool definitions from `router.get_available_tools()`
///
/// # Returns
/// Formatted tool definitions wrapped in `<tools>` tags, or empty string if no tools.
///
/// # Example
/// ```rust
/// let tools = router.get_available_tools().await;
/// let formatted = format_tools_for_qwen3(&tools);
/// // Returns: "<tools>\n[...tool JSON schemas...]\n</tools>"
/// ```
///
/// # Implementation Notes
/// - Empty tool list returns empty string (not an error)
/// - Missing descriptions default to empty string
/// - `input_schema` is already in JSON Schema format (no conversion needed)
/// - Pretty printing makes debugging easier without performance cost
#[must_use]
pub fn format_tools_for_qwen3(tools: &[ToolInfo]) -> String {
    if tools.is_empty() {
        return String::new();
    }

    let tool_schemas: Vec<Value> = tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name.as_ref(),
                    "description": tool.description.as_deref().unwrap_or(""),
                    "parameters": tool.input_schema.as_ref()
                }
            })
        })
        .collect();

    let tools_json =
        serde_json::to_string_pretty(&tool_schemas).unwrap_or_else(|_| "[]".to_string());

    format!("<tools>\n{tools_json}\n</tools>")
}
