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
        Ok(vec![])
    }
}
