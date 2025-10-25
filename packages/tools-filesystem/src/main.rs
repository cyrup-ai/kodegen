//! Filesystem Category SSE Server
//!
//! Serves filesystem tools via SSE/HTTPS transport using kodegen_mcp_server_core.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("filesystem", |config, _tracker| {
        let tool_router = ToolRouter::new();
        let prompt_router = PromptRouter::new();
        let managers = Managers::new();

        // Create search manager for search tools
        let search_manager = Arc::new(kodegen_tools_filesystem::search::SearchManager::new(
            config.clone(),
        ));

        // Get configuration values
        let file_read_line_limit = config.get_file_read_line_limit();

        // Register all 14 filesystem tools
        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::ReadFileTool::new(
                file_read_line_limit,
                config.clone(),
            ),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::ReadMultipleFilesTool::new(
                file_read_line_limit,
                config.clone(),
            ),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::WriteFileTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::MoveFileTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::DeleteFileTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::DeleteDirectoryTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::ListDirectoryTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::CreateDirectoryTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::GetFileInfoTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::EditBlockTool::new(config.clone()),
        );

        // Search tools using SearchManager
        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::search::StartSearchTool::new(search_manager.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::search::GetMoreSearchResultsTool::new(search_manager.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::search::StopSearchTool::new(search_manager.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::search::ListSearchesTool::new(search_manager.clone()),
        );

        // CRITICAL: Start cleanup task after all tools are registered
        search_manager.start_cleanup_task();

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
