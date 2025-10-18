// packages/server/src/sse/server.rs
use anyhow::Result;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::router::{tool::ToolRouter, prompt::PromptRouter},
    model::*,
    service::RequestContext,
};
use std::net::SocketAddr;
use kodegen_utils::usage_tracker::UsageTracker;

/// MCP Server that serves tools via SSE transport
#[derive(Clone)]
pub struct SseServer {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    usage_tracker: UsageTracker,
    config_manager: kodegen_config::ConfigManager,
}

impl SseServer {
    /// Create a new SSE server with pre-built routers
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
    
    /// Create and serve SSE server on the given address
    pub async fn serve(self, addr: SocketAddr) -> Result<crate::sse::ServerHandle> {
        use rmcp::transport::sse_server::SseServer as RmcpSseServer;
        use tokio::sync::oneshot;
        
        log::info!("Starting SSE server on http://{}", addr);
        log::info!("SSE endpoint: http://{}/sse", addr);
        log::info!("Message endpoint: http://{}/message", addr);
        
        // Create completion channel for graceful shutdown signaling
        let (completion_tx, completion_rx) = oneshot::channel();
        
        // Start rmcp SSE server
        let ct = RmcpSseServer::serve(addr)
            .await?
            .with_service_directly(move || self.clone());
        
        // Spawn monitor task to detect shutdown initiation
        // Note: rmcp doesn't expose task completion, so we signal when
        // cancellation is triggered. This allows early exit from timeout
        // while the configured timeout acts as a safety maximum.
        let monitor_ct = ct.clone();
        let cancellation_time = std::time::Instant::now();
        tokio::spawn(async move {
            // Wait for cancellation signal
            monitor_ct.cancelled().await;
            let signal_latency = cancellation_time.elapsed();
            
            log::debug!(
                "Server cancellation detected after {:?}, signaling shutdown readiness",
                signal_latency
            );
            
            // Signal that cancellation has been processed
            match completion_tx.send(()) {
                Ok(()) => {
                    log::debug!("Completion signal sent successfully after {:?}", signal_latency);
                }
                Err(_) => {
                    log::debug!(
                        "Completion signal not sent - receiver already dropped after {:?}. \
                         Shutdown completed before monitor could signal, or timeout expired.",
                        signal_latency
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
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(
            self, 
            request, 
            context
        );
        
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
        if let Err(e) = self.config_manager.set_client_info(request.client_info).await {
            log::warn!("Failed to store client info: {:?}", e);
        }
        
        Ok(self.get_info())
    }
}
