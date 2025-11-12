// packages/server/src/stdio/server.rs
use anyhow::Result;
use kodegen_utils::usage_tracker::UsageTracker;
use rand::Rng;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
    model::{
        CallToolRequestParam, CallToolResult, GetPromptRequestParam, GetPromptResult,
        Implementation, InitializeRequestParam, InitializeResult, ListPromptsResult,
        ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, PaginatedRequestParam,
        ProtocolVersion, ReadResourceRequestParam, ReadResourceResult, ServerCapabilities,
        ServerInfo, Tool,
    },
    service::RequestContext,
    transport::stdio,
};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use super::metadata::{all_tool_metadata, build_routing_table, CATEGORY_PORTS};

/// Configuration for HTTP connection retry logic
#[derive(Debug, Clone)]
pub struct HttpConnectionConfig {
    /// Initial backoff duration, doubles on each retry
    pub retry_backoff: Duration,
    /// Maximum number of connection attempts
    pub max_retries: u32,
    /// Timeout for each connection attempt
    pub connection_timeout: Duration,
    /// HTTP server host (default: mcp.kodegen.ai)
    pub host: String,
    /// Disable TLS (use HTTP instead of HTTPS)
    pub no_tls: bool,
}

impl Default for HttpConnectionConfig {
    fn default() -> Self {
        Self {
            retry_backoff: Duration::from_millis(100),
            max_retries: 1,
            connection_timeout: Duration::from_secs(30),
            host: "mcp.kodegen.ai".to_string(),
            no_tls: false,
        }
    }
}

