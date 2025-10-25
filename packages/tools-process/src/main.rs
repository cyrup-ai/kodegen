//! Process Category SSE Server
//!
//! Serves process management tools via SSE/HTTPS transport using kodegen_mcp_server_core.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("process", |_config, _tracker| {
        let tool_router = ToolRouter::new();
        let prompt_router = PromptRouter::new();
        let managers = Managers::new();

        // Register all 2 process tools
        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_process::ListProcessesTool::new(),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_process::KillProcessTool::new(),
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
