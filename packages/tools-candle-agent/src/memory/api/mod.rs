//! API module for exposing memory system functionality
//! This module is feature-gated with the "api" feature

#[cfg(feature = "api")]
pub mod handlers;
#[cfg(feature = "api")]
pub mod middleware;
#[cfg(feature = "api")]
pub mod models;
#[cfg(feature = "api")]
pub mod routes;

#[cfg(feature = "api")]
use std::net::SocketAddr;
#[cfg(feature = "api")]
use std::sync::Arc;

#[cfg(feature = "api")]
use axum::Router;

#[cfg(feature = "api")]
use crate::memory::SurrealMemoryManager;
#[cfg(feature = "api")]
use crate::memory::utils::config::APIConfig;

/// API server for the memory system
#[cfg(feature = "api")]
pub struct APIServer {
    /// Memory manager used by route handlers
    _memory_manager: Arc<SurrealMemoryManager>,
    /// API configuration
    config: APIConfig,
    /// Router with all memory API endpoints
    router: Router,
}

#[cfg(feature = "api")]
impl APIServer {
    /// Create a new API server
    pub fn new(memory_manager: Arc<SurrealMemoryManager>, config: APIConfig) -> Self {
        let router = routes::create_router(memory_manager.clone());

        Self {
            _memory_manager: memory_manager,
            config,
            router,
        }
    }

    /// Start the API server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", self.config.host, self.config.port).parse::<SocketAddr>()?;

        log::info!("API server listening on {}", addr);

        // Updated to use tokio::net::TcpListener with axum 0.8.x
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, self.router.clone()).await?;

        Ok(())
    }
}
