// Category SSE Server: Browser Tools
//
// This binary serves browser automation tools over SSE/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30440.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, ShutdownHook, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::sync::Arc;

// Wrapper to impl ShutdownHook for Arc<BrowserManager>
struct BrowserManagerWrapper(Arc<kodegen_tools_browser::BrowserManager>);

impl ShutdownHook for BrowserManagerWrapper {
    fn shutdown(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let manager = self.0.clone();
        Box::pin(async move {
            manager.shutdown().await
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("browser", |_config, _tracker| {
        let mut tool_router = ToolRouter::new();
        let mut prompt_router = PromptRouter::new();
        let mut managers = Managers::new();

        // Initialize browser manager
        let browser_manager = kodegen_tools_browser::BrowserManager::global();
        managers.register(BrowserManagerWrapper(browser_manager.clone()));

        // Register all browser tools (need BrowserManager)
        use kodegen_tools_browser::*;

        // Core browser automation tools (8 tools)
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserNavigateTool::new(browser_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserClickTool::new(browser_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserTypeTextTool::new(browser_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserScreenshotTool::new(browser_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserExtractTextTool::new(browser_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserScrollTool::new(browser_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserWaitTool::new(),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserWaitForTool::new(browser_manager.clone()),
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
