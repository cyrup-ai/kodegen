// packages/server/src/common/tool_registry.rs
use anyhow::Result;
use rmcp::handler::server::router::{tool::ToolRouter, prompt::PromptRouter};
#[cfg(any(feature = "filesystem", feature = "terminal", feature = "sequential_thinking", feature = "claude_agent", feature = "citescrape", feature = "database"))]
use std::sync::Arc;
use std::collections::HashSet;
use kodegen_utils::usage_tracker::UsageTracker;
use kodegen_mcp_tool::Tool;

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
#[cfg(feature = "sequential_thinking")]
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

/// Warm up connection pool by pre-establishing min_connections
async fn warmup_pool(pool: &sqlx::AnyPool, min_connections: u32) -> Result<()> {
    use std::time::{Duration, Instant};
    
    let start = Instant::now();
    
    // Acquire min_connections concurrently to force establishment
    let mut handles = Vec::new();
    for i in 0..min_connections {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            sqlx::query("SELECT 1")
                .fetch_one(&pool_clone)
                .await
                .map_err(|e| anyhow::anyhow!("Warmup connection {} failed: {}", i + 1, e))
        });
        handles.push(handle);
    }
    
    // Wait for all warmup queries to complete
    let mut success_count = 0;
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => log::warn!("Connection {} warmup failed: {}", i + 1, e),
            Err(e) => log::warn!("Connection {} warmup task panicked: {}", i + 1, e),
        }
    }
    
    let elapsed = start.elapsed();
    
    if success_count > 0 {
        log::info!(
            "✓ Connection pool warmed up: {}/{} connections ready ({:?})", 
            success_count, min_connections, elapsed
        );
        
        if elapsed > Duration::from_secs(2) {
            log::warn!(
                "Pool warmup was slow ({:?}), queries may have experienced high latency", 
                elapsed
            );
        }
        
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Pool warmup failed: 0/{} connections established", 
            min_connections
        ))
    }
}

