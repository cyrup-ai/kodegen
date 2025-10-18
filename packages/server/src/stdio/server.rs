// packages/server/src/stdio/server.rs
use anyhow::Result;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
    handler::server::router::{tool::ToolRouter, prompt::PromptRouter},
    model::{
        CallToolRequestParam, CallToolResult, GetPromptRequestParam, GetPromptResult,
        Implementation, InitializeRequestParam, InitializeResult, ListPromptsResult,
        ListResourceTemplatesResult, ListResourcesResult, ListToolsResult,
        PaginatedRequestParam, ProtocolVersion, ReadResourceRequestParam,
        ReadResourceResult, ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
    transport::stdio,
};
use serde_json::json;
use std::time::Duration;
use kodegen_utils::usage_tracker::UsageTracker;
use tokio_util::sync::CancellationToken;

/// Connect to SSE server with exponential backoff retry
/// 
/// Attempts connection up to `max_attempts` times with exponential backoff.
/// Backoff starts at `initial_backoff` and doubles on each retry, capped at 10 seconds.
/// Each connection attempt is subject to the specified timeout and can be cancelled via shutdown token.
/// 
/// # Arguments
/// * `url` - SSE server URL to connect to
/// * `max_attempts` - Maximum number of connection attempts
/// * `initial_backoff` - Initial backoff duration, doubles on each retry
/// * `timeout` - Timeout for each individual connection attempt
/// * `shutdown_token` - Cancellation token for graceful shutdown
/// 
/// # Returns
/// * `Ok((client, connection))` - Successfully connected client and connection tuple
/// * `Err` - Connection failed (timeout, cancellation, or connection error)
async fn connect_with_retry(
    url: &str,
    max_attempts: u32,
    initial_backoff: Duration,
    timeout: Duration,
    shutdown_token: &CancellationToken,
) -> Result<(kodegen_mcp_client::KodegenClient, kodegen_mcp_client::KodegenConnection)> {
    let mut backoff = initial_backoff;
    
    for attempt in 1..=max_attempts {
        log::debug!("SSE connection attempt {}/{} to {} (timeout: {:?})", attempt, max_attempts, url, timeout);
        
        // Race connection against timeout and cancellation
        let connect_future = kodegen_mcp_client::create_sse_client(url);
        let result = tokio::select! {
            res = connect_future => Some(res),
            _ = tokio::time::sleep(timeout) => None,
            _ = shutdown_token.cancelled() => {
                log::info!("Connection attempt cancelled during shutdown");
                return Err(anyhow::anyhow!("Connection cancelled during shutdown"));
            }
        };
        
        match result {
            Some(Ok((client, connection))) => {
                if attempt > 1 {
                    log::info!("Connected to SSE server on attempt {}/{}", attempt, max_attempts);
                } else {
                    log::info!("Connected to SSE server");
                }
                return Ok((client, connection));
            }
            Some(Err(e)) => {
                if attempt == max_attempts {
                    return Err(anyhow::anyhow!(
                        "Failed to connect after {} attempt{}: {}",
                        max_attempts,
                        if max_attempts == 1 { "" } else { "s" },
                        e
                    ));
                }
                
                log::debug!(
                    "Connection attempt {}/{} failed: {}. Retrying in {:?}",
                    attempt,
                    max_attempts,
                    e,
                    backoff
                );
            }
            None => {
                // Timeout
                if attempt == max_attempts {
                    return Err(anyhow::anyhow!(
                        "Connection timeout after {} attempt{} ({}s per attempt)",
                        max_attempts,
                        if max_attempts == 1 { "" } else { "s" },
                        timeout.as_secs()
                    ));
                }
                
                log::debug!(
                    "Connection attempt {}/{} timed out after {:?}. Retrying in {:?}",
                    attempt,
                    max_attempts,
                    timeout,
                    backoff
                );
            }
        }
        
        // Sleep before retry, but make it cancellable
        tokio::select! {
            _ = tokio::time::sleep(backoff) => {},
            _ = shutdown_token.cancelled() => {
                log::info!("Connection retry cancelled during backoff");
                return Err(anyhow::anyhow!("Connection cancelled during shutdown"));
            }
        }
        
        // Double backoff for next iteration, capped at 10 seconds
        backoff = (backoff * 2).min(Duration::from_secs(10));
    }
    
    unreachable!()
}

/// MCP Server that provides stdio transport with optional SSE proxy
/// 
/// When configured with an SSE URL, forwards tool execution to the SSE server.
/// When no SSE URL provided, executes tools locally (standalone mode).
pub struct StdioProxyServer {
    /// Optional SSE client for proxying tool calls.
    /// KodegenClient is cheap to clone (Arc pointers internally).
    sse_client: Option<kodegen_mcp_client::KodegenClient>,
    
    /// Connection lifecycle manager (not Clone, held to keep connection alive).
    /// When this is dropped, the SSE connection will be closed.
    /// NOT wrapped in Arc - KodegenConnection should never be cloned.
    #[allow(dead_code)]
    sse_connection: Option<kodegen_mcp_client::KodegenConnection>,
    
    /// Tool router for metadata and optional local execution
    tool_router: ToolRouter<Self>,
    /// Prompt router for serving prompts locally
    prompt_router: PromptRouter<Self>,
    /// Usage tracker for metrics
    usage_tracker: UsageTracker,
    /// Configuration manager
    config_manager: kodegen_config::ConfigManager,
}

