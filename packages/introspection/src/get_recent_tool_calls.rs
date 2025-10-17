use kodegen_tool::error::McpError;
use kodegen_tool::Tool;
use kodegen_tool::tool_history;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ============================================================================
// TOOL ARGUMENTS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetRecentToolCallsArgs {
    /// Maximum number of results to return (default: 50, max: 1000)
    /// Ignored when offset is negative
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    
    /// Offset for pagination (default: 0)
    /// Positive: Start from result N (0-based, oldest to newest)
    /// Negative: Read last N results from end (tail behavior, most recent)
    #[serde(default)]
    pub offset: i64,
    
    /// Filter by specific tool name (optional)
    #[serde(default)]
    pub tool_name: Option<String>,
    
    /// Only return calls since this timestamp (ISO 8601 format)
    #[serde(default)]
    pub since: Option<String>,
}

fn default_max_results() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetRecentToolCallsPromptArgs {}


// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone, Default)]
pub struct GetRecentToolCallsTool;

impl GetRecentToolCallsTool {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for GetRecentToolCallsTool {
    type Args = GetRecentToolCallsArgs;
    type PromptArgs = GetRecentToolCallsPromptArgs;

    fn name() -> &'static str {
        "get_recent_tool_calls"
    }

    fn description() -> &'static str {
        "Get recent tool call history with their arguments and outputs. \
         Returns chronological list of tool calls made during this session. \
         Supports pagination via offset parameter (negative for tail behavior).\n\n\
         Useful for:\n\
         - Onboarding new chats about work already done\n\
         - Recovering context after chat history loss\n\
         - Debugging tool call sequences\n\
         - Navigating large tool histories with pagination\n\n\
         Note: Does not track its own calls or other meta/query tools. \
         History kept in memory (last 1000 calls, persisted to disk)."
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

    fn open_world() -> bool {
        false
    }


    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let history = tool_history::get_global_history()
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Tool history not initialized")))?;
        
        // Get filtered calls
        let calls = history.get_recent_calls(
            args.max_results,
            args.offset,
            args.tool_name.as_deref(),
            args.since.as_deref(),
        ).await;
        
        let stats = history.get_stats().await;
        
        Ok(json!({
            "summary": format!(
                "Tool Call History ({} results, {} total in memory)",
                calls.len(),
                stats.total_entries
            ),
            "calls": calls
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }


    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use get_recent_tool_calls to see what work has been done?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The get_recent_tool_calls tool helps you understand what tools have been \
                     executed and what they did. This is especially useful when:\n\n\
                     1. **New chat context**: You join a new chat and want to understand what \
                     work was already done\n\n\
                     2. **Debugging**: You want to trace the sequence of operations that led \
                     to the current state\n\n\
                     3. **Learning**: You want to see how tools were used together to accomplish \
                     a task\n\n\
                     Usage examples:\n\n\
                     ```\n\
                     # Get first 50 tool calls (default)\n\
                     get_recent_tool_calls({})\n\n\
                     # Get first 100 calls\n\
                     get_recent_tool_calls({ max_results: 100 })\n\n\
                     # Get calls 50-99 (pagination)\n\
                     get_recent_tool_calls({ offset: 50, max_results: 50 })\n\n\
                     # Get last 20 calls (most recent)\n\
                     get_recent_tool_calls({ offset: -20 })\n\n\
                     # Get last 10 read_file calls\n\
                     get_recent_tool_calls({ tool_name: \"read_file\", offset: -10 })\n\n\
                     # Get only read_file calls\n\
                     get_recent_tool_calls({ tool_name: \"read_file\" })\n\n\
                     # Get calls since a specific timestamp\n\
                     get_recent_tool_calls({ since: \"2024-10-12T20:00:00Z\" })\n\
                     ```\n\n\
                     The response includes:\n\
                     - Timestamp of each call\n\
                     - Tool name\n\
                     - Arguments passed\n\
                     - Output received\n\
                     - Execution duration in milliseconds\n\n\
                     Note: History is kept in memory (last 1000 calls) and persisted to \
                     ~/.config/kodegen-mcp/tool-history.jsonl for durability across restarts."
                ),
            },
        ])
    }
}
