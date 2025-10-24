//! Introspection tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// GET USAGE STATS
// ============================================================================

/// Arguments for `get_usage_stats` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GetUsageStatsArgs {
    // No arguments needed
}

/// Prompt arguments for `get_usage_stats` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GetUsageStatsPromptArgs {
    // No arguments needed
}

// ============================================================================
// GET RECENT TOOL CALLS
// ============================================================================

/// Default value for max_results
fn default_max_results() -> usize {
    50
}

/// Arguments for `get_recent_tool_calls` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
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

/// Prompt arguments for `get_recent_tool_calls` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GetRecentToolCallsPromptArgs {
    // No arguments needed
}
