// Category SSE Server: Git Tools
//
// This binary serves only git-related tools over SSE/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30450.

use anyhow::Result;
use clap::Parser;
use kodegen_utils::usage_tracker::UsageTracker;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::router::{prompt::PromptRouter, tool::ToolRouter},
    model::{
        CallToolRequestParam, CallToolResult, GetPromptRequestParam, GetPromptResult,
        Implementation, InitializeRequestParam, InitializeResult, ListPromptsResult,
        ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, PaginatedRequestParam,
        ProtocolVersion, ReadResourceRequestParam, ReadResourceResult, ServerCapabilities,
        ServerInfo,
    },
    service::RequestContext,
    transport::sse_server::{SseServer as RmcpSseServer, SseServerConfig},
};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[command(name = "kodegen-git")]
#[command(about = "Git version control tools SSE server")]
struct Args {
    /// SSE server bind address (e.g., "127.0.0.1:30450")
    #[arg(long)]
    sse: SocketAddr,

    /// TLS certificate path (PEM format)
    #[arg(long, requires = "tls_key")]
    tls_cert: Option<PathBuf>,

    /// TLS private key path (PEM format)
    #[arg(long, requires = "tls_cert")]
    tls_key: Option<PathBuf>,
}

/// Git SSE Server handler
#[derive(Clone)]
struct GitServer {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    usage_tracker: UsageTracker,
    config_manager: kodegen_tools_config::ConfigManager,
}

