use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::introspection::{GetUsageStatsArgs, GetUsageStatsPromptArgs};
use kodegen_utils::usage_tracker::UsageTracker;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct GetUsageStatsTool {
    usage_tracker: UsageTracker,
}

impl GetUsageStatsTool {
    #[must_use]
    pub fn new(usage_tracker: UsageTracker) -> Self {
        Self { usage_tracker }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for GetUsageStatsTool {
    type Args = GetUsageStatsArgs;
    type PromptArgs = GetUsageStatsPromptArgs;

    fn name() -> &'static str {
        "get_usage_stats"
    }

    fn description() -> &'static str {
        "Get usage statistics for debugging and analysis. Returns summary of tool usage, \
         success/failure rates, and performance metrics."
    }

    async fn execute(&self, _args: Self::Args) -> Result<Value, McpError> {
        let summary = self.usage_tracker.get_summary();
        Ok(json!({ "summary": summary }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How can I check what tools have been used and see usage statistics?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "I can show you usage statistics using the get_usage_stats tool. \
                     This provides:\n\n\
                     - Total tool calls (successful and failed)\n\
                     - Success/failure rates\n\
                     - Breakdown by category (filesystem, terminal, edit, search, etc.)\n\
                     - Per-tool call counts\n\
                     - Session information\n\
                     - Timestamps\n\n\
                     Let me get those stats for you.",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("Yes, please show me the stats."),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "[Calling get_usage_stats tool]\n\n\
                     The statistics show all tool usage since the server started. \
                     This is useful for debugging, understanding usage patterns, \
                     and identifying frequently used tools.",
                ),
            },
        ])
    }

    // Behavior annotations
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
}
