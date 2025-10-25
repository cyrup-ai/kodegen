//! Terminal Category SSE Server
//!
//! Serves terminal tools via SSE/HTTPS transport using kodegen_mcp_server_core.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("terminal", |config, _tracker| {
        let tool_router = ToolRouter::new();
        let prompt_router = PromptRouter::new();
        let managers = Managers::new();

        // Create managers for terminal tools
        let terminal_manager = Arc::new(kodegen_tools_terminal::TerminalManager::new());
        let command_manager =
            kodegen_tools_terminal::CommandManager::new(config.get_blocked_commands());

        // Register all 5 terminal tools
        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_terminal::StartTerminalCommandTool::new(
                terminal_manager.clone(),
                command_manager,
            ),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_terminal::ReadTerminalOutputTool::new(terminal_manager.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_terminal::SendTerminalInputTool::new(terminal_manager.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_terminal::StopTerminalCommandTool::new(terminal_manager.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_terminal::ListTerminalCommandsTool::new(terminal_manager.clone()),
        );

        // CRITICAL: Start cleanup task after all tools are registered
        terminal_manager.start_cleanup_task();

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
