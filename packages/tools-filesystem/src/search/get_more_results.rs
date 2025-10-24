use super::manager::SearchManager;
use kodegen_mcp_schema::filesystem::{GetMoreSearchResultsArgs, GetMoreSearchResultsPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::sync::Arc;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct GetMoreSearchResultsTool {
    manager: Arc<SearchManager>,
}

impl GetMoreSearchResultsTool {
    #[must_use]
    pub fn new(manager: Arc<SearchManager>) -> Self {
        Self { manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for GetMoreSearchResultsTool {
    type Args = GetMoreSearchResultsArgs;
    type PromptArgs = GetMoreSearchResultsPromptArgs;

    fn name() -> &'static str {
        "get_more_search_results"
    }

    fn description() -> &'static str {
        "Get more results from an active search with offset-based pagination.\n\n\
         Supports partial result reading with:\n\
         - 'offset' (start result index, default: 0)\n\
           * Positive: Start from result N (0-based indexing)\n\
           * Negative: Read last N results from end (tail behavior)\n\
         - 'length' (max results to read, default: 100)\n\
           * Used with positive offsets for range reading\n\
           * Ignored when offset is negative (reads all requested tail results)\n\n\
         Examples:\n\
         - offset: 0, length: 100     → First 100 results\n\
         - offset: 200, length: 50    → Results 200-249\n\
         - offset: -20                → Last 20 results\n\
         - offset: -5, length: 10     → Last 5 results (length ignored)\n\n\
         Returns only results in the specified range, along with search status.\n\
         Works like read_process_output - call this repeatedly to get progressive\n\
         results from a search started with start_search."
    }

    fn read_only() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let response = self
            .manager
            .get_more_results(&args.session_id, args.offset, args.length)
            .await?;

        // Return structured JSON response
        Ok(json!({
            "session_id": response.session_id,
            "results": response.results,
            "returned_count": response.returned_count,
            "total_results": response.total_results,
            "total_matches": response.total_matches,
            "is_complete": response.is_complete,
            "is_error": response.is_error,
            "error": response.error,
            "has_more_results": response.has_more_results,
            "runtime_ms": response.runtime_ms,
            "was_incomplete": response.was_incomplete,
            "error_count": response.error_count,
            "errors": response.errors,
            "results_limited": response.results_limited,
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
                    "How do I read results from a streaming search?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use get_more_search_results to read results from a search started with start_search:\n\n\
                     1. Read first 100 results:\n\
                        get_more_search_results({\"session_id\": \"search_1_123\", \"offset\": 0, \"length\": 100})\n\n\
                     2. Read next page:\n\
                        get_more_search_results({\"session_id\": \"search_1_123\", \"offset\": 100, \"length\": 100})\n\n\
                     3. Read last 20 results:\n\
                        get_more_search_results({\"session_id\": \"search_1_123\", \"offset\": -20})\n\n\
                     The response shows:\n\
                     - Current search status (IN PROGRESS or COMPLETED)\n\
                     - Results in the requested range\n\
                     - Whether more results are available\n\
                     - Next offset to use for pagination",
                ),
            },
        ])
    }
}
