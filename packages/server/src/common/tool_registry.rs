// packages/server/src/common/tool_registry.rs
use anyhow::Result;
use rmcp::handler::server::router::{tool::ToolRouter, prompt::PromptRouter};
use std::sync::Arc;
use std::collections::HashSet;
use kodegen_utils::usage_tracker::UsageTracker;
use kodegen_tool::Tool;

/// Helper function for category checking
fn is_category_enabled(category: &str, enabled_categories: &Option<HashSet<String>>) -> bool {
    match enabled_categories {
        None => true,
        Some(set) => set.contains(category),
    }
}

/// Register a single tool with both routers
/// Takes ownership of the tool, wraps it in Arc once, clones that Arc for both routes
fn register_tool<S, T>(
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

/// Register a tool that's already Arc-wrapped (for tools with cleanup tasks)
/// Avoids creating the tool twice - uses the same Arc for both registration and cleanup
fn register_tool_arc<S, T>(
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

/// Register all available tools with the routers
pub async fn register_all_tools<S>(
    mut tool_router: ToolRouter<S>,
    mut prompt_router: PromptRouter<S>,
    config_manager: &kodegen_config::ConfigManager,
    usage_tracker: &UsageTracker,
    enabled_categories: &Option<HashSet<String>>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    // Filesystem tools
    #[cfg(feature = "filesystem")]
    if is_category_enabled("filesystem", enabled_categories) {
        (tool_router, prompt_router) = register_filesystem_tools(tool_router, prompt_router, config_manager).await?;
    }
    
    // Terminal tools
    #[cfg(feature = "terminal")]
    if is_category_enabled("terminal", enabled_categories) {
        (tool_router, prompt_router) = register_terminal_tools(tool_router, prompt_router, config_manager).await?;
    }
    
    // Process tools
    #[cfg(feature = "process")]
    if is_category_enabled("process", enabled_categories) {
        (tool_router, prompt_router) = register_process_tools(tool_router, prompt_router).await?;
    }
    
    // Introspection tools
    #[cfg(feature = "introspection")]
    if is_category_enabled("introspection", enabled_categories) {
        (tool_router, prompt_router) = register_introspection_tools(tool_router, prompt_router, usage_tracker).await?;
    }
    
    // Prompt tools
    #[cfg(feature = "prompt")]
    if is_category_enabled("prompt", enabled_categories) {
        (tool_router, prompt_router) = register_prompt_tools(tool_router, prompt_router).await?;
    }
    
    // Sequential thinking tool
    #[cfg(feature = "sequential_thinking")]
    if is_category_enabled("sequential_thinking", enabled_categories) {
        (tool_router, prompt_router) = register_sequential_thinking_tool(tool_router, prompt_router).await?;
    }
    
    // Claude agent tools
    #[cfg(feature = "claude_agent")]
    if is_category_enabled("claude_agent", enabled_categories) {
        (tool_router, prompt_router) = register_claude_agent_tools(tool_router, prompt_router).await?;
    }
    
    // Citescrape tools
    #[cfg(feature = "citescrape")]
    if is_category_enabled("citescrape", enabled_categories) {
        (tool_router, prompt_router) = register_citescrape_tools(tool_router, prompt_router).await?;
    }
    
    // Git tools
    #[cfg(feature = "git")]
    if is_category_enabled("git", enabled_categories) {
        (tool_router, prompt_router) = register_git_tools(tool_router, prompt_router).await?;
    }
    
    // GitHub tools
    #[cfg(feature = "github")]
    if is_category_enabled("github", enabled_categories) {
        (tool_router, prompt_router) = register_github_tools(tool_router, prompt_router).await?;
    }
    
    // Config tools
    #[cfg(feature = "config")]
    if is_category_enabled("config", enabled_categories) {
        (tool_router, prompt_router) = register_config_tools(tool_router, prompt_router, config_manager).await?;
    }
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "filesystem")]
async fn register_filesystem_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    config_manager: &kodegen_config::ConfigManager,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing filesystem tools");
    
    let search_manager = Arc::new(kodegen_filesystem::search::SearchManager::new(config_manager.clone()));
    
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_filesystem::ReadFileTool::new(
            config_manager.get_file_read_line_limit(),
            config_manager.clone()
        )
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_filesystem::ReadMultipleFilesTool::new(
            config_manager.get_file_read_line_limit(),
            config_manager.clone()
        )
    );
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::WriteFileTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::MoveFileTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::DeleteFileTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::DeleteDirectoryTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::ListDirectoryTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::CreateDirectoryTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::GetFileInfoTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::EditBlockTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::search::StartSearchTool::new(search_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::search::GetMoreSearchResultsTool::new(search_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::search::StopSearchTool::new(search_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_filesystem::search::ListSearchesTool::new(search_manager.clone()));
    
    // Start cleanup task after all tools are registered to avoid race conditions
    search_manager.start_cleanup_task();
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "terminal")]
async fn register_terminal_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    config_manager: &kodegen_config::ConfigManager,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing terminal tools");
    
    let terminal_manager = Arc::new(kodegen_terminal::TerminalManager::new());
    let command_manager = kodegen_terminal::CommandManager::new(config_manager.get_blocked_commands());
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_terminal::StartTerminalCommandTool::new(terminal_manager.clone(), command_manager));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_terminal::ReadTerminalOutputTool::new(terminal_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_terminal::SendTerminalInputTool::new(terminal_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_terminal::StopTerminalCommandTool::new(terminal_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_terminal::ListTerminalCommandsTool::new(terminal_manager.clone()));
    
    // Start cleanup task after all tools are registered to avoid race conditions
    terminal_manager.start_cleanup_task();
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "process")]
async fn register_process_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing process tools");
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_process::ListProcessesTool::new());
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_process::KillProcessTool::new());
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "introspection")]
async fn register_introspection_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    usage_tracker: &UsageTracker,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing introspection tools");
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_introspection::GetUsageStatsTool::new(usage_tracker.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_introspection::GetRecentToolCallsTool::new());
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "prompt")]
async fn register_prompt_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing prompt tools");
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_prompt::AddPromptTool::new().await?);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_prompt::EditPromptTool::new().await?);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_prompt::DeletePromptTool::new().await?);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_prompt::GetPromptTool::new().await?);
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "sequential_thinking")]
async fn register_sequential_thinking_tool<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing sequential thinking tool");
    
    let thinking_tool = Arc::new(kodegen_sequential_thinking::SequentialThinkingTool::new());
    
    let (tool_router, prompt_router) = register_tool_arc(
        tool_router,
        prompt_router,
        thinking_tool.clone()
    );
    
    // Start cleanup task after tool is registered to avoid race conditions
    thinking_tool.start_cleanup_task();
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "claude_agent")]
async fn register_claude_agent_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing claude agent tools");
    
    let agent_manager = Arc::new(kodegen_claude_agent::AgentManager::new());
    let prompt_manager = Arc::new(kodegen_prompt::PromptManager::new());
    prompt_manager.init().await.map_err(|e| anyhow::anyhow!("Failed to init prompt manager: {}", e))?;
    
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_claude_agent::SpawnClaudeAgentTool::new(agent_manager.clone(), prompt_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_claude_agent::ReadClaudeAgentOutputTool::new(agent_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_claude_agent::SendClaudeAgentPromptTool::new(agent_manager.clone(), prompt_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_claude_agent::TerminateClaudeAgentSessionTool::new(agent_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_claude_agent::ListClaudeAgentsTool::new(agent_manager)
    );
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "citescrape")]
async fn register_citescrape_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing citescrape tools");
    
    kodegen_citescrape::preinit_lazy_statics();
    
    let session_manager = Arc::new(kodegen_citescrape::CrawlSessionManager::new());
    let engine_cache = Arc::new(kodegen_citescrape::SearchEngineCache::new());
    
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_citescrape::StartCrawlTool::new(session_manager.clone(), engine_cache.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_citescrape::GetCrawlResultsTool::new(session_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_citescrape::SearchCrawlResultsTool::new(session_manager.clone(), engine_cache.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_citescrape::WebSearchTool::new()
    );
    
    // Start cleanup tasks after all tools are registered to avoid race conditions
    session_manager.start_cleanup_task();
    engine_cache.start_cleanup_task();
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "git")]
async fn register_git_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing git tools");
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitInitTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitOpenTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitCloneTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitDiscoverTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitBranchCreateTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitBranchDeleteTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitBranchListTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitBranchRenameTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitCommitTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitLogTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitAddTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitCheckoutTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitFetchTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitMergeTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitWorktreeAddTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitWorktreeRemoveTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitWorktreeListTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitWorktreeLockTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitWorktreeUnlockTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_git::GitWorktreePruneTool);
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "github")]
async fn register_github_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing github tools");
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::CreateIssueTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::GetIssueTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::ListIssuesTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::UpdateIssueTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::SearchIssuesTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::AddIssueCommentTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::GetIssueCommentsTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::CreatePullRequestTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::UpdatePullRequestTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::MergePullRequestTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::GetPullRequestStatusTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::GetPullRequestFilesTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::GetPullRequestReviewsTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::CreatePullRequestReviewTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::AddPullRequestReviewCommentTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodgen_github::RequestCopilotReviewTool);
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "config")]
async fn register_config_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    config_manager: &kodegen_config::ConfigManager,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing config tools");
    
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_config::GetConfigTool::new(config_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_config::SetConfigValueTool::new(config_manager.clone())
    );
    
    Ok((tool_router, prompt_router))
}
