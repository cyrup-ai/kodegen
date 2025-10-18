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
    search_manager.clone().start_cleanup_task();
    
    let read_file = Arc::new(kodegen_filesystem::ReadFileTool::new(
        config_manager.get_file_read_line_limit(), 
        config_manager.clone()
    ));
    let read_multiple = Arc::new(kodegen_filesystem::ReadMultipleFilesTool::new(
        config_manager.get_file_read_line_limit(), 
        config_manager.clone()
    ));
    let write_file = Arc::new(kodegen_filesystem::WriteFileTool::new(config_manager.clone()));
    let move_file = Arc::new(kodegen_filesystem::MoveFileTool::new(config_manager.clone()));
    let delete_file = Arc::new(kodegen_filesystem::DeleteFileTool::new(config_manager.clone()));
    let delete_directory = Arc::new(kodegen_filesystem::DeleteDirectoryTool::new(config_manager.clone()));
    let list_directory = Arc::new(kodegen_filesystem::ListDirectoryTool::new(config_manager.clone()));
    let create_directory = Arc::new(kodegen_filesystem::CreateDirectoryTool::new(config_manager.clone()));
    let get_file_info = Arc::new(kodegen_filesystem::GetFileInfoTool::new(config_manager.clone()));
    let edit_block = Arc::new(kodegen_filesystem::EditBlockTool::new(config_manager.clone()));
    let start_search = Arc::new(kodegen_filesystem::search::StartSearchTool::new(search_manager.clone()));
    let get_more_results = Arc::new(kodegen_filesystem::search::GetMoreSearchResultsTool::new(search_manager.clone()));
    let stop_search = Arc::new(kodegen_filesystem::search::StopSearchTool::new(search_manager.clone()));
    let list_searches = Arc::new(kodegen_filesystem::search::ListSearchesTool::new(search_manager));
    
    let tool_router = tool_router
        .with_route(read_file.clone().arc_into_tool_route())
        .with_route(read_multiple.clone().arc_into_tool_route())
        .with_route(write_file.clone().arc_into_tool_route())
        .with_route(move_file.clone().arc_into_tool_route())
        .with_route(delete_file.clone().arc_into_tool_route())
        .with_route(delete_directory.clone().arc_into_tool_route())
        .with_route(list_directory.clone().arc_into_tool_route())
        .with_route(create_directory.clone().arc_into_tool_route())
        .with_route(get_file_info.clone().arc_into_tool_route())
        .with_route(edit_block.clone().arc_into_tool_route())
        .with_route(start_search.clone().arc_into_tool_route())
        .with_route(get_more_results.clone().arc_into_tool_route())
        .with_route(stop_search.clone().arc_into_tool_route())
        .with_route(list_searches.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(read_file.arc_into_prompt_route())
        .with_route(read_multiple.arc_into_prompt_route())
        .with_route(write_file.arc_into_prompt_route())
        .with_route(move_file.arc_into_prompt_route())
        .with_route(delete_file.arc_into_prompt_route())
        .with_route(delete_directory.arc_into_prompt_route())
        .with_route(list_directory.arc_into_prompt_route())
        .with_route(create_directory.arc_into_prompt_route())
        .with_route(get_file_info.arc_into_prompt_route())
        .with_route(edit_block.arc_into_prompt_route())
        .with_route(start_search.arc_into_prompt_route())
        .with_route(get_more_results.arc_into_prompt_route())
        .with_route(stop_search.arc_into_prompt_route())
        .with_route(list_searches.arc_into_prompt_route());
    
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
    
    let terminal_manager = kodegen_terminal::TerminalManager::new();
    let command_manager = kodegen_terminal::CommandManager::new(config_manager.get_blocked_commands());
    let terminal_manager_arc = Arc::new(terminal_manager.clone());
    terminal_manager_arc.start_cleanup_task();
    
    let start_cmd = Arc::new(kodegen_terminal::StartTerminalCommandTool::new(terminal_manager.clone(), command_manager));
    let read_output = Arc::new(kodegen_terminal::ReadTerminalOutputTool::new(terminal_manager.clone()));
    let send_input = Arc::new(kodegen_terminal::SendTerminalInputTool::new(terminal_manager.clone()));
    let stop_cmd = Arc::new(kodegen_terminal::StopTerminalCommandTool::new(terminal_manager.clone()));
    let list_cmds = Arc::new(kodegen_terminal::ListTerminalCommandsTool::new(terminal_manager));
    
    let tool_router = tool_router
        .with_route(start_cmd.clone().arc_into_tool_route())
        .with_route(read_output.clone().arc_into_tool_route())
        .with_route(send_input.clone().arc_into_tool_route())
        .with_route(stop_cmd.clone().arc_into_tool_route())
        .with_route(list_cmds.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(start_cmd.arc_into_prompt_route())
        .with_route(read_output.arc_into_prompt_route())
        .with_route(send_input.arc_into_prompt_route())
        .with_route(stop_cmd.arc_into_prompt_route())
        .with_route(list_cmds.arc_into_prompt_route());
    
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
    
    let list_proc = Arc::new(kodegen_process::ListProcessesTool::new());
    let kill_proc = Arc::new(kodegen_process::KillProcessTool::new());
    
    let tool_router = tool_router
        .with_route(list_proc.clone().arc_into_tool_route())
        .with_route(kill_proc.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(list_proc.arc_into_prompt_route())
        .with_route(kill_proc.arc_into_prompt_route());
    
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
    
    let get_stats = Arc::new(kodegen_introspection::GetUsageStatsTool::new(usage_tracker.clone()));
    let get_recent = Arc::new(kodegen_introspection::GetRecentToolCallsTool::new());
    
    let tool_router = tool_router
        .with_route(get_stats.clone().arc_into_tool_route())
        .with_route(get_recent.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(get_stats.arc_into_prompt_route())
        .with_route(get_recent.arc_into_prompt_route());
    
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
    
    let add = Arc::new(kodegen_prompt::AddPromptTool::new().await?);
    let edit = Arc::new(kodegen_prompt::EditPromptTool::new().await?);
    let delete = Arc::new(kodegen_prompt::DeletePromptTool::new().await?);
    let get = Arc::new(kodegen_prompt::GetPromptTool::new().await?);
    
    let tool_router = tool_router
        .with_route(add.clone().arc_into_tool_route())
        .with_route(edit.clone().arc_into_tool_route())
        .with_route(delete.clone().arc_into_tool_route())
        .with_route(get.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(add.arc_into_prompt_route())
        .with_route(edit.arc_into_prompt_route())
        .with_route(delete.arc_into_prompt_route())
        .with_route(get.arc_into_prompt_route());
    
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
    
    let tool = Arc::new(kodegen_sequential_thinking::SequentialThinkingTool::new());
    tool.clone().start_cleanup_task();
    
    let tool_router = tool_router.with_route(tool.clone().arc_into_tool_route());
    let prompt_router = prompt_router.with_route(tool.arc_into_prompt_route());
    
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
    
    let spawn = Arc::new(kodegen_claude_agent::SpawnClaudeAgentTool::new(agent_manager.clone(), prompt_manager.clone()));
    let read = Arc::new(kodegen_claude_agent::ReadClaudeAgentOutputTool::new(agent_manager.clone()));
    let send = Arc::new(kodegen_claude_agent::SendClaudeAgentPromptTool::new(agent_manager.clone(), prompt_manager.clone()));
    let terminate = Arc::new(kodegen_claude_agent::TerminateClaudeAgentSessionTool::new(agent_manager.clone()));
    let list = Arc::new(kodegen_claude_agent::ListClaudeAgentsTool::new(agent_manager));
    
    let tool_router = tool_router
        .with_route(spawn.clone().arc_into_tool_route())
        .with_route(read.clone().arc_into_tool_route())
        .with_route(send.clone().arc_into_tool_route())
        .with_route(terminate.clone().arc_into_tool_route())
        .with_route(list.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(spawn.arc_into_prompt_route())
        .with_route(read.arc_into_prompt_route())
        .with_route(send.arc_into_prompt_route())
        .with_route(terminate.arc_into_prompt_route())
        .with_route(list.arc_into_prompt_route());
    
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
    
    let session_manager = kodegen_citescrape::CrawlSessionManager::new();
    let engine_cache = kodegen_citescrape::SearchEngineCache::new();
    let session_arc = Arc::new(session_manager.clone());
    let engine_arc = Arc::new(engine_cache.clone());
    session_arc.start_cleanup_task();
    engine_arc.start_cleanup_task();
    
    let start = Arc::new(kodegen_citescrape::StartCrawlTool::new(session_manager.clone(), engine_cache.clone()));
    let get = Arc::new(kodegen_citescrape::GetCrawlResultsTool::new(session_manager.clone()));
    let search = Arc::new(kodegen_citescrape::SearchCrawlResultsTool::new(session_manager, engine_cache));
    let web = Arc::new(kodegen_citescrape::WebSearchTool::new());
    
    let tool_router = tool_router
        .with_route(start.clone().arc_into_tool_route())
        .with_route(get.clone().arc_into_tool_route())
        .with_route(search.clone().arc_into_tool_route())
        .with_route(web.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(start.arc_into_prompt_route())
        .with_route(get.arc_into_prompt_route())
        .with_route(search.arc_into_prompt_route())
        .with_route(web.arc_into_prompt_route());
    
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
    
    let git_init = Arc::new(kodgen_git::GitInitTool);
    let git_open = Arc::new(kodgen_git::GitOpenTool);
    let git_clone = Arc::new(kodgen_git::GitCloneTool);
    let git_discover = Arc::new(kodgen_git::GitDiscoverTool);
    let git_branch_create = Arc::new(kodgen_git::GitBranchCreateTool);
    let git_branch_delete = Arc::new(kodgen_git::GitBranchDeleteTool);
    let git_branch_list = Arc::new(kodgen_git::GitBranchListTool);
    let git_branch_rename = Arc::new(kodgen_git::GitBranchRenameTool);
    let git_commit = Arc::new(kodgen_git::GitCommitTool);
    let git_log = Arc::new(kodgen_git::GitLogTool);
    let git_add = Arc::new(kodgen_git::GitAddTool);
    let git_checkout = Arc::new(kodgen_git::GitCheckoutTool);
    let git_fetch = Arc::new(kodgen_git::GitFetchTool);
    let git_merge = Arc::new(kodgen_git::GitMergeTool);
    let git_worktree_add = Arc::new(kodgen_git::GitWorktreeAddTool);
    let git_worktree_remove = Arc::new(kodgen_git::GitWorktreeRemoveTool);
    let git_worktree_list = Arc::new(kodgen_git::GitWorktreeListTool);
    let git_worktree_lock = Arc::new(kodgen_git::GitWorktreeLockTool);
    let git_worktree_unlock = Arc::new(kodgen_git::GitWorktreeUnlockTool);
    let git_worktree_prune = Arc::new(kodgen_git::GitWorktreePruneTool);
    
    let tool_router = tool_router
        .with_route(git_init.clone().arc_into_tool_route())
        .with_route(git_open.clone().arc_into_tool_route())
        .with_route(git_clone.clone().arc_into_tool_route())
        .with_route(git_discover.clone().arc_into_tool_route())
        .with_route(git_branch_create.clone().arc_into_tool_route())
        .with_route(git_branch_delete.clone().arc_into_tool_route())
        .with_route(git_branch_list.clone().arc_into_tool_route())
        .with_route(git_branch_rename.clone().arc_into_tool_route())
        .with_route(git_commit.clone().arc_into_tool_route())
        .with_route(git_log.clone().arc_into_tool_route())
        .with_route(git_add.clone().arc_into_tool_route())
        .with_route(git_checkout.clone().arc_into_tool_route())
        .with_route(git_fetch.clone().arc_into_tool_route())
        .with_route(git_merge.clone().arc_into_tool_route())
        .with_route(git_worktree_add.clone().arc_into_tool_route())
        .with_route(git_worktree_remove.clone().arc_into_tool_route())
        .with_route(git_worktree_list.clone().arc_into_tool_route())
        .with_route(git_worktree_lock.clone().arc_into_tool_route())
        .with_route(git_worktree_unlock.clone().arc_into_tool_route())
        .with_route(git_worktree_prune.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(git_init.arc_into_prompt_route())
        .with_route(git_open.arc_into_prompt_route())
        .with_route(git_clone.arc_into_prompt_route())
        .with_route(git_discover.arc_into_prompt_route())
        .with_route(git_branch_create.arc_into_prompt_route())
        .with_route(git_branch_delete.arc_into_prompt_route())
        .with_route(git_branch_list.arc_into_prompt_route())
        .with_route(git_branch_rename.arc_into_prompt_route())
        .with_route(git_commit.arc_into_prompt_route())
        .with_route(git_log.arc_into_prompt_route())
        .with_route(git_add.arc_into_prompt_route())
        .with_route(git_checkout.arc_into_prompt_route())
        .with_route(git_fetch.arc_into_prompt_route())
        .with_route(git_merge.arc_into_prompt_route())
        .with_route(git_worktree_add.arc_into_prompt_route())
        .with_route(git_worktree_remove.arc_into_prompt_route())
        .with_route(git_worktree_list.arc_into_prompt_route())
        .with_route(git_worktree_lock.arc_into_prompt_route())
        .with_route(git_worktree_unlock.arc_into_prompt_route())
        .with_route(git_worktree_prune.arc_into_prompt_route());
    
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
    
    let create_issue = Arc::new(kodgen_github::CreateIssueTool);
    let get_issue = Arc::new(kodgen_github::GetIssueTool);
    let list_issues = Arc::new(kodgen_github::ListIssuesTool);
    let update_issue = Arc::new(kodgen_github::UpdateIssueTool);
    let search_issues = Arc::new(kodgen_github::SearchIssuesTool);
    let add_issue_comment = Arc::new(kodgen_github::AddIssueCommentTool);
    let get_issue_comments = Arc::new(kodgen_github::GetIssueCommentsTool);
    let create_pr = Arc::new(kodgen_github::CreatePullRequestTool);
    let update_pr = Arc::new(kodgen_github::UpdatePullRequestTool);
    let merge_pr = Arc::new(kodgen_github::MergePullRequestTool);
    let get_pr_status = Arc::new(kodgen_github::GetPullRequestStatusTool);
    let get_pr_files = Arc::new(kodgen_github::GetPullRequestFilesTool);
    let get_pr_reviews = Arc::new(kodgen_github::GetPullRequestReviewsTool);
    let create_pr_review = Arc::new(kodgen_github::CreatePullRequestReviewTool);
    let add_pr_review_comment = Arc::new(kodgen_github::AddPullRequestReviewCommentTool);
    let request_copilot = Arc::new(kodgen_github::RequestCopilotReviewTool);
    
    let tool_router = tool_router
        .with_route(create_issue.clone().arc_into_tool_route())
        .with_route(get_issue.clone().arc_into_tool_route())
        .with_route(list_issues.clone().arc_into_tool_route())
        .with_route(update_issue.clone().arc_into_tool_route())
        .with_route(search_issues.clone().arc_into_tool_route())
        .with_route(add_issue_comment.clone().arc_into_tool_route())
        .with_route(get_issue_comments.clone().arc_into_tool_route())
        .with_route(create_pr.clone().arc_into_tool_route())
        .with_route(update_pr.clone().arc_into_tool_route())
        .with_route(merge_pr.clone().arc_into_tool_route())
        .with_route(get_pr_status.clone().arc_into_tool_route())
        .with_route(get_pr_files.clone().arc_into_tool_route())
        .with_route(get_pr_reviews.clone().arc_into_tool_route())
        .with_route(create_pr_review.clone().arc_into_tool_route())
        .with_route(add_pr_review_comment.clone().arc_into_tool_route())
        .with_route(request_copilot.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(create_issue.arc_into_prompt_route())
        .with_route(get_issue.arc_into_prompt_route())
        .with_route(list_issues.arc_into_prompt_route())
        .with_route(update_issue.arc_into_prompt_route())
        .with_route(search_issues.arc_into_prompt_route())
        .with_route(add_issue_comment.arc_into_prompt_route())
        .with_route(get_issue_comments.arc_into_prompt_route())
        .with_route(create_pr.arc_into_prompt_route())
        .with_route(update_pr.arc_into_prompt_route())
        .with_route(merge_pr.arc_into_prompt_route())
        .with_route(get_pr_status.arc_into_prompt_route())
        .with_route(get_pr_files.arc_into_prompt_route())
        .with_route(get_pr_reviews.arc_into_prompt_route())
        .with_route(create_pr_review.arc_into_prompt_route())
        .with_route(add_pr_review_comment.arc_into_prompt_route())
        .with_route(request_copilot.arc_into_prompt_route());
    
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
    
    let get = Arc::new(kodegen_config::GetConfigTool::new(config_manager.clone()));
    let set = Arc::new(kodegen_config::SetConfigValueTool::new(config_manager.clone()));
    
    let tool_router = tool_router
        .with_route(get.clone().arc_into_tool_route())
        .with_route(set.clone().arc_into_tool_route());
    
    let prompt_router = prompt_router
        .with_route(get.arc_into_prompt_route())
        .with_route(set.arc_into_prompt_route());
    
    Ok((tool_router, prompt_router))
}
