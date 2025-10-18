use anyhow::Result;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    ServiceExt, transport::stdio,
    handler::server::router::{tool::ToolRouter, prompt::PromptRouter},
    model::{
        CallToolRequestParam, CallToolResult, GetPromptRequestParam, GetPromptResult,
        Implementation, InitializeRequestParam, InitializeResult, ListPromptsResult,
        ListResourceTemplatesResult, ListResourcesResult, ListToolsResult,
        PaginatedRequestParam, ProtocolVersion, ReadResourceRequestParam,
        ReadResourceResult, ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
};
use serde_json::json;
use std::sync::Arc;
use std::collections::HashSet;
use std::net::SocketAddr;

mod cli;
mod common;

use cli::{Cli, ServerMode};
use clap::Parser;

// Workspace package imports
use kodegen_tool::Tool;
use kodegen_utils::usage_tracker::UsageTracker;

// Conditional feature imports
#[cfg(feature = "claude_agent")]
use kodegen_claude_agent::{
    AgentManager,
    SpawnClaudeAgentTool,
    ReadClaudeAgentOutputTool,
    SendClaudeAgentPromptTool,
    TerminateClaudeAgentSessionTool,
    ListClaudeAgentsTool,
};

#[cfg(feature = "citescrape")]
use kodegen_citescrape::{
    StartCrawlTool,
    GetCrawlResultsTool,
    SearchCrawlResultsTool,
    WebSearchTool,
    CrawlSessionManager,
    SearchEngineCache,
};

/// MCP Server that serves tools via stdio transport
#[derive(Clone)]
pub struct StdioServer {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    usage_tracker: UsageTracker,
    config_manager: kodegen_config::ConfigManager,
}

impl StdioServer {
    pub fn new(
        tool_router: ToolRouter<Self>,
        prompt_router: PromptRouter<Self>,
        usage_tracker: UsageTracker,
        config_manager: kodegen_config::ConfigManager,
    ) -> Self {
        Self {
            tool_router,
            prompt_router,
            usage_tracker,
            config_manager,
        }
    }
}

impl ServerHandler for StdioServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "KODEGEN MCP Server - filesystem, terminal, process, and Claude agent tools".to_string(),
            ),
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_name = request.name.clone();
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        
        let result = self.tool_router.call(tcc).await;
        
        // Track the call (fire-and-forget, no spawning)
        if result.is_ok() {
            self.usage_tracker.track_success(&tool_name);
        } else {
            self.usage_tracker.track_failure(&tool_name);
        }
        
        result
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let items = self.tool_router.list_all();
        Ok(ListToolsResult::with_all_items(items))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let pcc = rmcp::handler::server::prompt::PromptContext::new(
            self,
            request.name,
            request.arguments,
            context,
        );
        self.prompt_router.get_prompt(pcc).await
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let items = self.prompt_router.list_all();
        Ok(ListPromptsResult::with_all_items(items))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        Err(McpError::resource_not_found(
            "resource_not_found",
            Some(json!({ "uri": uri })),
        ))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }

    async fn initialize(
        &self,
        request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        // Capture and store client information from MCP handshake
        if let Err(e) = self.config_manager.set_client_info(request.client_info).await {
            log::warn!("Failed to store client info: {:?}", e);
            // Don't fail initialization if client tracking fails - graceful degradation
        }
        
        Ok(self.get_info())
    }
}

fn main() -> Result<()> {
    // Pre-initialize citescrape LazyLock statics BEFORE starting tokio runtime
    // This prevents "Cannot block the current thread" panics
    #[cfg(feature = "citescrape")]
    {
        kodegen_citescrape::preinit_lazy_statics();
    }
    
    // Create tokio runtime manually so preinit happens first
    tokio::runtime::Runtime::new()?.block_on(async_main())
}

async fn async_main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Handle --list-categories
    if cli.list_categories {
        println!("Available tool categories:");
        for category in cli::available_categories() {
            println!("  - {}", category);
        }
        return Ok(());
    }
    
    // Handle subcommands
    if let Some(command) = &cli.command {
        match command {
            cli::Commands::Install => {
                return run_install();
            }
        }
    }
    
    // Get server mode from CLI
    let server_mode = cli.server_mode();
    
    // Get enabled categories filter (None = all, Some(set) = filtered)
    let enabled_categories = cli.enabled_categories();
    
    // Initialize logging - adjust level based on mode
    // Can be overridden with RUST_LOG environment variable
    env_logger::Builder::from_default_env()
        .filter_level(match server_mode {
            ServerMode::Stdio => log::LevelFilter::Warn,   // Quiet for stdio
            ServerMode::Sse(_) => log::LevelFilter::Info,   // Verbose for SSE
        })
        .target(env_logger::Target::Stderr)
        .init();

    log::info!("Starting KODEGEN MCP Server in {:?} mode", server_mode);

    // Initialize global tool history
    let _history = kodegen_tool::tool_history::init_global_history().await;

    // Create config manager and usage tracker
    let config_manager = kodegen_config::ConfigManager::new();
    config_manager.init().await?;
    let usage_tracker = UsageTracker::new();

    // Build routers (async)
    let routers = common::build_routers::<StdioServer>(
        &config_manager,
        &usage_tracker,
        &enabled_categories,
    ).await?;

    // Create server
    let server = StdioServer::new(
        routers.tool_router,
        routers.prompt_router,
        usage_tracker,
        config_manager.clone(),
    );

    // Ensure daemon is running for stdio mode
    if matches!(server_mode, ServerMode::Stdio) {
        ensure_daemon_running().await?;
    }

    // Choose transport based on mode
    match server_mode {
        ServerMode::Stdio => {
            serve_stdio(server).await
        }
        ServerMode::Sse(addr) => {
            serve_sse(server, addr).await
        }
    }
}

