//! Kodegen MCP Server Library
//!
//! Exposes reusable components for MCP server implementation

pub mod cli;
pub mod common;
pub mod sse;
pub mod stdio;

// Export SseServer type for external crates
pub use sse::SseServer;
// Export StdioProxyServer type for external crates
pub use stdio::StdioProxyServer;
