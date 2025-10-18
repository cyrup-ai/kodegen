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

use anyhow::Result;
use rmcp::handler::server::router::{tool::ToolRouter, prompt::PromptRouter};
use std::collections::HashSet;
use kodegen_utils::usage_tracker::UsageTracker;

// Stub function for library mode (not fully implemented)
pub async fn build_routers<T>(
    _config_manager: &kodegen_config::ConfigManager,
    _usage_tracker: &UsageTracker,
    _enabled_categories: &Option<HashSet<String>>,
) -> Result<(ToolRouter<T>, PromptRouter<T>)> {
    anyhow::bail!("Library mode not yet fully implemented - use the binary version.")
}