async fn serve_stdio(server: StdioServer) -> Result<()> {
    log::info!("Serving MCP via stdio transport");
    
    let (_read, _write) = stdio::split();
    Ok(().serve(stdio::stdio_transport()).await?)
}

/// Serve via SSE transport (new, based on counter_sse_directly.rs example)
async fn serve_sse(server: StdioServer, bind_addr: SocketAddr) -> Result<()> {
    use anyhow::Context;
    use rmcp::transport::sse_server::SseServer;
    
    log::info!("Starting SSE server on http://{}", bind_addr);
    log::info!("SSE endpoint: http://{}/sse", bind_addr);
    log::info!("Message endpoint: http://{}/message", bind_addr);
    
    // Create SSE server using the simpler pattern from counter_sse_directly.rs
                Arc::new(kodegen_filesystem::DeleteFileTool::new(config_manager.clone())),
                Arc::new(kodegen_filesystem::DeleteDirectoryTool::new(config_manager.clone())),
                Arc::new(kodegen_filesystem::ListDirectoryTool::new(config_manager.clone())),
                Arc::new(kodegen_filesystem::CreateDirectoryTool::new(config_manager.clone())),
                Arc::new(kodegen_filesystem::GetFileInfoTool::new(config_manager.clone())),
                Arc::new(kodegen_filesystem::EditBlockTool::new(config_manager.clone())),
                Arc::new(kodegen_filesystem::search::StartSearchTool::new(search_manager.clone())),
                Arc::new(kodegen_filesystem::search::GetMoreSearchResultsTool::new(search_manager.clone())),
                Arc::new(kodegen_filesystem::search::StopSearchTool::new(search_manager.clone())),
                Arc::new(kodegen_filesystem::search::ListSearchesTool::new(search_manager)),
            )
        })
    } else {
        log::debug!("Filesystem tools disabled by runtime filter");
        None
    };
    
    // Instantiate process tools (2 tools) - CONDITIONAL
    #[cfg(feature = "process")]
    let process_tools = if is_category_enabled("process", enabled_categories) {
        log::debug!("Initializing process tools");
        Some((
            Arc::new(kodegen_process::ListProcessesTool::new()),
            Arc::new(kodegen_process::KillProcessTool::new()),
        ))
    } else {
        log::debug!("Process tools disabled by runtime filter");
        None
    };
    
    // Instantiate terminal tools (5 tools) - CONDITIONAL
    #[cfg(feature = "terminal")]
    let terminal_tools = if is_category_enabled("terminal", enabled_categories) {
        log::debug!("Initializing terminal tools");
        Some({
            // Create terminal managers
            let terminal_manager = kodegen_terminal::TerminalManager::new();
            let command_manager = kodegen_terminal::CommandManager::new(config_manager.get_blocked_commands());
            
            // Wrap in Arc and start cleanup task
            let terminal_manager_arc = std::sync::Arc::new(terminal_manager.clone());
            terminal_manager_arc.start_cleanup_task();
            
            (
                Arc::new(kodegen_terminal::StartTerminalCommandTool::new(terminal_manager.clone(), command_manager)),
                Arc::new(kodegen_terminal::ReadTerminalOutputTool::new(terminal_manager.clone())),
                Arc::new(kodegen_terminal::SendTerminalInputTool::new(terminal_manager.clone())),
                Arc::new(kodegen_terminal::StopTerminalCommandTool::new(terminal_manager.clone())),
                Arc::new(kodegen_terminal::ListTerminalCommandsTool::new(terminal_manager)),
            )
        })
    } else {
        log::debug!("Terminal tools disabled by runtime filter");
        None
    };
    
    // Instantiate introspection tools (2 tools) - CONDITIONAL
    #[cfg(feature = "introspection")]
    let introspection_tools = if is_category_enabled("introspection", enabled_categories) {
        log::debug!("Initializing introspection tools");
        Some((
            Arc::new(kodegen_introspection::GetUsageStatsTool::new(usage_tracker.clone())),
            Arc::new(kodegen_introspection::GetRecentToolCallsTool::new()),
        ))
    } else {
        log::debug!("Introspection tools disabled by runtime filter");
        None
    };
    
    // Instantiate prompt management tools (4 tools) - CONDITIONAL (async initialization)
    #[cfg(feature = "prompt")]
    let prompt_tools = if is_category_enabled("prompt", enabled_categories) {
        log::debug!("Initializing prompt tools");
        Some((
            Arc::new(kodegen_prompt::AddPromptTool::new().await?),
            Arc::new(kodegen_prompt::EditPromptTool::new().await?),
            Arc::new(kodegen_prompt::DeletePromptTool::new().await?),
            Arc::new(kodegen_prompt::GetPromptTool::new().await?),
        ))
    } else {
        log::debug!("Prompt tools disabled by runtime filter");
        None
    };
    
    // Instantiate sequential thinking tool (1 tool) - CONDITIONAL
    #[cfg(feature = "sequential_thinking")]
    let sequential_thinking = if is_category_enabled("sequential_thinking", enabled_categories) {
        log::debug!("Initializing sequential thinking tool");
        let tool = Arc::new(kodegen_sequential_thinking::SequentialThinkingTool::new());
        tool.clone().start_cleanup_task();
        Some(tool)
    } else {
        log::debug!("Sequential thinking tool disabled by runtime filter");
        None
    };

    // Instantiate claude agent tools (5 tools) - CONDITIONAL
    #[cfg(feature = "claude_agent")]
    let claude_agent_tools = if is_category_enabled("claude_agent", enabled_categories) {
        log::debug!("Initializing claude agent tools");
        Some({
            // Agent manager for spawning sub-agents
            let agent_manager = Arc::new(AgentManager::new());

            // Prompt manager for template rendering
            let kodegen_prompt_manager = Arc::new(kodegen_prompt::PromptManager::new());
            kodegen_prompt_manager.init().await.map_err(|e| anyhow::anyhow!("Failed to init prompt manager: {}", e))?;

            (
                Arc::new(SpawnClaudeAgentTool::new(agent_manager.clone(), kodegen_prompt_manager.clone())),
                Arc::new(ReadClaudeAgentOutputTool::new(agent_manager.clone())),
                Arc::new(SendClaudeAgentPromptTool::new(agent_manager.clone(), kodegen_prompt_manager.clone())),
                Arc::new(TerminateClaudeAgentSessionTool::new(agent_manager.clone())),
                Arc::new(ListClaudeAgentsTool::new(agent_manager)),
            )
        })
    } else {
        log::debug!("Claude agent tools disabled by runtime filter");
        None
    };

    // Instantiate citescrape tools (4 tools) - CONDITIONAL
    #[cfg(feature = "citescrape")]
    let citescrape_tools = if is_category_enabled("citescrape", enabled_categories) {
        log::debug!("Initializing citescrape tools");
        
        // CRITICAL: Pre-initialize all LazyLock statics BEFORE starting cleanup tasks
        // LazyLock can block during first access, which causes panic in tokio runtime
        kodegen_citescrape::preinit_lazy_statics();
        
        Some({
            // Create shared managers
            let session_manager = CrawlSessionManager::new();
            let engine_cache = SearchEngineCache::new();

            // Wrap in Arc and start background cleanup tasks to prevent memory leaks
            let session_manager_arc = std::sync::Arc::new(session_manager.clone());
            let engine_cache_arc = std::sync::Arc::new(engine_cache.clone());

            session_manager_arc.start_cleanup_task();
            engine_cache_arc.start_cleanup_task();

            (
                Arc::new(StartCrawlTool::new(
                    session_manager.clone(),
                    engine_cache.clone(),
                )),
                Arc::new(GetCrawlResultsTool::new(session_manager.clone())),
                Arc::new(SearchCrawlResultsTool::new(
                    session_manager.clone(),
                    engine_cache.clone(),
                )),
                Arc::new(WebSearchTool::new()),
            )
        })
    } else {
        log::debug!("Citescrape tools disabled by runtime filter");
        None
    };

    // Instantiate git tools (20 tools) - CONDITIONAL
    #[cfg(feature = "git")]
    let git_tools = if is_category_enabled("git", enabled_categories) {
        log::debug!("Initializing git tools");
        Some((
            Arc::new(kodgen_git::GitInitTool),
            Arc::new(kodgen_git::GitOpenTool),
            Arc::new(kodgen_git::GitCloneTool),
            Arc::new(kodgen_git::GitDiscoverTool),
            Arc::new(kodgen_git::GitBranchCreateTool),
            Arc::new(kodgen_git::GitBranchDeleteTool),
            Arc::new(kodgen_git::GitBranchListTool),
            Arc::new(kodgen_git::GitBranchRenameTool),
            Arc::new(kodgen_git::GitCommitTool),
            Arc::new(kodgen_git::GitLogTool),
            Arc::new(kodgen_git::GitAddTool),
            Arc::new(kodgen_git::GitCheckoutTool),
            Arc::new(kodgen_git::GitFetchTool),
            Arc::new(kodgen_git::GitMergeTool),
            Arc::new(kodgen_git::GitWorktreeAddTool),
            Arc::new(kodgen_git::GitWorktreeRemoveTool),
            Arc::new(kodgen_git::GitWorktreeListTool),
            Arc::new(kodgen_git::GitWorktreeLockTool),
            Arc::new(kodgen_git::GitWorktreeUnlockTool),
            Arc::new(kodgen_git::GitWorktreePruneTool),
        ))
    } else {
        log::debug!("Git tools disabled by runtime filter");
        None
    };

    // Instantiate github tools (16 tools) - CONDITIONAL
    #[cfg(feature = "github")]
    let github_tools = if is_category_enabled("github", enabled_categories) {
        log::debug!("Initializing github tools");
        Some((
            // Issue tools (7)
            Arc::new(kodgen_github::CreateIssueTool),
            Arc::new(kodgen_github::GetIssueTool),
            Arc::new(kodgen_github::ListIssuesTool),
            Arc::new(kodgen_github::UpdateIssueTool),
            Arc::new(kodgen_github::SearchIssuesTool),
            Arc::new(kodgen_github::AddIssueCommentTool),
            Arc::new(kodgen_github::GetIssueCommentsTool),
            // PR tools (5)
            Arc::new(kodgen_github::CreatePullRequestTool),
            Arc::new(kodgen_github::UpdatePullRequestTool),
            Arc::new(kodgen_github::MergePullRequestTool),
            Arc::new(kodgen_github::GetPullRequestStatusTool),
            Arc::new(kodgen_github::GetPullRequestFilesTool),
            // PR Review tools (4)
            Arc::new(kodgen_github::GetPullRequestReviewsTool),
            Arc::new(kodgen_github::CreatePullRequestReviewTool),
            Arc::new(kodgen_github::AddPullRequestReviewCommentTool),
            Arc::new(kodgen_github::RequestCopilotReviewTool),
        ))
    } else {
        log::debug!("GitHub tools disabled by runtime filter");
        None
    };

    // Instantiate config tools (2 tools) - CONDITIONAL
    #[cfg(feature = "config")]
    let config_tools = if is_category_enabled("config", enabled_categories) {
        log::debug!("Initializing config tools");
        Some((
            Arc::new(kodegen_config::GetConfigTool::new(config_manager.clone())),
            Arc::new(kodegen_config::SetConfigValueTool::new(config_manager.clone())),
        ))
    } else {
        log::debug!("Config tools disabled by runtime filter");
        None
    };

    // Build tool router with conditional routes
    let mut tool_router = ToolRouter::new();
    
    // Register filesystem tools (14 tools)
    #[cfg(feature = "filesystem")]
    if let Some((
        read_file, read_multiple_files, write_file, move_file,
        delete_file, delete_directory, list_directory, create_directory,
        get_file_info, edit_block, start_search, get_more_results,
        stop_search, list_searches
    )) = &filesystem_tools {
        tool_router = tool_router
            .with_route(Arc::clone(read_file).arc_into_tool_route())
            .with_route(Arc::clone(read_multiple_files).arc_into_tool_route())
            .with_route(Arc::clone(write_file).arc_into_tool_route())
            .with_route(Arc::clone(move_file).arc_into_tool_route())
            .with_route(Arc::clone(delete_file).arc_into_tool_route())
            .with_route(Arc::clone(delete_directory).arc_into_tool_route())
            .with_route(Arc::clone(list_directory).arc_into_tool_route())
            .with_route(Arc::clone(create_directory).arc_into_tool_route())
            .with_route(Arc::clone(get_file_info).arc_into_tool_route())
            .with_route(Arc::clone(edit_block).arc_into_tool_route())
            .with_route(Arc::clone(start_search).arc_into_tool_route())
            .with_route(Arc::clone(get_more_results).arc_into_tool_route())
            .with_route(Arc::clone(stop_search).arc_into_tool_route())
            .with_route(Arc::clone(list_searches).arc_into_tool_route());
    }
    
    // Register terminal tools (5 tools)
    #[cfg(feature = "terminal")]
    if let Some((
        start_terminal_command, read_terminal_output, send_terminal_input,
        stop_terminal_command, list_terminal_commands
    )) = &terminal_tools {
        tool_router = tool_router
            .with_route(Arc::clone(start_terminal_command).arc_into_tool_route())
            .with_route(Arc::clone(read_terminal_output).arc_into_tool_route())
            .with_route(Arc::clone(send_terminal_input).arc_into_tool_route())
            .with_route(Arc::clone(stop_terminal_command).arc_into_tool_route())
            .with_route(Arc::clone(list_terminal_commands).arc_into_tool_route());
    }
    
    // Register process tools (2 tools)
    #[cfg(feature = "process")]
    if let Some((list_processes, kill_process)) = &process_tools {
        tool_router = tool_router
            .with_route(Arc::clone(list_processes).arc_into_tool_route())
            .with_route(Arc::clone(kill_process).arc_into_tool_route());
    }
    
    // Register introspection tools (2 tools)
    #[cfg(feature = "introspection")]
    if let Some((get_usage_stats, get_recent_tool_calls)) = &introspection_tools {
        tool_router = tool_router
            .with_route(Arc::clone(get_usage_stats).arc_into_tool_route())
            .with_route(Arc::clone(get_recent_tool_calls).arc_into_tool_route());
    }
    
    // Register prompt tools (4 tools)
    #[cfg(feature = "prompt")]
    if let Some((add_prompt, edit_prompt, delete_prompt, get_prompt)) = &prompt_tools {
        tool_router = tool_router
            .with_route(Arc::clone(add_prompt).arc_into_tool_route())
            .with_route(Arc::clone(edit_prompt).arc_into_tool_route())
            .with_route(Arc::clone(delete_prompt).arc_into_tool_route())
            .with_route(Arc::clone(get_prompt).arc_into_tool_route());
    }
    
    // Register sequential thinking tool (1 tool)
    #[cfg(feature = "sequential_thinking")]
    if let Some(sequential_thinking_tool) = &sequential_thinking {
        tool_router = tool_router
            .with_route(Arc::clone(sequential_thinking_tool).arc_into_tool_route());
    }
    
    // Register claude agent tools (5 tools)
    #[cfg(feature = "claude_agent")]
    if let Some((
        spawn_claude_agent, read_claude_agent_output, send_claude_agent_prompt,
        terminate_claude_agent_session, list_claude_agents
    )) = &claude_agent_tools {
        tool_router = tool_router
            .with_route(Arc::clone(spawn_claude_agent).arc_into_tool_route())
            .with_route(Arc::clone(read_claude_agent_output).arc_into_tool_route())
            .with_route(Arc::clone(send_claude_agent_prompt).arc_into_tool_route())
            .with_route(Arc::clone(terminate_claude_agent_session).arc_into_tool_route())
            .with_route(Arc::clone(list_claude_agents).arc_into_tool_route());
    }
    
    // Register citescrape tools (4 tools)
    #[cfg(feature = "citescrape")]
    if let Some((
        start_crawl, get_crawl_results, search_crawl_results, web_search
    )) = &citescrape_tools {
        tool_router = tool_router
            .with_route(Arc::clone(start_crawl).arc_into_tool_route())
            .with_route(Arc::clone(get_crawl_results).arc_into_tool_route())
            .with_route(Arc::clone(search_crawl_results).arc_into_tool_route())
            .with_route(Arc::clone(web_search).arc_into_tool_route());
    }
    
    // Register git tools (20 tools)
    #[cfg(feature = "git")]
    if let Some((
        git_init, git_open, git_clone, git_discover,
        git_branch_create, git_branch_delete, git_branch_list, git_branch_rename,
        git_commit, git_log, git_add, git_checkout,
        git_fetch, git_merge,
        git_worktree_add, git_worktree_remove, git_worktree_list,
        git_worktree_lock, git_worktree_unlock, git_worktree_prune
    )) = &git_tools {
        tool_router = tool_router
            .with_route(Arc::clone(git_init).arc_into_tool_route())
            .with_route(Arc::clone(git_open).arc_into_tool_route())
            .with_route(Arc::clone(git_clone).arc_into_tool_route())
            .with_route(Arc::clone(git_discover).arc_into_tool_route())
            .with_route(Arc::clone(git_branch_create).arc_into_tool_route())
            .with_route(Arc::clone(git_branch_delete).arc_into_tool_route())
            .with_route(Arc::clone(git_branch_list).arc_into_tool_route())
            .with_route(Arc::clone(git_branch_rename).arc_into_tool_route())
            .with_route(Arc::clone(git_commit).arc_into_tool_route())
            .with_route(Arc::clone(git_log).arc_into_tool_route())
            .with_route(Arc::clone(git_add).arc_into_tool_route())
            .with_route(Arc::clone(git_checkout).arc_into_tool_route())
            .with_route(Arc::clone(git_fetch).arc_into_tool_route())
            .with_route(Arc::clone(git_merge).arc_into_tool_route())
            .with_route(Arc::clone(git_worktree_add).arc_into_tool_route())
            .with_route(Arc::clone(git_worktree_remove).arc_into_tool_route())
            .with_route(Arc::clone(git_worktree_list).arc_into_tool_route())
            .with_route(Arc::clone(git_worktree_lock).arc_into_tool_route())
            .with_route(Arc::clone(git_worktree_unlock).arc_into_tool_route())
            .with_route(Arc::clone(git_worktree_prune).arc_into_tool_route());
    }
    
    // Register github tools (16 tools)
    #[cfg(feature = "github")]
    if let Some((
        create_issue, get_issue, list_issues, update_issue,
        search_issues, add_issue_comment, get_issue_comments,
        create_pr, update_pr, merge_pr, get_pr_status, get_pr_files,
        get_pr_reviews, create_pr_review, add_pr_review_comment, request_copilot
    )) = &github_tools {
        tool_router = tool_router
            .with_route(Arc::clone(create_issue).arc_into_tool_route())
            .with_route(Arc::clone(get_issue).arc_into_tool_route())
            .with_route(Arc::clone(list_issues).arc_into_tool_route())
            .with_route(Arc::clone(update_issue).arc_into_tool_route())
            .with_route(Arc::clone(search_issues).arc_into_tool_route())
            .with_route(Arc::clone(add_issue_comment).arc_into_tool_route())
            .with_route(Arc::clone(get_issue_comments).arc_into_tool_route())
            .with_route(Arc::clone(create_pr).arc_into_tool_route())
            .with_route(Arc::clone(update_pr).arc_into_tool_route())
            .with_route(Arc::clone(merge_pr).arc_into_tool_route())
            .with_route(Arc::clone(get_pr_status).arc_into_tool_route())
            .with_route(Arc::clone(get_pr_files).arc_into_tool_route())
            .with_route(Arc::clone(get_pr_reviews).arc_into_tool_route())
            .with_route(Arc::clone(create_pr_review).arc_into_tool_route())
            .with_route(Arc::clone(add_pr_review_comment).arc_into_tool_route())
            .with_route(Arc::clone(request_copilot).arc_into_tool_route());
    }
    
    // Register config tools (2 tools)
    #[cfg(feature = "config")]
    if let Some((get_config, set_config_value)) = &config_tools {
        tool_router = tool_router
            .with_route(Arc::clone(get_config).arc_into_tool_route())
            .with_route(Arc::clone(set_config_value).arc_into_tool_route());
    }
    
    // Build prompt router with conditional routes (moves ownership)
    let mut prompt_router = PromptRouter::new();
    
    // Register filesystem tools (14 tools)
    #[cfg(feature = "filesystem")]
    if let Some((
        read_file, read_multiple_files, write_file, move_file,
        delete_file, delete_directory, list_directory, create_directory,
        get_file_info, edit_block, start_search, get_more_results,
        stop_search, list_searches
    )) = filesystem_tools {
        prompt_router = prompt_router
            .with_route(read_file.arc_into_prompt_route())
            .with_route(read_multiple_files.arc_into_prompt_route())
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
    }
    
    // Register terminal tools (5 tools)
    #[cfg(feature = "terminal")]
    if let Some((
        start_terminal_command, read_terminal_output, send_terminal_input,
        stop_terminal_command, list_terminal_commands
    )) = terminal_tools {
        prompt_router = prompt_router
            .with_route(start_terminal_command.arc_into_prompt_route())
            .with_route(read_terminal_output.arc_into_prompt_route())
            .with_route(send_terminal_input.arc_into_prompt_route())
            .with_route(stop_terminal_command.arc_into_prompt_route())
            .with_route(list_terminal_commands.arc_into_prompt_route());
    }
    
    // Register process tools (2 tools)
    #[cfg(feature = "process")]
    if let Some((list_processes, kill_process)) = process_tools {
        prompt_router = prompt_router
            .with_route(list_processes.arc_into_prompt_route())
            .with_route(kill_process.arc_into_prompt_route());
    }
    
    // Register introspection tools (2 tools)
    #[cfg(feature = "introspection")]
    if let Some((get_usage_stats, get_recent_tool_calls)) = introspection_tools {
        prompt_router = prompt_router
            .with_route(get_usage_stats.arc_into_prompt_route())
            .with_route(get_recent_tool_calls.arc_into_prompt_route());
    }
    
    // Register prompt tools (4 tools)
    #[cfg(feature = "prompt")]
    if let Some((add_prompt, edit_prompt, delete_prompt, get_prompt)) = prompt_tools {
        prompt_router = prompt_router
            .with_route(add_prompt.arc_into_prompt_route())
            .with_route(edit_prompt.arc_into_prompt_route())
            .with_route(delete_prompt.arc_into_prompt_route())
            .with_route(get_prompt.arc_into_prompt_route());
    }
    
    // Register sequential thinking tool (1 tool)
    #[cfg(feature = "sequential_thinking")]
    if let Some(sequential_thinking_tool) = sequential_thinking {
        prompt_router = prompt_router
            .with_route(sequential_thinking_tool.arc_into_prompt_route());
    }
    
    // Register claude agent tools (5 tools)
    #[cfg(feature = "claude_agent")]
    if let Some((
        spawn_claude_agent, read_claude_agent_output, send_claude_agent_prompt,
        terminate_claude_agent_session, list_claude_agents
    )) = claude_agent_tools {
        prompt_router = prompt_router
            .with_route(spawn_claude_agent.arc_into_prompt_route())
            .with_route(read_claude_agent_output.arc_into_prompt_route())
            .with_route(send_claude_agent_prompt.arc_into_prompt_route())
            .with_route(terminate_claude_agent_session.arc_into_prompt_route())
            .with_route(list_claude_agents.arc_into_prompt_route());
    }
    
    // Register citescrape tools (4 tools)
    #[cfg(feature = "citescrape")]
    if let Some((
        start_crawl, get_crawl_results, search_crawl_results, web_search
    )) = citescrape_tools {
        prompt_router = prompt_router
            .with_route(start_crawl.arc_into_prompt_route())
            .with_route(get_crawl_results.arc_into_prompt_route())
            .with_route(search_crawl_results.arc_into_prompt_route())
            .with_route(web_search.arc_into_prompt_route());
    }
    
    // Register git tools (20 tools)
    #[cfg(feature = "git")]
    if let Some((
        git_init, git_open, git_clone, git_discover,
        git_branch_create, git_branch_delete, git_branch_list, git_branch_rename,
        git_commit, git_log, git_add, git_checkout,
        git_fetch, git_merge,
        git_worktree_add, git_worktree_remove, git_worktree_list,
        git_worktree_lock, git_worktree_unlock, git_worktree_prune
    )) = git_tools {
        prompt_router = prompt_router
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
    }
    
    // Register github tools (16 tools)
    #[cfg(feature = "github")]
    if let Some((
        create_issue, get_issue, list_issues, update_issue,
        search_issues, add_issue_comment, get_issue_comments,
        create_pr, update_pr, merge_pr, get_pr_status, get_pr_files,
        get_pr_reviews, create_pr_review, add_pr_review_comment, request_copilot
    )) = github_tools {
        prompt_router = prompt_router
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
    }
    
    // Register config tools (2 tools)
    #[cfg(feature = "config")]
    if let Some((get_config, set_config_value)) = config_tools {
        prompt_router = prompt_router
            .with_route(get_config.arc_into_prompt_route())
            .with_route(set_config_value.arc_into_prompt_route());
    }

    Ok((tool_router, prompt_router))
}

