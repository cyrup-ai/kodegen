//! API routes for the memory system
//! This module defines the HTTP routes and endpoints

use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use super::handlers::{
    create_memory, delete_memory, get_health, get_memory, get_metrics, search_memories,
    update_memory,
};
use crate::memory::SurrealMemoryManager;

/// Create the main API router
pub fn create_router(memory_manager: Arc<SurrealMemoryManager>) -> Router {
    Router::new()
        // Memory operations
        .route("/memories", post(create_memory))
        .route("/memories/:id", get(get_memory))
        .route("/memories/:id", put(update_memory))
        .route("/memories/:id", delete(delete_memory))
        .route("/memories/search", post(search_memories))
        // Health and monitoring
        .route("/health", get(get_health))
        .route("/metrics", get(get_metrics))
        // Inject memory manager as application state
        .with_state(memory_manager)
}
