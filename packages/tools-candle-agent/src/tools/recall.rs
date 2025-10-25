//! Recall Tool - Retrieve relevant memories from a library using semantic search

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::claude_agent::{RecallArgs, RecallPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::memory::core::manager::coordinator::MemoryCoordinator;
use crate::memory::core::ops::filter::MemoryFilter;

#[derive(Clone)]
pub struct RecallTool {
    coordinator: Arc<MemoryCoordinator>,
}

impl RecallTool {
    pub fn new(coordinator: Arc<MemoryCoordinator>) -> Self {
        Self { coordinator }
    }
}

impl Tool for RecallTool {
    type Args = RecallArgs;
    type PromptArgs = RecallPromptArgs;

    fn name() -> &'static str {
        "recall"
    }

    fn description() -> &'static str {
        "Retrieve relevant memories from a library using semantic search. \
         Searches for content similar to the provided context and returns the most relevant results. \
         Uses vector similarity (cosine) to find semantically related memories."
    }

    fn read_only() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Create filter for the library tag
        let filter = MemoryFilter::new().with_tags(vec![args.library.clone()]);

        // Search using coordinator's public API with filtering
        let results = self
            .coordinator
            .search_memories(&args.context, args.limit, Some(filter))
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Search failed: {}", e)))?;

        // Convert to simplified format
        let memories: Vec<Value> = results
            .into_iter()
            .map(|memory| {
                json!({
                    "id": memory.id(),
                    "content": memory.content().to_string(),
                    "created_at": memory.creation_time(),
                    "relevance_score": memory.metadata.importance
                })
            })
            .collect();

        let count = memories.len();

        Ok(json!({
            "memories": memories,
            "library": args.library,
            "count": count
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        use rmcp::model::{PromptMessageRole, PromptMessageContent};

        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use the recall tool to retrieve relevant memories using semantic search?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The recall tool retrieves memories from a library using semantic similarity search. \
                     It finds content similar to your context query, not just exact keyword matches.\n\n\
                     Basic usage:\n\
                     1. Find similar patterns: recall({\"library\": \"rust_patterns\", \"context\": \"error handling with Result\"})\n\
                     2. Recall insights: recall({\"library\": \"debugging_insights\", \"context\": \"React rendering performance\"})\n\
                     3. Get preferences: recall({\"library\": \"user_style\", \"context\": \"async patterns\"})\n\
                     4. Limit results: recall({\"library\": \"api_knowledge\", \"context\": \"rate limiting\", \"limit\": 5})\n\n\
                     Semantic search capability:\n\
                     - Finds conceptually similar content, not just keyword matches\n\
                     - Uses 1024-dimensional vector embeddings (cosine similarity)\n\
                     - Query \"authentication\" will match memories about \"login\", \"auth\", \"credentials\"\n\
                     - Results ranked by relevance_score (higher = more similar)\n\n\
                     Response format:\n\
                     {\n\
                       \"memories\": [\n\
                         {\n\
                           \"id\": \"uuid-string\",\n\
                           \"content\": \"the actual memory content text...\",\n\
                           \"created_at\": \"2025-01-15T10:30:00Z\",\n\
                           \"relevance_score\": 0.85\n\
                         }\n\
                       ],\n\
                       \"library\": \"rust_patterns\",\n\
                       \"count\": 3\n\
                     }\n\n\
                     Parameters:\n\
                     - library: The memory library to search within (required)\n\
                     - context: Your search query or context (required)\n\
                     - limit: Maximum results to return (optional, default: 10)\n\n\
                     When to use recall:\n\
                     - Need to retrieve previously stored knowledge\n\
                     - Looking for similar solutions or patterns\n\
                     - Want to remember what you learned about a topic\n\
                     - Checking user preferences or coding style\n\
                     - Finding relevant examples from past work\n\n\
                     Pro tips:\n\
                     - Write context as a question or description of what you're looking for\n\
                     - Use general concepts in context, not exact phrases\n\
                     - Check relevance_score - higher scores mean better matches\n\
                     - Adjust limit based on how many examples you need",
                ),
            },
        ])
    }
}