fn main() -> Result<()> {
    // Pre-initialize citescrape LazyLock statics BEFORE starting tokio runtime
    // This prevents "Cannot block the current thread" panics
    #[cfg(feature = "citescrape")]
    {
        kodegen_citescrape::preinit_lazy_statics();
    }
    
    // Create tokio runtime manually so preinit happens first
    tokio::runtime::Runtime::new()?.block_on(async_main())
}

async fn async_main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Handle --list-categories
    if cli.list_categories {
        println!("Available tool categories:");
        for category in cli::available_categories() {
            println!("  - {}", category);
        }
        return Ok(());
    }
    
    // Handle subcommands
    if let Some(command) = &cli.command {
        match command {
            cli::Commands::Install => {
                return run_install();
            }
        }
    }
    
    // Get server mode from CLI
    let server_mode = cli.server_mode();
    
    // Get enabled categories filter (None = all, Some(set) = filtered)
    let enabled_categories = cli.enabled_categories();
    
    // Initialize logging - adjust level based on mode
    // Can be overridden with RUST_LOG environment variable
    env_logger::Builder::from_default_env()
        .filter_level(match server_mode {
            ServerMode::Stdio => log::LevelFilter::Warn,   // Quiet for stdio
            ServerMode::Sse(_) => log::LevelFilter::Info,   // Verbose for SSE
        })
        .target(env_logger::Target::Stderr)
        .init();

    log::info!("Starting KODEGEN MCP Server in {:?} mode", server_mode);

    // Initialize global tool history
    let _history = kodegen_tool::tool_history::init_global_history().await;

    // Create config manager and usage tracker
    let config_manager = kodegen_config::ConfigManager::new();
    config_manager.init().await?;
    let usage_tracker = UsageTracker::new();

    // Build routers (async)
    let routers = common::build_routers::<StdioServer>(
        &config_manager,
        &usage_tracker,
        &enabled_categories,
    ).await?;

    // Create server
    let server = StdioServer::new(
        routers.tool_router,
        routers.prompt_router,
        usage_tracker,
        config_manager.clone(),
    );

    // Ensure daemon is running for stdio mode
    if matches!(server_mode, ServerMode::Stdio) {
        ensure_daemon_running().await?;
    }

    // Choose transport based on mode
    match server_mode {
        ServerMode::Stdio => {
            serve_stdio(server).await
        }
        ServerMode::Sse(bind_addr) => {
            serve_sse(server, bind_addr).await
        }
    }
}