/// Register all available tools with the routers
pub async fn register_all_tools<S>(
    mut tool_router: ToolRouter<S>,
    mut prompt_router: PromptRouter<S>,
    config_manager: &kodegen_tools_config::ConfigManager,
    _usage_tracker: &UsageTracker,
    enabled_categories: &Option<HashSet<String>>,
    _database_dsn: Option<&str>,
    #[cfg(feature = "database")]
    _ssh_config: Option<(kodegen_tools_database::SSHConfig, kodegen_tools_database::TunnelConfig)>,
    #[cfg(not(feature = "database"))]
    _ssh_config: Option<()>,
) -> Result<(ToolRouter<S>, PromptRouter<S>, crate::common::router_builder::Managers)>
where
    S: Send + Sync + 'static
{
    // Initialize managers (will be populated as tools are registered)
    #[allow(unused_mut)]
    let mut managers = crate::common::router_builder::Managers {
        #[cfg(feature = "citescrape")]
        browser_manager: None,
        #[cfg(feature = "database")]
        tunnel_guard: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
    };

    // Initialize database connection if DSN provided
    #[cfg(feature = "database")]
    let mut database_pool: Option<(Arc<sqlx::AnyPool>, String)> = None;
    
    #[cfg(feature = "database")]
    if let Some(dsn) = _database_dsn {
        use kodegen_tools_database::{establish_tunnel, rewrite_dsn_for_tunnel, ExposeSecret, SecretString};
        use anyhow::Context;
        use sqlx::pool::PoolOptions;
        use std::time::Duration;
        
        let final_dsn = if let Some((ssh_cfg, tunnel_cfg)) = _ssh_config {
            // Establish tunnel
            let tunnel = establish_tunnel(ssh_cfg, tunnel_cfg).await?;
            let tunneled_dsn = rewrite_dsn_for_tunnel(dsn, tunnel.local_port())?;
            *managers.tunnel_guard.lock().await = Some(tunnel);
            log::info!("✓ SSH tunnel established for database connection");
            tunneled_dsn
        } else {
            SecretString::from(dsn.to_string())
        };
        
        // Extract min_connections BEFORE pool block for warmup access
        let min_connections = config_manager
            .get_value("db_min_connections")
            .and_then(|v| match v {
                kodegen_tools_config::ConfigValue::Number(n) => Some(n as u32),
                _ => None,
            })
            .unwrap_or(2); // 2 connections default for responsiveness
        
        // Install database drivers for sqlx::any
        // This MUST be called before creating AnyPool or AnyConnection
        // It registers the compiled-in drivers (postgres, mysql, sqlite) based on cargo features
        sqlx::any::install_default_drivers();
        
        // Connect to database with timeout configuration
        let pool = {
            // Get timeout configuration from ConfigManager
            let acquire_timeout = config_manager
                .get_value("db_acquire_timeout_secs")
                .and_then(|v| match v {
                    kodegen_tools_config::ConfigValue::Number(n) => Some(Duration::from_secs(n as u64)),
                    _ => None,
                })
                .unwrap_or(Duration::from_secs(30)); // 30s default
            
            let idle_timeout = config_manager
                .get_value("db_idle_timeout_secs")
                .and_then(|v| match v {
                    kodegen_tools_config::ConfigValue::Number(n) => Some(Duration::from_secs(n as u64)),
                    _ => None,
                })
                .unwrap_or(Duration::from_secs(600)); // 10 minutes default
            
            let max_lifetime = config_manager
                .get_value("db_max_lifetime_secs")
                .and_then(|v| match v {
                    kodegen_tools_config::ConfigValue::Number(n) => Some(Duration::from_secs(n as u64)),
                    _ => None,
                })
                .unwrap_or(Duration::from_secs(1800)); // 30 minutes default
            
            let max_connections = config_manager
                .get_value("db_max_connections")
                .and_then(|v| match v {
                    kodegen_tools_config::ConfigValue::Number(n) => Some(n as u32),
                    _ => None,
                })
                .unwrap_or(10); // 10 connections default
            
            // Build pool with PoolOptions
            PoolOptions::new()
                .max_connections(max_connections)
                .min_connections(min_connections)
                .acquire_timeout(acquire_timeout)
                .idle_timeout(Some(idle_timeout))
                .max_lifetime(Some(max_lifetime))
                .test_before_acquire(true) // Verify connection health
                .after_connect(|conn, _meta| Box::pin(async move {
                    // Simple ping to verify connection liveness
                    // This runs on NEW connections (test_before_acquire handles reused ones)
                    sqlx::query("SELECT 1")
                        .fetch_one(conn)
                        .await?;
                    
                    // Optional: Set application name for easier monitoring
                    // Database-specific examples (commented out by default):
                    // PostgreSQL: conn.execute("SET application_name = 'kodegen'").await?;
                    // MySQL: conn.execute("SET @@session.time_zone = '+00:00'").await?;
                    
                    Ok(())
                }))
                .connect(final_dsn.expose_secret())
                .await
                .context("Failed to connect to database")?
        };
        
        // Warmup: Force synchronous connection establishment
        warmup_pool(&pool, min_connections).await?;
        
        log::info!("✓ Database connected ({})", 
            kodegen_tools_database::detect_database_type(final_dsn.expose_secret())?);
        
        database_pool = Some((Arc::new(pool), final_dsn.expose_secret().to_string()));
    }

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
        (tool_router, prompt_router) = register_introspection_tools(tool_router, prompt_router, _usage_tracker).await?;
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
    
    // Reasoner tools
    #[cfg(feature = "reasoner")]
    if is_category_enabled("reasoner", enabled_categories) {
        (tool_router, prompt_router) = register_reasoner_tools(tool_router, prompt_router).await?;
    }
    
    // Claude agent tools
    #[cfg(feature = "claude_agent")]
    if is_category_enabled("claude_agent", enabled_categories) {
        (tool_router, prompt_router) = register_claude_agent_tools(tool_router, prompt_router).await?;
    }
    
    // Citescrape tools
    #[cfg(feature = "citescrape")]
    if is_category_enabled("citescrape", enabled_categories) {
        let browser_manager;
        (tool_router, prompt_router, browser_manager) = register_citescrape_tools(tool_router, prompt_router).await?;
        managers.browser_manager = Some(browser_manager);
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
    
    // Database tools
    #[cfg(feature = "database")]
    if is_category_enabled("database", enabled_categories) {
        if let Some((pool, connection_url)) = database_pool {
            (tool_router, prompt_router) = register_database_tools(
                tool_router,
                prompt_router,
                pool,
                &connection_url,
                config_manager,
            ).await?;
        } else {
            log::warn!("Database tools enabled but no database connection provided");
        }
    }
    
    // Browser tools
    #[cfg(feature = "browser")]
    if is_category_enabled("browser", enabled_categories) {
        (tool_router, prompt_router) = register_browser_tools(tool_router, prompt_router).await?;
    }
    
    // Reasoner tools
    #[cfg(feature = "reasoner")]
    if is_category_enabled("reasoner", enabled_categories) {
        (tool_router, prompt_router) = register_reasoner_tools(tool_router, prompt_router).await?;
    }
    
    Ok((tool_router, prompt_router, managers))
}

#[cfg(feature = "filesystem")]
async fn register_filesystem_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    config_manager: &kodegen_tools_config::ConfigManager,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing filesystem tools");
    
    let search_manager = Arc::new(kodegen_tools_filesystem::search::SearchManager::new(config_manager.clone()));
    
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_filesystem::ReadFileTool::new(
            config_manager.get_file_read_line_limit(),
            config_manager.clone()
        )
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_filesystem::ReadMultipleFilesTool::new(
            config_manager.get_file_read_line_limit(),
            config_manager.clone()
        )
    );
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::WriteFileTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::MoveFileTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::DeleteFileTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::DeleteDirectoryTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::ListDirectoryTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::CreateDirectoryTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::GetFileInfoTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::EditBlockTool::new(config_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::search::StartSearchTool::new(search_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::search::GetMoreSearchResultsTool::new(search_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::search::StopSearchTool::new(search_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_filesystem::search::ListSearchesTool::new(search_manager.clone()));
    
    // Start cleanup task after all tools are registered to avoid race conditions
    search_manager.start_cleanup_task();
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "terminal")]
async fn register_terminal_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    config_manager: &kodegen_tools_config::ConfigManager,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing terminal tools");
    
    let terminal_manager = Arc::new(kodegen_tools_terminal::TerminalManager::new());
    let command_manager = kodegen_tools_terminal::CommandManager::new(config_manager.get_blocked_commands());
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_terminal::StartTerminalCommandTool::new(terminal_manager.clone(), command_manager));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_terminal::ReadTerminalOutputTool::new(terminal_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_terminal::SendTerminalInputTool::new(terminal_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_terminal::StopTerminalCommandTool::new(terminal_manager.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_terminal::ListTerminalCommandsTool::new(terminal_manager.clone()));
    
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
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_process::ListProcessesTool::new());
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_process::KillProcessTool::new());
    
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
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_introspection::GetUsageStatsTool::new(usage_tracker.clone()));
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_introspection::GetRecentToolCallsTool::new());
    
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
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_prompt::AddPromptTool::new().await?);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_prompt::EditPromptTool::new().await?);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_prompt::DeletePromptTool::new().await?);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_prompt::GetPromptTool::new().await?);
    
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
    
    let thinking_tool = Arc::new(kodegen_tools_sequential_thinking::SequentialThinkingTool::new());
    
    let (tool_router, prompt_router) = register_tool_arc(
        tool_router,
        prompt_router,
        thinking_tool.clone()
    );
    
    // Start cleanup task after tool is registered to avoid race conditions
    thinking_tool.start_cleanup_task();
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "reasoner")]
async fn register_reasoner_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing reasoner tools");

    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_reasoner::SequentialThinkingReasonerTool::new(None)
    );

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
    
    let agent_manager = Arc::new(kodegen_tools_claude_agent::AgentManager::new());
    let prompt_manager = Arc::new(kodegen_tools_prompt::PromptManager::new());
    prompt_manager.init().await.map_err(|e| anyhow::anyhow!("Failed to init prompt manager: {e}"))?;
    
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_claude_agent::SpawnClaudeAgentTool::new(agent_manager.clone(), prompt_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_claude_agent::ReadClaudeAgentOutputTool::new(agent_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_claude_agent::SendClaudeAgentPromptTool::new(agent_manager.clone(), prompt_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_claude_agent::TerminateClaudeAgentSessionTool::new(agent_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_claude_agent::ListClaudeAgentsTool::new(agent_manager)
    );
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "citescrape")]
async fn register_citescrape_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>, Arc<kodegen_tools_citescrape::BrowserManager>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing citescrape tools");
    
    let session_manager = Arc::new(kodegen_tools_citescrape::CrawlSessionManager::new());
    let engine_cache = Arc::new(kodegen_tools_citescrape::SearchEngineCache::new());
    let browser_manager = Arc::new(kodegen_tools_citescrape::BrowserManager::new());
    
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_citescrape::StartCrawlTool::new(session_manager.clone(), engine_cache.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_citescrape::GetCrawlResultsTool::new(session_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_citescrape::SearchCrawlResultsTool::new(session_manager.clone(), engine_cache.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_citescrape::WebSearchTool::new(browser_manager.clone())
    );
    
    // Start cleanup tasks after all tools are registered to avoid race conditions
    session_manager.start_cleanup_task();
    engine_cache.start_cleanup_task();
    
    Ok((tool_router, prompt_router, browser_manager))
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
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitInitTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitOpenTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitCloneTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitDiscoverTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitBranchCreateTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitBranchDeleteTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitBranchListTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitBranchRenameTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitCommitTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitLogTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitAddTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitCheckoutTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitFetchTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitMergeTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitWorktreeAddTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitWorktreeRemoveTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitWorktreeListTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitWorktreeLockTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitWorktreeUnlockTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_git::GitWorktreePruneTool);
    
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
    
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::CreateIssueTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::GetIssueTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::ListIssuesTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::UpdateIssueTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::SearchIssuesTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::AddIssueCommentTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::GetIssueCommentsTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::CreatePullRequestTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::UpdatePullRequestTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::MergePullRequestTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::GetPullRequestStatusTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::GetPullRequestFilesTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::GetPullRequestReviewsTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::CreatePullRequestReviewTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::AddPullRequestReviewCommentTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::RequestCopilotReviewTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::CreateRepositoryTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::ForkRepositoryTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::ListBranchesTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::CreateBranchTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::ListCommitsTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::GetCommitTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::SearchCodeTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::SearchRepositoriesTool);
    let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, kodegen_tools_github::SearchUsersTool);

    Ok((tool_router, prompt_router))
}

