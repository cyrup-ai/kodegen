//! List Memory Libraries Tool - List all unique library names

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::claude_agent::{ListMemoryLibrariesArgs, ListMemoryLibrariesPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::sync::Arc;
use std::collections::HashSet;

use crate::memory::core::manager::coordinator::MemoryCoordinator;
use crate::memory::core::ops::filter::MemoryFilter;

#[derive(Clone)]
pub struct ListMemoryLibrariesTool {
    coordinator: Arc<MemoryCoordinator>,
}

impl ListMemoryLibrariesTool {
    pub fn new(coordinator: Arc<MemoryCoordinator>) -> Self {
        Self { coordinator }
    }
}

impl Tool for ListMemoryLibrariesTool {
    type Args = ListMemoryLibrariesArgs;
    type PromptArgs = ListMemoryLibrariesPromptArgs;

    fn name() -> &'static str {
        "list_memory_libraries"
    }

    fn description() -> &'static str {
        "List all unique memory library names that have been created. \
         Returns a list of all libraries that contain at least one memory. \
         Use this to discover what libraries are available for recall."
    }

    fn read_only() -> bool {
        true
    }

    async fn execute(&self, _args: Self::Args) -> Result<Value, McpError> {
        // Get all memories and extract unique tags
        let filter = MemoryFilter::new();
        let memories = self
            .coordinator
            .get_memories(filter)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to fetch memories: {}", e)))?;

        // Collect all unique tags (library names)
        let mut libraries: HashSet<String> = HashSet::new();
        for memory in memories {
            for tag in &memory.metadata.tags {
                libraries.insert(tag.to_string());
            }
        }

        // Convert to sorted vector
        let mut libraries_vec: Vec<String> = libraries.into_iter().collect();
        libraries_vec.sort();

        let count = libraries_vec.len();

        Ok(json!({
            "libraries": libraries_vec,
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
