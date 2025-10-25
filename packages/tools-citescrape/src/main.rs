// Category SSE Server: Citescrape Tools
//
// This binary serves web crawling and search tools over SSE/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30445.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, ShutdownHook, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::sync::Arc;

// Wrapper to impl ShutdownHook for Arc<BrowserManager>
struct BrowserManagerWrapper(Arc<kodegen_tools_citescrape::BrowserManager>);

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
    run_sse_server("citescrape", |_config, _tracker| {
        let mut tool_router = ToolRouter::new();
        let mut prompt_router = PromptRouter::new();
        let mut managers = Managers::new();

        // Create managers
        let session_manager = Arc::new(kodegen_tools_citescrape::CrawlSessionManager::new());
        let engine_cache = Arc::new(kodegen_tools_citescrape::SearchEngineCache::new());
        let browser_manager = Arc::new(kodegen_tools_citescrape::BrowserManager::new());

        // Register browser manager for shutdown (closes Chrome)
        managers.register(BrowserManagerWrapper(browser_manager.clone()));

        // Register all 4 citescrape tools
        use kodegen_tools_citescrape::*;

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            StartCrawlTool::new(session_manager.clone(), engine_cache.clone()),
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            GetCrawlResultsTool::new(session_manager.clone()),
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            SearchCrawlResultsTool::new(session_manager.clone(), engine_cache.clone()),
        );

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            WebSearchTool::new(browser_manager.clone()),
        );

        // CRITICAL: Start cleanup tasks after all tools are registered
        // Pattern from mcp-server/src/common/tool_registry.rs:766-767
        session_manager.start_cleanup_task();
        engine_cache.start_cleanup_task();

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
