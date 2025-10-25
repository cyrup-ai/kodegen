// Category SSE Server: Prompt Tools
//
// This binary serves prompt template management tools over SSE/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30438.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("prompt", |_config, _tracker| {
        let mut tool_router = ToolRouter::new();
        let mut prompt_router = PromptRouter::new();
        let managers = Managers::new();

        // Initialize PromptManager (async init in sync context)
        let manager = kodegen_tools_prompt::PromptManager::new();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(manager.init())
        })?;

        // Register all 4 prompt management tools with shared manager
        use kodegen_tools_prompt::*;

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            AddPromptTool::with_manager(manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            EditPromptTool::with_manager(manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            DeletePromptTool::with_manager(manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            GetPromptTool::with_manager(manager.clone()),
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
