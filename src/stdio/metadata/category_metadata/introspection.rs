//! Introspection tools: tool call history and usage statistics

use kodegen_mcp_schema::introspection::{self, INSPECT_TOOL_CALLS, INSPECT_USAGE_STATS};
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn introspection_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: INSPECT_TOOL_CALLS,
            category: "introspection",
            description: "Get recent tool call history with their arguments and outputs. Returns chronological list of tool calls made during this session. Supports paginati...",
            schema: build_schema::<introspection::InspectToolCallsArgs>(),
        },
        ToolMetadata {
            name: INSPECT_USAGE_STATS,
            category: "introspection",
            description: "Get usage statistics for debugging and analysis. Returns summary of tool usage, success/failure rates, and performance metrics.' } async fn execute...",
            schema: build_schema::<introspection::InspectUsageStatsArgs>(),
        },
    ]
}