#[cfg(feature = "config")]
async fn register_config_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    config_manager: &kodegen_tools_config::ConfigManager,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing config tools");
    
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_config::GetConfigTool::new(config_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_config::SetConfigValueTool::new(config_manager.clone())
    );
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "database")]
async fn register_database_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
    pool: Arc<sqlx::AnyPool>,
    connection_url: &str,
    config_manager: &kodegen_tools_config::ConfigManager,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing database tools");
    
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_database::tools::ExecuteSQLTool::new(pool.clone(), config_manager.clone(), connection_url)?
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_database::tools::ListSchemasTool::new(pool.clone(), connection_url, config_manager.clone())?
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_database::tools::ListTablesTool::new(pool.clone(), connection_url, config_manager.clone())?
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_database::tools::GetTableSchemaTool::new(pool.clone(), connection_url, Arc::new(config_manager.clone()))?
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_database::tools::GetTableIndexesTool::new(pool.clone(), connection_url, Arc::new(config_manager.clone()))?
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_database::tools::GetStoredProceduresTool::new(pool.clone(), connection_url, Arc::new(config_manager.clone()))?
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_database::tools::GetPoolStatsTool::new(pool.clone(), connection_url)?
    );
    
    Ok((tool_router, prompt_router))
}

#[cfg(feature = "browser")]
async fn register_browser_tools<S>(
    tool_router: ToolRouter<S>,
    prompt_router: PromptRouter<S>,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static
{
    log::debug!("Initializing browser tools");
    
    // Get global browser manager singleton (lazy-loads Chrome on first use)
    let browser_manager = kodegen_tools_browser::BrowserManager::global();
    
    // Register all 7 available tools
    let (tool_router, prompt_router) = register_tool(
        tool_router, 
        prompt_router,
        kodegen_tools_browser::BrowserNavigateTool::new(browser_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router, 
        prompt_router,
        kodegen_tools_browser::BrowserScreenshotTool::new(browser_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router, 
        prompt_router,
        kodegen_tools_browser::BrowserClickTool::new(browser_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router, 
        prompt_router,
        kodegen_tools_browser::BrowserTypeTextTool::new(browser_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router, 
        prompt_router,
        kodegen_tools_browser::BrowserExtractTextTool::new(browser_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router, 
        prompt_router,
        kodegen_tools_browser::BrowserScrollTool::new(browser_manager.clone())
    );
    let (tool_router, prompt_router) = register_tool(
        tool_router,
        prompt_router,
        kodegen_tools_browser::BrowserWaitTool::new()
    );
    
    Ok((tool_router, prompt_router))
}