impl ServerHandler for GitServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("KODEGEN Git Category Server".to_string()),
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
        request: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        use serde_json::json;
        Err(McpError::resource_not_found(
            "resource_not_found",
            Some(json!({ "uri": request.uri })),
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
        if let Err(e) = self.config_manager.set_client_info(request.client_info).await {
            log::warn!("Failed to store client info: {e:?}");
        }
        Ok(self.get_info())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Install default CryptoProvider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Parse CLI arguments
    let args = Args::parse();

    // Generate unique instance ID
    let instance_id = chrono::Utc::now().format("%Y%m%d-%H%M%S-git").to_string();

    // Initialize shared components
    let config_manager = kodegen_tools_config::ConfigManager::new();
    config_manager.init().await?;
    let usage_tracker = UsageTracker::new(instance_id.clone());

    // Initialize tool call history tracking
    kodegen_mcp_tool::tool_history::init_global_history(instance_id).await;

    // Build routers with only git tools
    let tool_router = ToolRouter::new();
    let prompt_router = PromptRouter::new();

    // Register all git tools (NO managers needed)
    let (tool_router, prompt_router) = register_git_tools(
        tool_router,
        prompt_router,
        &config_manager,
        &usage_tracker,
    )?;

    // Create server instance
    let server = GitServer {
        tool_router,
        prompt_router,
        usage_tracker,
        config_manager,
    };

    // Start SSE server
    let tls_config = args.tls_cert.zip(args.tls_key);
    let protocol = if tls_config.is_some() { "https" } else { "http" };

    log::info!("Starting git category SSE server on {protocol}://{}", args.sse);

    // Create completion channel for shutdown signaling
    let (completion_tx, completion_rx) = oneshot::channel();
    let ct = CancellationToken::new();

    // Create rmcp SSE server config
    let sse_config = SseServerConfig {
        bind: args.sse,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: ct.clone(),
        sse_keep_alive: None,
    };

    // Create SSE server and router
    let (sse_server, router) = RmcpSseServer::new(sse_config);

    // Create axum-server handle for graceful shutdown
    let axum_handle = axum_server::Handle::new();
    let shutdown_handle = axum_handle.clone();

    // Spawn server with or without TLS
    let server_task = if let Some((cert_path, key_path)) = tls_config {
        log::info!("Loading TLS certificate from: {cert_path:?}");
        log::info!("Loading TLS private key from: {key_path:?}");

        let rustls_config =
            axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await?;

        tokio::spawn(async move {
            let result = axum_server::bind_rustls(args.sse, rustls_config)
                .handle(axum_handle)
                .serve(router.into_make_service())
                .await;

            if let Err(e) = result {
                log::error!("SSE server error: {e}");
            }
        })
    } else {
        tokio::spawn(async move {
            let result = axum_server::bind(args.sse)
                .handle(axum_handle)
                .serve(router.into_make_service())
                .await;

            if let Err(e) = result {
                log::error!("SSE server error: {e}");
            }
        })
    };

    // Attach our service to handle incoming transports
    let _service_ct = sse_server.with_service_directly(move || server.clone());

    // Clone ct for the monitor task
    let ct_clone = ct.clone();

    // Spawn monitor task for graceful shutdown
    tokio::spawn(async move {
        ct_clone.cancelled().await;
        log::debug!("Cancellation token fired, initiating graceful shutdown");
        shutdown_handle.graceful_shutdown(None);

        let _ = server_task.await;
        log::debug!("Server shutdown completed");

        let _ = completion_tx.send(());
    });

    // Wait for shutdown signal
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let ctrl_c = tokio::signal::ctrl_c();
        let mut sigterm = signal(SignalKind::terminate())?;

        tokio::select! {
            _ = ctrl_c => log::debug!("Received SIGINT"),
            _ = sigterm.recv() => log::debug!("Received SIGTERM"),
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
    }

    log::info!("Shutdown signal received, initiating graceful shutdown");
    ct.cancel();

    match tokio::time::timeout(std::time::Duration::from_secs(30), completion_rx).await {
        Ok(Ok(())) => log::info!("Server shutdown completed successfully"),
        Ok(Err(_)) => log::warn!("Completion channel closed unexpectedly"),
        Err(_) => log::warn!("Graceful shutdown timeout elapsed, forcing exit"),
    }

    log::info!("Git server stopped");
    Ok(())
}

/// Register all git tools into routers
/// 
/// CRITICAL: Git tools are ZERO-STATE structs - pass directly, NO ::new()
fn register_git_tools<S>(
    mut tool_router: ToolRouter<S>,
    mut prompt_router: PromptRouter<S>,
    _config_manager: &kodegen_tools_config::ConfigManager,
    _usage_tracker: &UsageTracker,
) -> Result<(ToolRouter<S>, PromptRouter<S>)>
where
    S: Send + Sync + 'static,
{
    use kodegen_tools_git::*;
    use std::sync::Arc;

    // Helper function to register a tool
    // Note: Tools passed WITHOUT ::new() - they are zero-state structs
    fn register<S, T>(
        tool_router: ToolRouter<S>,
        prompt_router: PromptRouter<S>,
        tool: T,
    ) -> (ToolRouter<S>, PromptRouter<S>)
    where
        S: Send + Sync + 'static,
        T: kodegen_mcp_tool::Tool,
    {
        let tool = Arc::new(tool);
        let tool_router = tool_router.with_route(tool.clone().arc_into_tool_route());
        let prompt_router = prompt_router.with_route(tool.arc_into_prompt_route());
        (tool_router, prompt_router)
    }

    // Repository initialization (4 tools)
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitInitTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitOpenTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitCloneTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitDiscoverTool);

    // Branch operations (4 tools)
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitBranchCreateTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitBranchDeleteTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitBranchListTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitBranchRenameTool);

    // Core git operations (4 tools)
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitCommitTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitLogTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitAddTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitCheckoutTool);

    // Remote operations (2 tools)
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitFetchTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitMergeTool);

    // Worktree operations (6 tools)
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitWorktreeAddTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitWorktreeRemoveTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitWorktreeListTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitWorktreeLockTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitWorktreeUnlockTool);
    (tool_router, prompt_router) = register(tool_router, prompt_router, GitWorktreePruneTool);

    Ok((tool_router, prompt_router))
}
