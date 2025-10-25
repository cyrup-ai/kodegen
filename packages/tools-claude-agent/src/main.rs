// Category SSE Server: Claude Agent Tools
//
// This binary serves Claude agent delegation tools over SSE/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30460.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, ShutdownHook, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::sync::Arc;

// Wrapper to impl ShutdownHook for Arc<AgentManager>
struct AgentManagerWrapper(Arc<kodegen_tools_claude_agent::AgentManager>);

impl ShutdownHook for AgentManagerWrapper {
    fn shutdown(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
        let manager = self.0.clone();
        Box::pin(async move {
            manager.shutdown().await.map_err(|e| anyhow::anyhow!("{}", e))
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("claude-agent", |_config, _tracker| {
        let mut tool_router = ToolRouter::new();
        let mut prompt_router = PromptRouter::new();
        let mut managers = Managers::new();

        // Initialize agent manager
        let agent_manager = Arc::new(kodegen_tools_claude_agent::AgentManager::new());
        managers.register(AgentManagerWrapper(agent_manager.clone()));

        // Initialize prompt manager
        let prompt_manager = Arc::new(kodegen_tools_prompt::PromptManager::new());

        // Register all 5 Claude agent tools (need AgentManager + PromptManager)
        use kodegen_tools_claude_agent::*;

        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            SpawnClaudeAgentTool::new(agent_manager.clone(), prompt_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            SendClaudeAgentPromptTool::new(agent_manager.clone(), prompt_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            ReadClaudeAgentOutputTool::new(agent_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            ListClaudeAgentsTool::new(agent_manager.clone()),
        );
        (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            TerminateClaudeAgentSessionTool::new(agent_manager.clone()),
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
