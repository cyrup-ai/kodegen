//! Tool Interface
//!
//! This module provides tool routing and execution for the Candle Agent framework.
//! Tools are executed via local implementations, remote MCP servers, and Cylo backends.
//! Users never directly call tools - they prompt naturally and the LLM
//! decides which tools to call, similar to `OpenAI` function calling.
//!
//! Key components:
//! - `CandleToolRouter`: Unified tool routing (local, remote, Cylo)
//! - OpenAI-style function calling experience
//! - Full `tokio_stream::Stream` compatibility

pub mod router;
pub mod selector;

// Re-export the router and error types
pub use router::{CandleToolRouter, CyloBackendConfig, RouterError};
pub use selector::*;

// Re-export workspace MCP types
pub use kodegen_mcp_client::KodegenClient;
pub use kodegen_mcp_tool::Tool;
pub use rmcp::model::Tool as ToolInfo; // Type alias for backwards compatibility
