//! MCP bridge module
//!
//! This module provides comprehensive MCP server communication bridge
//! functionality including HTTP client setup, request forwarding, response
//! handling, and JSON-RPC validation with zero allocation patterns and
//! blazing-fast performance.

pub mod core;
pub mod forwarding;
pub mod validation;

// Re-export key types and functions for ergonomic usage
pub use core::McpBridge;

// Removed unused import: pub use forwarding::ForwardingStats;
// Removed unused import: McpBridgeBuilder
pub use validation::{create_invalid_request_response, validate_json_rpc_request};
// Removed unused import: validate_batch_requests
