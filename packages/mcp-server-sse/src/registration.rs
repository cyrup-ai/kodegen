use kodegen_mcp_tool::Tool;
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::sync::Arc;

/// Register a single tool with both routers
///
/// Takes ownership of the tool, wraps it in Arc once, clones that Arc for both routes.
/// This ensures the tool instance is shared between tool and prompt routers.
///
/// Example usage:
/// ```
/// let (tool_router, prompt_router) = register_tool(
///     tool_router,
///     prompt_router,
///     ReadFileTool::new(config.clone()),
/// );
/// ```
pub fn register_tool<S, T>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    tool: T,
) -> (ToolRouter<S>, PromptRouter<S>)
where
    S: Send + Sync + 'static,
    T: Tool,
{
    let tool = Arc::new(tool);
    let tool_router = tool_router.with_route(tool.clone().arc_into_tool_route());
    let prompt_router = prompt_router.with_route(tool.arc_into_prompt_route());
    (tool_router, prompt_router)
}

/// Register an already-Arc-wrapped tool
///
/// Use this when you need to hold a reference to the tool for cleanup tasks.
/// Avoids creating the tool twice - uses the same Arc for both registration and cleanup.
///
/// Example usage:
/// ```
/// let thinking_tool = Arc::new(SequentialThinkingTool::new());
/// let (tool_router, prompt_router) = register_tool_arc(
///     tool_router,
///     prompt_router,
///     thinking_tool.clone(),
/// );
/// thinking_tool.start_cleanup_task();  // Hold reference for background task
/// ```
pub fn register_tool_arc<S, T>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    tool: Arc<T>,
) -> (ToolRouter<S>, PromptRouter<S>)
where
    S: Send + Sync + 'static,
    T: Tool,
{
    let tool_router = tool_router.with_route(tool.clone().arc_into_tool_route());
    let prompt_router = prompt_router.with_route(tool.arc_into_prompt_route());
    (tool_router, prompt_router)
}
