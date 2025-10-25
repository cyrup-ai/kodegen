//! Memorize Tool - Store content in a named memory library

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::claude_agent::{MemorizeArgs, MemorizePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::memory::core::manager::coordinator::MemoryCoordinator;
use crate::memory::core::primitives::metadata::MemoryMetadata;
use crate::domain::memory::primitives::types::MemoryTypeEnum;

#[derive(Clone)]
pub struct MemorizeTool {
    coordinator: Arc<MemoryCoordinator>,
}

impl MemorizeTool {
    pub fn new(coordinator: Arc<MemoryCoordinator>) -> Self {
        Self { coordinator }
    }
}

impl Tool for MemorizeTool {
    type Args = MemorizeArgs;
    type PromptArgs = MemorizePromptArgs;

    fn name() -> &'static str {
        "memorize"
    }

    fn description() -> &'static str {
        "Store content in a named memory library with automatic embedding generation. \
         The memory will be tagged with the library name and can be retrieved later using recall(). \
         Each library is a separate namespace for organizing memories."
    }

    fn read_only() -> bool {
        false
    }

    fn idempotent() -> bool {
        false // Creates new memories each time
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Create metadata with library tag
        let mut metadata = MemoryMetadata::default();
        metadata.add_tag(&args.library);

        // Store memory using coordinator's public API
        let created = self
            .coordinator
            .add_memory(args.content, MemoryTypeEnum::LongTerm, Some(metadata))
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create memory: {}", e)))?;

        Ok(json!({
            "success": true,
            "memory_id": created.id(),
            "library": args.library,
            "message": format!("Memorized content in library '{}'", args.library)
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
                    "How do I use the memorize tool to store important knowledge for later retrieval?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The memorize tool stores content in named memory libraries with automatic semantic embeddings. \
                     Each memory is vectorized for similarity-based retrieval using recall().\n\n\
                     Basic usage:\n\
                     1. Store insight: memorize({\"library\": \"debugging_insights\", \"content\": \"Found that React re-renders happen when...\"})\n\
                     2. Store solution: memorize({\"library\": \"rust_patterns\", \"content\": \"Elegant error handling pattern using Result<T, E>...\"})\n\
                     3. Store preference: memorize({\"library\": \"user_style\", \"content\": \"User prefers async/await over raw futures\"})\n\
                     4. Store learning: memorize({\"library\": \"api_knowledge\", \"content\": \"OpenAI API rate limits are 60 requests per minute...\"})\n\n\
                     Library organization:\n\
                     - Use descriptive names that indicate the knowledge domain\n\
                     - Organize by project, topic, or category\n\
                     - Same library name groups related memories together\n\
                     - Examples: \"rust_patterns\", \"bug_fixes\", \"user_preferences\", \"api_docs\"\n\n\
                     What to memorize:\n\
                     - Important insights from debugging sessions\n\
                     - Elegant solutions worth remembering\n\
                     - User preferences and coding style notes\n\
                     - Key learnings from documentation\n\
                     - Patterns that worked well\n\
                     - Anything you want to recall later using semantic search\n\n\
                     Response format:\n\
                     {\n\
                       \"success\": true,\n\
                       \"memory_id\": \"uuid-string\",\n\
                       \"library\": \"rust_patterns\",\n\
                       \"message\": \"Memorized content in library 'rust_patterns'\"\n\
                     }\n\n\
                     The content is automatically converted to embeddings (1024-dimensional vectors) for semantic search. \
                     Later, use recall() to retrieve similar content based on context, not just exact keyword matches.",
                ),
            },
        ])
    }
}
