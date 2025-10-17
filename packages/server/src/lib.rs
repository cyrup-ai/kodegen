//! Kodegen MCP Server Library
//!
//! Exposes reusable components for MCP server implementation

// Re-export main module items by declaring main as a public module
// This allows external crates to use server components without duplication

// Since main.rs has #[tokio::main], we can't directly include it as a module
// Instead, we duplicate the minimal necessary types and functions here

pub mod cli;

use anyhow::Result;
use rmcp::handler::server::router::{tool::ToolRouter, prompt::PromptRouter};
use std::collections::HashSet;
use kodegen_utils::usage_tracker::UsageTracker;

// For now, just re-export a stub that directs users to compile with the right features
// The proper solution would be to move shared code to a separate module
// This is a minimal implementation to allow compilation

pub async fn build_routers<T>(
    _config_manager: &kodegen_config::ConfigManager,
    _usage_tracker: &UsageTracker,
    _enabled_categories: &Option<HashSet<String>>,
) -> Result<(ToolRouter<T>, PromptRouter<T>)> {
    anyhow::bail!("Library mode not yet fully implemented - use the binary version. Compile mcp-backend with filesystem and other features enabled.")
}

// Minimal StdioServer export for type compatibility
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use rmcp::model::*;
use rmcp::service::RequestContext;
use serde_json::json;

#[derive(Clone)]
pub struct StdioServer {
    pub(crate) tool_router: ToolRouter<Self>,
    pub(crate) prompt_router: PromptRouter<Self>,
    pub(crate) usage_tracker: UsageTracker,
    pub(crate) config_manager: kodegen_config::ConfigManager,
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
                "KODEGEN MCP Server".to_string(),
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
        if let Err(e) = self.config_manager.set_client_info(request.client_info).await {
            log::warn!("Failed to store client info: {:?}", e);
        }
        
        Ok(self.get_info())
    }
}
