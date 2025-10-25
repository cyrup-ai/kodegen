//! Introspection Category SSE Server
//!
//! Serves introspection tools via SSE/HTTPS transport using kodegen_mcp_server_core.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("introspection", |_config, tracker| {
        let tool_router = ToolRouter::new();
        let prompt_router = PromptRouter::new();
        let managers = Managers::new();

        // Register all 2 introspection tools
        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_introspection::GetUsageStatsTool::new(tracker.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_introspection::GetRecentToolCallsTool::new(),
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
