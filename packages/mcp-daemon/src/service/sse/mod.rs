//! SSE (Server-Sent Events) transport module for MCP Claude Code integration
//!
//! This module provides a complete SSE server implementation that enables
//! Claude Code to connect to the Kodegen daemon via the SSE transport protocol.
//!
//! ## Architecture
//!
//! The SSE transport uses a dual-endpoint architecture:
//! - GET /sse - Establishes persistent SSE connection with session management
//! - POST /messages - Handles JSON-RPC requests routed by session ID
//!
//! ## Components
//!
//! - `events` - SSE event types and wire format encoding
//! - `session` - Session management and lifecycle
//! - `server` - HTTP server with SSE and messages endpoints
//! - `bridge` - Communication bridge to kodegen-axum MCP server
//! - `encoder` - SSE wire format encoding per RFC 6455

pub mod bridge;
pub mod encoder;
pub mod events;
pub mod server;
pub mod session;

use anyhow::Result;
use std::net::SocketAddr;

// Re-export only the main server type for external use
pub use server::SseServer;
use tokio::sync::oneshot;

/// SSE server configuration
#[derive(Debug, Clone)]
pub struct SseConfig {
    /// Port to bind SSE server to (default: 8080)
    pub port: u16,
    /// MCP server URL to bridge requests to
    pub mcp_server_url: String,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Ping interval for keep-alive (seconds)
    pub ping_interval: u64,
    /// Session timeout (seconds)
    pub session_timeout: u64,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,

    // MCP Bridge HTTP client configuration
    /// Request timeout in seconds
    pub mcp_timeout: u64,
    /// TCP keepalive timeout in seconds
    pub mcp_keepalive_timeout: u64,
    /// Max idle connections in pool
    pub mcp_max_idle_connections: usize,
    /// HTTP User-Agent header
    pub mcp_user_agent: String,

    // Retry configuration for transient failures
    /// Max retry attempts for critical operations
    pub mcp_max_retries: u32,
    /// Base delay between retries in milliseconds (exponential backoff)
    pub mcp_retry_delay_ms: u64,
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            mcp_server_url: "http://127.0.0.1:3000".to_string(),
            max_connections: 100,
            ping_interval: 30,
            session_timeout: 300,
            cors_origins: vec!["*".to_string()],

            // MCP Bridge defaults
            mcp_timeout: 30,
            mcp_keepalive_timeout: 90,
            mcp_max_idle_connections: 10,
            mcp_user_agent: "Kodegen-Daemon/1.0".to_string(),

            // Retry defaults
            mcp_max_retries: 3,
            mcp_retry_delay_ms: 100,
        }
    }
}

/// Start the SSE server with given configuration
pub async fn start_sse_server(config: SseConfig, shutdown_rx: oneshot::Receiver<()>) -> Result<()> {
    let addr: SocketAddr = ([127, 0, 0, 1], config.port).into();
    let server = SseServer::new(config);
    server.serve(addr, shutdown_rx).await
}