impl StdioProxyServer {
    /// Create a new stdio server with optional SSE proxy
    /// 
    /// # Arguments
    /// * `sse_url` - Optional URL of SSE server to proxy tool calls to
    /// * `config_manager` - Configuration manager
    /// * `usage_tracker` - Usage metrics tracker
    /// * `enabled_categories` - Tool categories to enable
    /// * `connection_timeout` - Timeout for each SSE connection attempt
    /// * `max_retries` - Maximum number of connection attempts (1 = no retry)
    /// * `retry_backoff` - Initial backoff duration, doubles on each retry
    /// * `proxy_required` - If true, server will fail if SSE connection fails (explicit proxy request)
    /// * `shutdown_token` - Cancellation token for graceful shutdown during initialization
    pub async fn new(
        sse_url: Option<&str>,
        config_manager: kodegen_config::ConfigManager,
        usage_tracker: UsageTracker,
        enabled_categories: &Option<std::collections::HashSet<String>>,
        connection_timeout: Duration,
        max_retries: u32,
        retry_backoff: Duration,
        proxy_required: bool,
        shutdown_token: CancellationToken,
    ) -> Result<Self> {
        // Build routers for metadata (schemas and prompts)
        let routers = crate::common::build_routers::<Self>(
            &config_manager,
            &usage_tracker,
            enabled_categories,
        ).await?;
        
        // Create SSE client if URL provided
        let (sse_client, sse_connection) = match sse_url {
            Some(url) => {
                if max_retries > 1 {
                    log::info!(
                        "Connecting to SSE server at {} (with retry, max {} attempts)",
                        url,
                        max_retries
                    );
                } else {
                    log::info!("Connecting to SSE server at {} (no retry)", url);
                }
                
                // Try to connect with exponential backoff retry
                // Connection attempts are cancellable and subject to timeout
                let connect_result = connect_with_retry(
                    url,
                    max_retries,
                    retry_backoff,
                    connection_timeout,
                    &shutdown_token
                ).await;
                
                match connect_result {
                    Ok((client, connection)) => {
                        (Some(client), Some(connection))
                    }
                    Err(e) => {
                        if proxy_required {
                            // User explicitly requested proxy - this is FATAL
                            log::error!(
                                "Failed to connect to SSE server: {}. \
                                 Cannot run in standalone mode when proxy explicitly requested.",
                                e
                            );
                            return Err(anyhow::anyhow!(
                                "SSE connection required but failed: {}",
                                e
                            ));
                        } else {
                            // Proxy was optional - fallback is acceptable
                            log::warn!(
                                "Failed to connect to SSE server after retries: {}. \
                                 Running in standalone mode.",
                                e
                            );
                            (None, None)
                        }
                    }
                }
            }
            None => {
                log::info!("No SSE URL provided, running in standalone mode");
                (None, None)
            }
        };
        
        Ok(Self {
            sse_client,
            sse_connection,
            tool_router: routers.tool_router,
            prompt_router: routers.prompt_router,
            usage_tracker,
            config_manager,
        })
    }
    
    /// Serve the stdio server
    pub async fn serve_stdio(self) -> Result<()> {
        log::info!("Starting stdio server (proxy mode: {})", 
                  if self.sse_client.is_some() { "enabled" } else { "disabled" });
        
        // Use rmcp's stdio transport
        let service = self.serve(stdio()).await.inspect_err(|e| {
            log::error!("serving error: {:?}", e);
        })?;
        service.waiting().await?;
        
        log::info!("Stdio server stopped");
        Ok(())
    }
}

impl ServerHandler for StdioProxyServer {
    fn get_info(&self) -> ServerInfo {
        let mode = if self.sse_client.is_some() { "proxy" } else { "standalone" };
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                format!("KODEGEN Stdio Server ({} mode) - MCP tools via stdio transport", mode)
            ),
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_name = request.name.clone();
        
        // Decide whether to proxy or execute locally
        let result = match &self.sse_client {
            Some(client) => {
                // PROXY MODE: Forward to SSE server
                log::debug!("Proxying tool call '{}' to SSE server", tool_name);
                
                // Convert arguments to JSON value
                let args = match request.arguments {
                    Some(map) => serde_json::Value::Object(map),
                    None => serde_json::Value::Object(serde_json::Map::new()),
                };
                
                // Call tool via SSE client
                match client.call_tool(&tool_name, args).await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        log::error!("SSE proxy error for tool '{}': {}", tool_name, e);
                        Err(McpError::internal_error(
                            format!("SSE proxy error: {}", e),
                            None
                        ))
                    }
                }
            }
            None => {
                // STANDALONE MODE: Execute locally
                log::debug!("Executing tool '{}' locally", tool_name);
                let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
                self.tool_router.call(tcc).await
            }
        };
        
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
        // Always serve tool metadata locally (for efficiency)
        let items = self.tool_router.list_all();
        Ok(ListToolsResult::with_all_items(items))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        // Always serve prompts locally (no execution needed)
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
        // Always serve prompt metadata locally
        let items = self.prompt_router.list_all();
        Ok(ListPromptsResult::with_all_items(items))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        // No resources in this implementation
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
        // Capture client information
        if let Err(e) = self.config_manager.set_client_info(request.client_info).await {
            log::warn!("Failed to store client info: {:?}", e);
        }
        
        Ok(self.get_info())
    }
}
