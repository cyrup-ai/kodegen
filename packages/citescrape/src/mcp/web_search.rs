//! web_search MCP tool implementation
//!
//! Performs web searches and returns structured results with titles, URLs, and snippets.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use kodegen_tool::Tool;
use kodegen_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

use crate::web_search;

// =============================================================================
// Arguments Structs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchArgs {
    /// Search query string (required)
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchPromptArgs {}

// =============================================================================
// Tool Struct
// =============================================================================

#[derive(Clone)]
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tool Trait Implementation
// =============================================================================

impl Tool for WebSearchTool {
    type Args = WebSearchArgs;
    type PromptArgs = WebSearchPromptArgs;

    fn name() -> &'static str {
        "web_search"
    }

    fn description() -> &'static str {
        "Perform a web search and return structured results with titles, URLs, and snippets.\\n\\n\
         Returns up to 10 search results with:\\n\
         - rank: Result position (1-10)\\n\
         - title: Page title\\n\
         - url: Page URL\\n\
         - snippet: Description excerpt\\n\\n\
         Example: web_search({\\\"query\\\": \\\"rust async programming\\\"})"
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn open_world() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate query is not empty
        if args.query.trim().is_empty() {
            return Err(McpError::invalid_arguments("Search query cannot be empty"));
        }

        // Perform search
        let results = web_search::search(args.query)
            .await
            .map_err(McpError::Other)?;

        // Convert to JSON response
        Ok(json!({
            "query": results.query,
            "result_count": results.results.len(),
            "results": results.results.iter().map(|r| json!({
                "rank": r.rank,
                "title": r.title,
                "url": r.url,
                "snippet": r.snippet,
            })).collect::<Vec<_>>(),
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I search the web?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The web_search tool performs web searches and returns structured results:\\n\\n\
                     **Basic usage:**\\n\
                     ```json\\n\
                     web_search({\\\"query\\\": \\\"rust async programming\\\"})\\n\
                     ```\\n\\n\
                     **Response format:**\\n\
                     ```json\\n\
                     {\\n\
                       \\\"query\\\": \\\"rust async programming\\\",\\n\
                       \\\"result_count\\\": 10,\\n\
                       \\\"results\\\": [\\n\
                         {\\n\
                           \\\"rank\\\": 1,\\n\
                           \\\"title\\\": \\\"Async Programming in Rust\\\",\\n\
                           \\\"url\\\": \\\"https://example.com/rust-async\\\",\\n\
                           \\\"snippet\\\": \\\"Learn about async/await in Rust...\\\"\\n\
                         }\\n\
                       ]\\n\
                     }\\n\
                     ```\\n\\n\
                     **Key features:**\\n\
                     - Returns up to 10 results\\n\
                     - Includes title, URL, and description snippet\\n\
                     - Results ranked by relevance\\n\
                     - Automatic retry with exponential backoff\\n\
                     - Stealth browser configuration to avoid bot detection\\n\\n\
                     **Use cases:**\\n\
                     - Research technical topics\\n\
                     - Find documentation and tutorials\\n\
                     - Gather information for code generation\\n\
                     - Discover relevant libraries and tools",
                ),
            },
        ])
    }
}
