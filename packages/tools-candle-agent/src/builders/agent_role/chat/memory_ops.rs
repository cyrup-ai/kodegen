//! Memory operations for chat functionality

use super::super::*;
use crate::memory::core::manager::coordinator::MemoryCoordinator;
use crate::memory::core::manager::surreal::SurrealDBMemoryManager;
use surrealdb::engine::any::connect;

pub(super) async fn initialize_memory_coordinator(
    emb_model: &TextEmbeddingModel,
) -> Result<Arc<MemoryCoordinator>, String> {
    let db_path = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cyrup")
        .join("agent.db");

    // Ensure database directory exists
    if let Some(parent) = db_path.parent()
        && let Err(e) = tokio::fs::create_dir_all(parent).await
    {
        return Err(format!("Failed to create database directory: {}", e));
    }

    let db_url = format!("surrealkv://{}", db_path.display());

    // Connect to database
    let db = match connect(&db_url).await {
        Ok(db) => db,
        Err(e) => return Err(format!("Failed to connect to database: {}", e)),
    };

    // Initialize database namespace
    if let Err(e) = db.use_ns("candle").use_db("agent").await {
        return Err(format!("Failed to initialize database namespace: {}", e));
    }

    // Create SurrealDBMemoryManager
    let surreal_manager = SurrealDBMemoryManager::with_embedding_model(db, emb_model.clone());

    if let Err(e) = surreal_manager.initialize().await {
        return Err(format!("Failed to initialize memory tables: {}", e));
    }

    let surreal_arc = Arc::new(surreal_manager);

    // Create MemoryCoordinator
    let coordinator = match MemoryCoordinator::new(surreal_arc, emb_model.clone()).await {
        Ok(coord) => coord,
        Err(e) => return Err(format!("Failed to create memory coordinator: {:?}", e)),
    };

    Ok(Arc::new(coordinator))
}
