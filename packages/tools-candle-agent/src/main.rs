//! Candle-Agent Category SSE Server
//!
//! Serves memory tools via SSE/HTTPS transport using kodegen_mcp_server_core.

use anyhow::{Result, anyhow};
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::sync::Arc;

use kodegen_candle_agent::capability::registry::TextEmbeddingModel;
use kodegen_candle_agent::memory::core::manager::coordinator::MemoryCoordinator;
use kodegen_candle_agent::tools::{MemorizeTool, RecallTool, ListMemoryLibrariesTool};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize MemoryCoordinator before run_sse_server (async initialization)
    let coordinator = initialize_memory_coordinator().await?;

    run_sse_server("candle-agent", move |_config, _tracker| {
        let mut tool_router = ToolRouter::new();
        let mut prompt_router = PromptRouter::new();
        let managers = Managers::new();

        // Register memory tools (3 tools)
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            MemorizeTool::new(coordinator.clone()),
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            RecallTool::new(coordinator.clone()),
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            ListMemoryLibrariesTool::new(coordinator.clone()),
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}

async fn initialize_memory_coordinator() -> Result<Arc<MemoryCoordinator>> {
    // Get embedding model from registry (Stella 400M variant - registered by default)
    use kodegen_candle_agent::capability::registry::FromRegistry;
    let emb_model = TextEmbeddingModel::from_registry("dunzhang/stella_en_400M_v5")
        .ok_or_else(|| anyhow!("Stella embedding model not found in registry"))?;

    // Initialize with library name - path managed internally at:
    // $XDG_CONFIG_HOME/kodegen/memory/production.db
    let coordinator = MemoryCoordinator::from_library("production", emb_model)
        .await
        .map_err(|e| anyhow!("Failed to initialize memory coordinator: {:?}", e))?;

    Ok(Arc::new(coordinator))
}
