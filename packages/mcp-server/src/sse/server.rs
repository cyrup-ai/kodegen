// packages/server/src/sse/server.rs
use anyhow::Result;
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
};
use std::net::SocketAddr;

/// MCP Server that serves tools via SSE transport
#[derive(Clone)]
pub struct SseServer {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    usage_tracker: UsageTracker,
    config_manager: kodegen_tools_config::ConfigManager,
    managers: std::sync::Arc<crate::common::router_builder::Managers>,
}

impl SseServer {
    /// Create a new SSE server with pre-built routers and managers
    pub fn new(
        tool_router: ToolRouter<Self>,
        prompt_router: PromptRouter<Self>,
        usage_tracker: UsageTracker,
        config_manager: kodegen_tools_config::ConfigManager,
        managers: crate::common::router_builder::Managers,
    ) -> Self {
        Self {
            tool_router,
            prompt_router,
            usage_tracker,
            config_manager,
            managers: std::sync::Arc::new(managers),
        }
    }

    /// Create and serve SSE server with optional TLS configuration
    pub async fn serve_with_tls(
        self,
        addr: SocketAddr,
        tls_config: Option<(std::path::PathBuf, std::path::PathBuf)>,
    ) -> Result<crate::sse::ServerHandle> {
        use rmcp::transport::sse_server::{SseServer as RmcpSseServer, SseServerConfig};
        use tokio::sync::oneshot;
        use tokio_util::sync::CancellationToken;

        // Clone managers before moving self
        let managers = self.managers.clone();

        let protocol = if tls_config.is_some() {
            "https"
        } else {
            "http"
        };
        log::info!("Starting SSE server on {protocol}://{addr}");
        log::info!("SSE endpoint: {protocol}://{addr}/sse");
        log::info!("Message endpoint: {protocol}://{addr}/message");

        // Create completion channel for graceful shutdown signaling
        let (completion_tx, completion_rx) = oneshot::channel();
        let ct = CancellationToken::new();

        // Create rmcp SSE server config
        let sse_config = SseServerConfig {
            bind: addr,
            sse_path: "/sse".to_string(),
            post_path: "/message".to_string(),
            ct: ct.clone(),
            sse_keep_alive: None,
        };

        // Create SSE server and router using new() instead of serve()
        let (sse_server, router) = RmcpSseServer::new(sse_config);

        // Create axum-server handle for graceful shutdown
        let axum_handle = axum_server::Handle::new();
        let shutdown_handle = axum_handle.clone();

        // Spawn server with or without TLS
        let server_task = if let Some((cert_path, key_path)) = tls_config {
            // TLS/HTTPS mode
            log::info!("Loading TLS certificate from: {cert_path:?}");
            log::info!("Loading TLS private key from: {key_path:?}");

            let rustls_config =
                axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to load TLS configuration: {e}"))?;

            tokio::spawn(async move {
                let result = axum_server::bind_rustls(addr, rustls_config)
                    .handle(axum_handle)
                    .serve(router.into_make_service())
                    .await;

                if let Err(e) = result {
                    log::error!("SSE server error: {e}");
                }
            })
        } else {
            // Plain HTTP mode
            tokio::spawn(async move {
                let result = axum_server::bind(addr)
                    .handle(axum_handle)
                    .serve(router.into_make_service())
                    .await;

                if let Err(e) = result {
                    log::error!("SSE server error: {e}");
                }
            })
        };

        // Attach our service to handle incoming transports
        let _service_ct = sse_server.with_service_directly(move || self.clone());

        // Clone ct for the monitor task (managers already cloned above)
        let ct_clone = ct.clone();

        // Spawn monitor task to handle graceful shutdown and completion signaling
        tokio::spawn(async move {
            // Wait for cancellation signal
            ct_clone.cancelled().await;
            let shutdown_start = std::time::Instant::now();

            log::debug!("Cancellation token fired, initiating graceful shutdown");
            shutdown_handle.graceful_shutdown(None);

            // Wait for server task to actually complete
            let _ = server_task.await;
            let shutdown_duration = shutdown_start.elapsed();

            log::debug!(
                "Server shutdown completed after {shutdown_duration:?}, shutting down managers"
            );

            // Shutdown managers (browser, etc.)
            if let Err(e) = managers.shutdown().await {
                log::error!("Failed to shutdown managers: {e}");
            }

            // Signal that shutdown is fully complete
            match completion_tx.send(()) {
                Ok(()) => {
                    log::debug!("Completion signal sent successfully");
                }
                Err(()) => {
                    log::debug!(
                        "Completion signal not sent - receiver already dropped. \
                         Timeout may have expired before shutdown completed."
                    );
                }
            }
        });

        Ok(crate::sse::ServerHandle::new(ct, completion_rx))
    }
}

impl ServerHandler for SseServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "KODEGEN SSE Server - Direct tool execution over HTTP/SSE".to_string(),
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

        // ACTUAL TOOL EXECUTION via router
        let result = self.tool_router.call(tcc).await;

        // Track usage metrics
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
        // Capture client information from MCP handshake
        if let Err(e) = self
            .config_manager
            .set_client_info(request.client_info)
            .await
        {
            log::warn!("Failed to store client info: {e:?}");
        }

        Ok(self.get_info())
    }
}
