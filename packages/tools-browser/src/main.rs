// Category SSE Server: Browser Tools
//
// This binary serves browser automation tools over SSE/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30440.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Cli, Managers, RouterSet, ShutdownHook, register_tool};
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

        // Fixed server URL for browser loopback tools (port 30438 managed by daemon)
        let server_url = "http://127.0.0.1:30438/sse".to_string();

        // Initialize browser manager
        let browser_manager = kodegen_tools_browser::BrowserManager::global();
        managers.register(BrowserManagerWrapper(browser_manager.clone()));

        // Initialize web search browser manager (separate from main browser manager)
        let web_search_manager = kodegen_tools_browser::web_search::BrowserManager::new();

        // Register all browser tools
        use kodegen_tools_browser::*;

        // Core browser automation tools (6 tools)
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

        // Advanced browser tools (2 tools)
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserAgentTool::new(browser_manager.clone(), server_url.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            BrowserResearchTool::new(),
        );

        // Web search tool (1 tool)
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            WebSearchTool::new(Arc::new(web_search_manager)),
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
