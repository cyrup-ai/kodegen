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
        Ok(vec![])
    }
}
