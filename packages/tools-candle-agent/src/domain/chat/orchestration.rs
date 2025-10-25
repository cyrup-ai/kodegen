//! Chat loop orchestration utilities for multi-stage tool calling
//!
//! This module provides helper functions for the 4-stage tool calling process:
//! - Stage 1: Tool Selection
//! - Stage 2: Function Calling  
//! - Stage 3: Tool Execution (handled by `SweetMcpRouter`)
//! - Stage 4: Result Interpretation

use anyhow::{Context, Result};
use rmcp::model::Tool as ToolInfo;
use serde_json::json;
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};

use super::templates;
use super::types::responses::{FinalResponse, OpenAIFunctionCallResponse, ToolSelectionResponse};

/// Format tools as simple text list for Stage 1 (Tool Selection)
#[must_use]
pub fn format_tools_for_selection(tools: &[ToolInfo]) -> String {
    tools
        .iter()
        .map(|tool| {
            let desc = tool.description.as_deref().unwrap_or("No description");
            format!("- {}: {}", tool.name, desc)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format tools in `OpenAI` tools format for Stage 2 (Function Calling)
///
/// # Errors
///
/// Returns error if serialization to JSON fails
pub fn format_tools_openai(tools: &[ToolInfo]) -> Result<String> {
    let openai_tools: Vec<serde_json::Value> = tools
        .iter()
        .map(|tool| {
            let description = tool.description.as_deref().unwrap_or("");

            json!({
                "type": "function",
                "function": {
                    "name": tool.name.as_ref(),
                    "description": description,
                    "parameters": &*tool.input_schema
                }
            })
        })
        .collect();

    serde_json::to_string_pretty(&openai_tools)
        .context("Failed to serialize tools to OpenAI format")
}

/// Format tool execution results for Stage 4 (Result Interpretation)
#[must_use]
pub fn format_tool_results(
    tool_calls: &[super::types::responses::ToolCall],
    results: &[(String, Result<serde_json::Value, String>)],
) -> String {
    results
        .iter()
        .enumerate()
        .map(|(idx, (call_id, result))| {
            let tool_name = tool_calls
                .get(idx)
                .map_or("unknown", |tc| tc.function.name.as_str());
            match result {
                Ok(value) => format!(
                    "Tool: {}\nCall ID: {}\nStatus: Success\nResult: {}",
                    tool_name,
                    call_id,
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
                ),
                Err(error) => {
                    format!("Tool: {tool_name}\nCall ID: {call_id}\nStatus: Error\nError: {error}")
                }
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Render Stage 1 prompt (Tool Selection)
///
/// # Errors
///
/// Returns error if template rendering fails
pub fn render_stage1_prompt(user_input: &str, available_tools: &[ToolInfo]) -> Result<String> {
    let mut variables = HashMap::new();
    variables.insert("user_input".to_string(), user_input.to_string());
    variables.insert(
        "available_tools".to_string(),
        format_tools_for_selection(available_tools),
    );

    templates::render_template("tool_selection", &variables)
        .context("Failed to render tool_selection template")
}

/// Render Stage 2 prompt (Function Calling)
///
/// # Errors
///
/// Returns error if template rendering or tool formatting fails
pub fn render_stage2_prompt(user_input: &str, selected_tools: &[ToolInfo]) -> Result<String> {
    let mut variables = HashMap::new();
    variables.insert("user_input".to_string(), user_input.to_string());
    variables.insert(
        "tools_json".to_string(),
        format_tools_openai(selected_tools)?,
    );

    templates::render_template("function_calling", &variables)
        .context("Failed to render function_calling template")
}

/// Render Stage 4 prompt (Result Interpretation)
///
/// # Errors
///
/// Returns error if template rendering or JSON serialization fails
pub fn render_stage4_prompt(
    user_message: &str,
    tool_calls: &[super::types::responses::ToolCall],
    results: &[(String, Result<serde_json::Value, String>)],
) -> Result<String> {
    let mut variables = HashMap::new();
    variables.insert("user_message".to_string(), user_message.to_string());

    // Serialize tool calls to JSON
    let tool_calls_json =
        serde_json::to_string_pretty(tool_calls).context("Failed to serialize tool calls")?;
    variables.insert("tool_calls_json".to_string(), tool_calls_json);

    // Format tool results
    variables.insert(
        "tool_results".to_string(),
        format_tool_results(tool_calls, results),
    );

    templates::render_template("result_interpretation", &variables)
        .context("Failed to render result_interpretation template")
}

/// Parse Stage 1 response (Tool Selection)
///
/// # Errors
///
/// Returns error if JSON parsing fails
pub fn parse_tool_selection_response(json_str: &str) -> Result<ToolSelectionResponse> {
    serde_json::from_str(json_str).context("Failed to parse tool selection response")
}

/// Parse Stage 2 response (Function Calling)
///
/// # Errors
///
/// Returns error if JSON parsing fails
pub fn parse_function_call_response(json_str: &str) -> Result<OpenAIFunctionCallResponse> {
    serde_json::from_str(json_str).context("Failed to parse function call response")
}

/// Parse Stage 4 response (Final Response)
///
/// # Errors
///
/// Returns error if JSON parsing fails
pub fn parse_final_response(json_str: &str) -> Result<FinalResponse> {
    serde_json::from_str(json_str).context("Failed to parse final response")
}

#[must_use]
pub fn get_selected_tool_schemas(
    selected_names: &[String],
    available_tools: &[ToolInfo],
) -> Vec<ToolInfo> {
    available_tools
        .iter()
        .filter(|tool| {
            selected_names
                .iter()
                .any(|n| n.as_str() == tool.name.as_ref())
        })
        .cloned()
        .collect()
}

/// Helper to collect `AsyncStream` into String
#[must_use]
pub async fn collect_stream_to_string(
    mut stream: Pin<
        Box<dyn Stream<Item = crate::domain::context::chunks::CandleStringChunk> + Send>,
    >,
) -> String {
    let mut result = String::new();
    while let Some(chunk) = stream.next().await {
        result.push_str(&chunk.text);
    }
    result
}
