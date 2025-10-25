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
) -> Result<(
    kodegen_mcp_client::KodegenClient,
    kodegen_mcp_client::KodegenConnection,
)> {
    let mut backoff = initial_backoff;

    for attempt in 1..=max_attempts {
        log::debug!(
            "SSE connection attempt {attempt}/{max_attempts} to {url} (timeout: {timeout:?})"
        );

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
/// Forwards tool execution to category-specific SSE servers.
/// Uses static metadata from kodegen_mcp_schema (no tool instantiation).
pub struct StdioProxyServer {
    /// SSE clients for each category server (category -> client)
    /// Each client is cheap to clone (Arc pointers internally)
    category_clients: HashMap<String, kodegen_mcp_client::KodegenClient>,

    /// Connection lifecycle managers for each category (not Clone)
    /// When these are dropped, SSE connections will be closed
    #[allow(dead_code)]
    category_connections: Vec<kodegen_mcp_client::KodegenConnection>,

    /// Routing table: tool_name -> (category, port)
    routing_table: HashMap<&'static str, (&'static str, u16)>,

    /// Enabled tool names (filtered by --tool/--tools CLI args)
    enabled_tools: Option<std::collections::HashSet<String>>,

    /// Usage tracker for metrics
    usage_tracker: UsageTracker,

    /// Configuration manager
    config_manager: kodegen_tools_config::ConfigManager,
}

impl StdioProxyServer {
    /// Create a new stdio server (thin client)
    ///
    /// # Arguments
    /// * `config_manager` - Configuration manager
    /// * `usage_tracker` - Usage metrics tracker
    /// * `enabled_categories` - Tool categories to enable (from CLI --tools)
    /// * `sse_config` - SSE connection configuration (retry, timeout, etc.)
    /// * `shutdown_token` - Cancellation token for graceful shutdown during initialization
    pub async fn new(
        config_manager: kodegen_tools_config::ConfigManager,
        usage_tracker: UsageTracker,
        enabled_categories: &Option<std::collections::HashSet<String>>,
        sse_config: SseConnectionConfig,
        shutdown_token: CancellationToken,
    ) -> Result<Self> {
        // Build routing table from static metadata
        let routing_table = build_routing_table();

        // Determine which categories need SSE connections
        let mut categories_to_connect: Vec<&str> = Vec::new();
        
        if let Some(enabled) = enabled_categories {
            // Filter categories based on enabled list
            for &(category, _port) in CATEGORY_PORTS {
                if enabled.contains(category) {
                    categories_to_connect.push(category);
                }
            }
        } else {
            // No filter - connect to all categories
            for &(category, _port) in CATEGORY_PORTS {
                categories_to_connect.push(category);
            }
        }

        log::info!(
            "Connecting to {} category servers: {}",
            categories_to_connect.len(),
            categories_to_connect.join(", ")
        );

        // Connect to each category server
        let mut category_clients = HashMap::new();
        let mut category_connections = Vec::new();
        let port_map: HashMap<&str, u16> = CATEGORY_PORTS.iter().copied().collect();

        for category in categories_to_connect {
            let port = port_map.get(category).copied().ok_or_else(|| {
                anyhow::anyhow!("No port assignment for category: {}", category)
            })?;

            let url = format!("http://localhost:{}/sse", port);
            
            log::debug!("Connecting to {category} server at {url}");

            match connect_with_retry(
                &url,
                sse_config.max_retries,
                sse_config.retry_backoff,
                sse_config.connection_timeout,
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

        // Build enabled_tools set from enabled_categories
        let enabled_tools = enabled_categories.as_ref().map(|categories| {
            let mut tools = std::collections::HashSet::new();
            for tool_meta in all_tool_metadata() {
                if categories.contains(tool_meta.category) {
                    tools.insert(tool_meta.name.to_string());
                }
            }
            tools
        });

        log::info!(
            "Stdio proxy server initialized with {} category connections",
            category_clients.len()
        );

        Ok(Self {
            category_clients,
            category_connections,
            routing_table,
            enabled_tools,
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

        // Call tool via category SSE client
        let result = match client.call_tool(&tool_name, args).await {
            Ok(result) => Ok(result),
            Err(e) => {
                log::error!("SSE proxy error for tool '{}': {}", tool_name, e);
                Err(McpError::internal_error(
                    format!("SSE proxy error: {}", e),
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