/// Connect to HTTP server with exponential backoff retry
///
/// Attempts connection up to `max_attempts` times with exponential backoff.
/// Backoff starts at `initial_backoff` and doubles on each retry, capped at 10 seconds.
/// Each connection attempt is subject to the specified timeout and can be cancelled via shutdown token.
///
/// # Arguments
/// * `url` - HTTP server URL to connect to
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
) -> Result<(
    kodegen_mcp_client::KodegenClient,
    kodegen_mcp_client::KodegenConnection,
)> {
    let mut backoff = initial_backoff;

    for attempt in 1..=max_attempts {
        log::debug!(
            "HTTP connection attempt {attempt}/{max_attempts} to {url} (timeout: {timeout:?})"
        );

        // Race connection against timeout and cancellation
        let connect_future = kodegen_mcp_client::create_streamable_client(url);
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
                    log::info!("Connected to HTTP server on attempt {attempt}/{max_attempts}");
                } else {
                    log::info!("Connected to HTTP server");
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
/// Forwards tool execution to category-specific HTTP servers.
/// Uses static metadata from kodegen_mcp_schema (no tool instantiation).
pub struct StdioProxyServer {
    /// HTTP clients for each category server (category -> client)
    /// Each client is cheap to clone (Arc pointers internally)
    category_clients: HashMap<String, kodegen_mcp_client::KodegenClient>,

    /// Connection lifecycle managers for each category (not Clone)
    /// When these are dropped, HTTP connections will be closed
    #[allow(dead_code)]
    category_connections: Vec<kodegen_mcp_client::KodegenConnection>,

    /// Routing table: tool_name -> (category, port)
    routing_table: HashMap<&'static str, (&'static str, u16)>,

    /// Enabled tool names (filtered by --tool/--tools CLI args)
    enabled_tools: Option<std::collections::HashSet<String>>,

    /// Usage tracker for metrics
    usage_tracker: UsageTracker,

    /// Configuration manager
    config_manager: kodegen_config_manager::ConfigManager,
}

impl StdioProxyServer {
    /// Create a new stdio server (thin client)
    ///
    /// # Arguments
    /// * `config_manager` - Configuration manager
    /// * `usage_tracker` - Usage metrics tracker
    /// * `enabled_tools` - Individual tool names to enable (from CLI --tool/--tools/--toolset)
    /// * `http_config` - HTTP connection configuration (retry, timeout, etc.)
    /// * `shutdown_token` - Cancellation token for graceful shutdown during initialization
    pub async fn new(
        config_manager: kodegen_config_manager::ConfigManager,
        usage_tracker: UsageTracker,
        enabled_tools: &Option<std::collections::HashSet<String>>,
        http_config: HttpConnectionConfig,
        shutdown_token: CancellationToken,
    ) -> Result<Self> {
        // Build routing table from static metadata
        let routing_table = build_routing_table();

        // Determine which categories need HTTP connections based on enabled TOOLS
        let mut categories_to_connect: std::collections::HashSet<&str> = std::collections::HashSet::new();
        
        if let Some(enabled) = enabled_tools {
            // Find which categories are needed for the enabled tools
            for tool_meta in all_tool_metadata() {
                if enabled.contains(tool_meta.name) {
                    categories_to_connect.insert(tool_meta.category);
                }
            }
        } else {
            // No filter - connect to all categories
            for &(category, _port) in CATEGORY_PORTS {
                categories_to_connect.insert(category);
            }
        }
        
        let categories_vec: Vec<&str> = categories_to_connect.iter().copied().collect();

        log::info!(
            "Connecting to {} category servers: {}",
            categories_vec.len(),
            categories_vec.join(", ")
        );

        // Connect to each category server
        let mut category_clients = HashMap::new();
        let mut category_connections = Vec::new();
        let port_map: HashMap<&str, u16> = CATEGORY_PORTS.iter().copied().collect();

        for category in categories_vec {
            let port = port_map.get(category).copied().ok_or_else(|| {
                anyhow::anyhow!("No port assignment for category: {}", category)
            })?;

            let protocol = if http_config.no_tls { "http" } else { "https" };
            let url = format!("{}://{}:{}/mcp", protocol, http_config.host, port);
            
            log::debug!("Connecting to {category} server at {url}");

            match connect_with_retry(
                &url,
                http_config.max_retries,
                http_config.retry_backoff,
                http_config.connection_timeout,
                &shutdown_token,
            )
            .await
            {
                Ok((client, connection)) => {
                    category_clients.insert(category.to_string(), client);
                    category_connections.push(connection);
                    log::info!("Connected to {category} server (port {port})");
                }
                Err(e) => {
                    log::warn!(
                        "Failed to connect to {category} server (port {port}): {e}. Tools in this category will be unavailable."
                    );
                }
            }
        }

        if category_clients.is_empty() {
            return Err(anyhow::anyhow!(
                "Failed to connect to any category servers. No tools available."
            ));
        }

        // Store enabled_tools for filtering during list_tools
        let enabled_tools_set = enabled_tools.clone();

        log::info!(
            "Stdio proxy server initialized with {} category connections",
            category_clients.len()
        );

        Ok(Self {
            category_clients,
            category_connections,
            routing_table,
            enabled_tools: enabled_tools_set,
            usage_tracker,
            config_manager,
        })
    }

    /// Serve the stdio server
    pub async fn serve_stdio(self) -> Result<()> {
        log::info!("Starting stdio server (thin client mode with static metadata)");

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
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "KODEGEN Stdio Server (thin client) - MCP tools via stdio transport using static metadata".to_string(),
            ),
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_name = request.name.clone();

        // Check if tool is enabled
        if let Some(ref enabled) = self.enabled_tools
            && !enabled.contains(&*tool_name) {
                return Err(McpError::invalid_params(
                    format!("Tool '{}' is not enabled", tool_name),
                    None,
                ));
            }

        // Route to appropriate category server
        let (category, _port) = self.routing_table.get(&*tool_name).ok_or_else(|| {
            McpError::invalid_params(format!("Unknown tool: {}", tool_name), None)
        })?;

        let client = self.category_clients.get(*category).ok_or_else(|| {
            McpError::internal_error(
                format!(
                    "Category server '{}' not connected (tool: {})",
                    category, tool_name
                ),
                None,
            )
        })?;

        log::debug!(
            "Proxying tool call '{}' to category '{}' server",
            tool_name,
            category
        );

        // Convert arguments to JSON value
        let args = match request.arguments {
            Some(map) => serde_json::Value::Object(map),
            None => serde_json::Value::Object(serde_json::Map::new()),
        };

        // Call tool via category HTTP client
        let result = match client.call_tool(&tool_name, args).await {
            Ok(result) => Ok(result),
            Err(e) => {
                log::error!("HTTP proxy error for tool '{}': {}", tool_name, e);
                Err(McpError::internal_error(
                    format!("HTTP proxy error: {}", e),
                    None,
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
        // Serve tool metadata from static metadata (no tool instantiation)
        let mut tools = Vec::new();

        for tool_meta in all_tool_metadata() {
            // Filter by enabled_tools if set
            if let Some(ref enabled) = self.enabled_tools
                && !enabled.contains(tool_meta.name) {
                    continue;
                }

            // Only include tools whose category server is connected
            if !self.category_clients.contains_key(tool_meta.category) {
                continue;
            }

            // Convert schema Value to Arc<JsonObject>
            let schema_obj = match tool_meta.schema.clone() {
                serde_json::Value::Object(obj) => std::sync::Arc::new(obj),
                _ => std::sync::Arc::new(serde_json::Map::new()),
            };

            tools.push(Tool {
                name: tool_meta.name.to_string().into(),
                title: None,
                description: Some(tool_meta.description.to_string().into()),
                input_schema: schema_obj,
                output_schema: None,
                annotations: None,
                icons: None,
            });
        }

        log::debug!("Serving {} tools from static metadata", tools.len());

        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn get_prompt(
        &self,
        _request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        // Stdio mode does not support prompts (prompts require tool instantiation)
        Err(McpError::invalid_request("Prompts not supported in stdio mode", None))
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        // Stdio mode does not support prompts
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    /// Resources are not implemented in stdio mode.
    ///
    /// This server focuses on tool execution via HTTP category servers.
    /// Resources capability is not advertised, so clients should not call these methods.
    /// These methods exist only to satisfy the ServerHandler trait.
    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        // Resources not supported in stdio mode (similar to prompts)
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        _request: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        // Resources not supported in stdio mode
        Err(McpError::invalid_request(
            "Resources not supported in stdio mode",
            Some(json!({
                "message": "This server only supports tools. Resources are not available in stdio mode.",
                "uri": _request.uri
            }))
        ))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        // Resources not supported in stdio mode
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
        // Store client info (fire-and-forget, errors logged in background task)
        let _ = self.config_manager.set_client_info(request.client_info).await;
        Ok(self.get_info())
    }
}