/// Serve via stdio transport (extracted from existing code)
async fn serve_stdio(server: StdioServer) -> Result<()> {
    let service = server.serve(stdio()).await.inspect_err(|e| {
        log::error!("serving error: {:?}", e);
    })?;
    service.waiting().await?;
    Ok(())
}

/// Serve via SSE transport (new, based on counter_sse_directly.rs example)
async fn serve_sse(server: StdioServer, bind_addr: SocketAddr) -> Result<()> {
    use anyhow::Context;
    use rmcp::transport::sse_server::SseServer;
    
    log::info!("Starting SSE server on http://{}", bind_addr);
    log::info!("SSE endpoint: http://{}/sse", bind_addr);
    log::info!("Message endpoint: http://{}/message", bind_addr);
    
    // Create SSE server using the simpler pattern from counter_sse_directly.rs
    // Wrap server in a closure that returns clones for each connection
    let ct = SseServer::serve(bind_addr)
        .await
        .context("Failed to start SSE server")?
        .with_service_directly(move || server.clone());
    
    log::info!("SSE server running, press Ctrl+C to stop");
    
    // Wait for Ctrl+C
    tokio::signal::ctrl_c()
        .await
        .context("Failed to listen for Ctrl+C")?;
    
    log::info!("Shutting down SSE server...");
    ct.cancel();
    
    Ok(())
}

