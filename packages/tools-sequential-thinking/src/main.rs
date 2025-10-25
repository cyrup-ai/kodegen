// Category SSE Server: Sequential Thinking
//
// This binary serves the sequential thinking tool over SSE/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30437.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, ShutdownHook, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::sync::Arc;

// Wrapper to impl ShutdownHook for Arc<SequentialThinkingTool>
struct SequentialThinkingWrapper(Arc<kodegen_tools_sequential_thinking::SequentialThinkingTool>);

impl ShutdownHook for SequentialThinkingWrapper {
    fn shutdown(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let tool = self.0.clone();
        Box::pin(async move {
            tool.shutdown().await
                .map_err(|e| anyhow::anyhow!("Failed to shutdown sequential thinking tool: {e}"))
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("sequential-thinking", |_config, _tracker| {
        let mut tool_router = ToolRouter::new();
        let mut prompt_router = PromptRouter::new();
        let mut managers = Managers::new();

        // Create sequential thinking tool
        let tool = kodegen_tools_sequential_thinking::SequentialThinkingTool::new();
        
        // Wrap in Arc and start cleanup task (required for session management)
        let tool_arc = Arc::new(tool.clone());
        tool_arc.clone().start_cleanup_task();

        // Register shutdown hook to persist active sessions on exit
        managers.register(SequentialThinkingWrapper(tool_arc));

        // Register the tool (1 tool)
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            tool,
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
