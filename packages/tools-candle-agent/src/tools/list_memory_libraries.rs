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
        use rmcp::model::{PromptMessageRole, PromptMessageContent};

        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use list_memory_libraries to see what knowledge is available?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The list_memory_libraries tool shows all unique library names that contain at least one memory. \
                     Use it for discovery and awareness of available knowledge bases.\n\n\
                     Basic usage:\n\
                     list_memory_libraries({})\n\n\
                     No parameters required - it returns all libraries.\n\n\
                     Response format:\n\
                     {\n\
                       \"libraries\": [\n\
                         \"api_knowledge\",\n\
                         \"bug_patterns\",\n\
                         \"debugging_insights\",\n\
                         \"rust_patterns\",\n\
                         \"user_preferences\"\n\
                       ],\n\
                       \"count\": 5\n\
                     }\n\n\
                     Libraries are returned in alphabetical order.\n\n\
                     When to use:\n\
                     - Discovery: \"What knowledge do I have available?\"\n\
                     - Awareness: See what topics you've memorized\n\
                     - Before recall: Check which libraries exist (though not mandatory)\n\
                     - User asks: \"What do you remember about X?\"\n\
                     - Exploration: Browse available knowledge domains\n\n\
                     Empty response:\n\
                     If no memories exist yet, you'll get:\n\
                     {\n\
                       \"libraries\": [],\n\
                       \"count\": 0\n\
                     }\n\n\
                     Workflow tips:\n\
                     - Within a session, you typically know which library you're working with\n\
                     - Use this when you need to see the full landscape of stored knowledge\n\
                     - Libraries are created automatically when you first memorize() with a library name\n\
                     - Each library is a separate namespace for organizing related memories",
                ),
            },
        ])
    }
}