/// Run the install command
fn run_install() -> Result<()> {
    use kodegen_client_autoconfig::install_all_clients;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
    use std::io::Write;

    let results = install_all_clients()?;

    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // Header
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "\n┌─────────────────────────────────────────────┐")?;
    writeln!(&mut stdout, "│   🔍 MCP Editor Configuration Results       │")?;
    writeln!(&mut stdout, "└─────────────────────────────────────────────┘\n")?;
    stdout.reset()?;

    let mut configured = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for result in &results {
        if result.success {
            // Success - green checkmark
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
            write!(&mut stdout, "  ✓ ")?;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(&mut stdout, "{}", result.client_name)?;
        } else {
            // Failed - red X or dim skip
            if result.message == "Not installed" {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
                write!(&mut stdout, "  ○ ")?;
                writeln!(&mut stdout, "{}", result.client_name)?;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                write!(&mut stdout, "  ✗ ")?;
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(&mut stdout, "{}", result.client_name)?;
            }
        }

        // Config path
        if let Some(ref path) = result.config_path {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
            writeln!(&mut stdout, "     {}", path.display())?;
        }

        // Status message
        if result.success {
            if result.message.contains("Already") {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                writeln!(&mut stdout, "     {}\n", result.message)?;
                skipped += 1;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                writeln!(&mut stdout, "     {}\n", result.message)?;
                configured += 1;
            }
        } else {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
            writeln!(&mut stdout, "     {}\n", result.message)?;
            failed += 1;
        }
        stdout.reset()?;
    }

    // Summary
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "─────────────────────────────────────────────")?;
    stdout.reset()?;

    write!(&mut stdout, "  ")?;
    if configured > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        write!(&mut stdout, "{} configured", configured)?;
        stdout.reset()?;
    }
    if skipped > 0 {
        if configured > 0 { write!(&mut stdout, " • ")?; }
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        write!(&mut stdout, "{} already configured", skipped)?;
        stdout.reset()?;
    }
    if failed > 0 {
        if configured > 0 || skipped > 0 { write!(&mut stdout, " • ")?; }
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
        write!(&mut stdout, "{} not installed", failed)?;
        stdout.reset()?;
    }
    writeln!(&mut stdout, "\n")?;

    Ok(())
}

/// Ensure the kodegend daemon is running before starting stdio mode
///
/// Checks daemon status and starts it if not running, then waits for ready
async fn ensure_daemon_running() -> Result<()> {
    use tokio::process::Command;
    use tokio::time::{sleep, Duration};

    // Check if daemon is already running
    let status = Command::new("kodegend")
        .arg("status")
        .status()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to check daemon status: {}", e))?;

    if status.success() {
        log::info!("kodegend daemon is already running");
        return Ok(());
    }

    // Daemon not running, attempt to start it
    log::info!("kodegend daemon not running, starting...");
    let start = Command::new("kodegend")
        .arg("start")
        .status()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start daemon: {}", e))?;

    if !start.success() {
        anyhow::bail!("Failed to start kodegend daemon");
    }

    // Wait for daemon to be ready (poll with backoff)
    for attempt in 1..=10 {
        sleep(Duration::from_millis(500)).await;

        let check = Command::new("kodegend")
            .arg("status")
            .status()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to check daemon status: {}", e))?;

        if check.success() {
            log::info!("kodegend daemon started successfully after {} attempts", attempt);
            return Ok(());
        }

        if attempt == 10 {
            anyhow::bail!("Daemon failed to start after 10 attempts");
        }
    }

    Ok(())
}
