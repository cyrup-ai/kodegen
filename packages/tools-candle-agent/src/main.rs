//! Candle-Agent Category SSE Server
//!
//! Serves memory tools via SSE/HTTPS transport using kodegen_mcp_server_core.

use anyhow::{Result, anyhow};
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::sync::Arc;

use kodegen_candle_agent::capability::registry::TextEmbeddingModel;
use kodegen_candle_agent::memory::core::manager::coordinator::MemoryCoordinator;
use kodegen_candle_agent::memory::core::manager::surreal::SurrealDBMemoryManager;
use kodegen_candle_agent::tools::{MemorizeTool, RecallTool, ListMemoryLibrariesTool};
use surrealdb::engine::any::connect;

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
    // Get embedding model from registry (Stella 1.5B 1024-dim variant)
    use kodegen_candle_agent::capability::registry::FromRegistry;
    let emb_model = TextEmbeddingModel::from_registry("dunzhang/stella_en_1.5B_v5")
        .ok_or_else(|| anyhow!("Stella embedding model not found in registry"))?;

    // Database path setup
    let db_path = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kodegen")
        .join("candle-agent.db");

    // Ensure database directory exists
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| anyhow!("Failed to create database directory: {}", e))?;
    }

    let db_url = format!("surrealkv://{}", db_path.display());

    // Connect to database
    let db = connect(&db_url)
        .await
        .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

    // Initialize database namespace
    db.use_ns("kodegen")
        .use_db("candle_agent")
        .await
        .map_err(|e| anyhow!("Failed to initialize database namespace: {}", e))?;

    // Create SurrealDBMemoryManager with embedding model
    let surreal_manager = SurrealDBMemoryManager::with_embedding_model(db, emb_model.clone());

    // Initialize database tables and schema
    surreal_manager
        .initialize()
        .await
        .map_err(|e| anyhow!("Failed to initialize memory tables: {:?}", e))?;

    let surreal_arc = Arc::new(surreal_manager);

    // Create MemoryCoordinator
    let coordinator = MemoryCoordinator::new(surreal_arc, emb_model)
        .await
        .map_err(|e| anyhow!("Failed to create memory coordinator: {:?}", e))?;

    Ok(Arc::new(coordinator))
}
