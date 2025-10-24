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
use rand::Rng;
use serde_json::json;
use std::time::Duration;
use kodegen_utils::usage_tracker::UsageTracker;
use tokio_util::sync::CancellationToken;

/// Configuration for SSE connection retry logic
#[derive(Debug, Clone)]
pub struct SseConnectionConfig {
    /// Initial backoff duration, doubles on each retry
    pub retry_backoff: Duration,
    /// Maximum number of connection attempts
    pub max_retries: u32,
    /// Timeout for each connection attempt
    pub connection_timeout: Duration,
}

impl Default for SseConnectionConfig {
    fn default() -> Self {
        Self {
            retry_backoff: Duration::from_millis(100),
            max_retries: 1,
            connection_timeout: Duration::from_secs(30),
        }
    }
}

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
        log::debug!("SSE connection attempt {attempt}/{max_attempts} to {url} (timeout: {timeout:?})");
        
        // Race connection against timeout and cancellation
        let connect_future = kodegen_mcp_client::create_sse_client(url);
        let result = tokio::select! {
            res = connect_future => Some(res),
            () = tokio::time::sleep(timeout) => None,
            () = shutdown_token.cancelled() => {
                log::info!("Connection attempt cancelled during shutdown");
                return Err(anyhow::anyhow!("Connection cancelled during shutdown"));
            }
        };
        
        match result {
            Some(Ok((client, connection))) => {
                if attempt > 1 {
                    log::info!("Connected to SSE server on attempt {attempt}/{max_attempts}");
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
                    "Connection attempt {attempt}/{max_attempts} failed: {e}. Retrying in {backoff:?}"
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
                    "Connection attempt {attempt}/{max_attempts} timed out after {timeout:?}. Retrying in {backoff:?}"
                );
            }
        }
        
        // Sleep before retry, but make it cancellable
        // Add jitter (0-25% of backoff) to prevent thundering herd
        let jitter_max = (backoff.as_millis() / 4).max(1);
        let jitter = rand::rng().random_range(0..jitter_max);
        let sleep_duration = backoff + Duration::from_millis(jitter as u64);
        
        tokio::select! {
            () = tokio::time::sleep(sleep_duration) => {},
            () = shutdown_token.cancelled() => {
                log::info!("Connection retry cancelled during backoff");
                return Err(anyhow::anyhow!("Connection cancelled during shutdown"));
            }
        }
        
        // Double backoff for next iteration, capped at 10 seconds
        backoff = (backoff * 2).min(Duration::from_secs(10));
    }
    
    unreachable!()
}

/// MCP Server that provides stdio transport (thin client)
/// 
/// Forwards all tool execution to the SSE server (daemon).
pub struct StdioProxyServer {
    /// SSE client for proxying tool calls to daemon.
    /// `KodegenClient` is cheap to clone (Arc pointers internally).
    sse_client: kodegen_mcp_client::KodegenClient,
    
    /// Connection lifecycle manager (not Clone, held to keep connection alive).
    /// When this is dropped, the SSE connection will be closed.
    /// NOT wrapped in Arc - `KodegenConnection` should never be cloned.
    #[allow(dead_code)]
    sse_connection: kodegen_mcp_client::KodegenConnection,
    
    /// Tool router for metadata only
    tool_router: ToolRouter<Self>,
    /// Prompt router for serving prompts locally
    prompt_router: PromptRouter<Self>,
    /// Usage tracker for metrics
    usage_tracker: UsageTracker,
    /// Configuration manager
    config_manager: kodegen_tools_config::ConfigManager,
}

impl StdioProxyServer {
    /// Create a new stdio server (thin client)
    /// 
    /// # Arguments
    /// * `sse_url` - URL of SSE server (daemon) to proxy tool calls to
    /// * `config_manager` - Configuration manager
    /// * `usage_tracker` - Usage metrics tracker
    /// * `enabled_categories` - Tool categories to enable
    /// * `sse_config` - SSE connection configuration (retry, timeout, etc.)
    /// * `shutdown_token` - Cancellation token for graceful shutdown during initialization
    pub async fn new(
        sse_url: &str,
        config_manager: kodegen_tools_config::ConfigManager,
        usage_tracker: UsageTracker,
        enabled_categories: &Option<std::collections::HashSet<String>>,
        sse_config: SseConnectionConfig,
        shutdown_token: CancellationToken,
    ) -> Result<Self> {
        // Build routers for metadata (schemas and prompts)
        // Note: stdio mode doesn't support database - database tools only available in SSE mode
        let routers = crate::common::build_routers::<Self>(
            &config_manager,
            &usage_tracker,
            enabled_categories,
            None, // database_dsn
            None, // ssh_config
            None, // server_url (stdio mode proxies to daemon, doesn't run own server)
        ).await?;
        
        // Connect to SSE server (daemon)
        if sse_config.max_retries > 1 {
            log::info!(
                "Connecting to SSE server at {} (with retry, max {} attempts)",
                sse_url,
                sse_config.max_retries
            );
        } else {
            log::info!("Connecting to SSE server at {sse_url} (no retry)");
        }

        // Try to connect with exponential backoff retry
        // Connection attempts are cancellable and subject to timeout
        let (sse_client, sse_connection) = connect_with_retry(
            sse_url,
            sse_config.max_retries,
            sse_config.retry_backoff,
            sse_config.connection_timeout,
            &shutdown_token
        ).await?;
        
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
        log::info!("Starting stdio server (thin client mode)");
        
        // Use rmcp's stdio transport
        let service = self.serve(stdio()).await.inspect_err(|e| {
            log::error!("serving error: {e:?}");
        })?;
        service.waiting().await?;
        
        log::info!("Stdio server stopped");
        Ok(())
    }
}

impl ServerHandler for StdioProxyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "KODEGEN Stdio Server (thin client) - MCP tools via stdio transport".to_string()
            ),
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_name = request.name.clone();
        
        // Proxy all tool calls to SSE server (daemon)
        log::debug!("Proxying tool call '{tool_name}' to SSE server");
        
        // Convert arguments to JSON value
        let args = match request.arguments {
            Some(map) => serde_json::Value::Object(map),
            None => serde_json::Value::Object(serde_json::Map::new()),
        };
        
        // Call tool via SSE client
        let result = match self.sse_client.call_tool(&tool_name, args).await {
            Ok(result) => Ok(result),
            Err(e) => {
                log::error!("SSE proxy error for tool '{tool_name}': {e}");
                Err(McpError::internal_error(
                    format!("SSE proxy error: {e}"),
                    None
                ))
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
            log::warn!("Failed to store client info: {e:?}");
        }
        
        Ok(self.get_info())
    }
}
