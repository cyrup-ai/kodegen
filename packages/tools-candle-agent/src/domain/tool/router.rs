//! Candle Tool Router
//!
//! This module provides the unified tool routing interface for the chat loop architecture,
//! providing transparent routing between local tools, remote MCP servers, and Cylo execution.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::domain::context::chunks::CandleJsonChunk;
use cylo::{BackendConfig, Cylo, ExecutionRequest, ExecutionResult, create_backend};
use kodegen_mcp_client::KodegenClient;
use rmcp::model::Tool as RmcpTool;

/// Candle Tool Router
///
/// Provides transparent tool routing for the chat loop architecture.
/// Supports three execution methods: local tools, remote MCP, and Cylo backends.
#[derive(Clone)]
pub struct CandleToolRouter {
    /// Remote MCP client (optional - for connecting to external MCP servers)
    mcp_client: Option<KodegenClient>,

    /// Local tools registered via Tool trait
    local_tools: Arc<RwLock<HashMap<String, Arc<dyn ToolExecutor>>>>,

    /// Cylo backend configuration for code execution
    cylo_config: Option<CyloBackendConfig>,

    /// Tool routing map: `tool_name` -> execution strategy
    tool_routes: Arc<RwLock<HashMap<String, ToolRoute>>>,
}

/// Tool execution route strategy
#[derive(Debug, Clone)]
enum ToolRoute {
    /// Local tool via Tool trait
    Local,
    /// Cylo code execution
    Cylo {
        backend_type: String,
        config: String,
    },
}

/// Configuration for Cylo execution backend
#[derive(Debug, Clone)]
pub struct CyloBackendConfig {
    pub backend_type: String, // "Apple", "LandLock", "FireCracker"
    pub config_value: String, // e.g., "python:alpine3.20" or "/tmp/sandbox"
}

/// Tool execution error types
#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Backend error: {0}")]
    BackendError(String),
    #[error("MCP client error: {0}")]
    McpClientError(String),
    #[error("Tool error: {0}")]
    ToolError(String),
}

/// Internal trait for executing tools with type erasure
trait ToolExecutor: Send + Sync {
    fn metadata(&self) -> RmcpTool;
    fn execute(
        &self,
        args: Value,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Value, RouterError>> + Send>>;
}

/// Wrapper for Tool trait implementations
struct ToolWrapper<T: kodegen_mcp_tool::Tool> {
    tool: Arc<T>,
}

impl<T: kodegen_mcp_tool::Tool> ToolWrapper<T> {
    fn new(tool: T) -> Self {
        Self {
            tool: Arc::new(tool),
        }
    }
}

impl<T: kodegen_mcp_tool::Tool> ToolExecutor for ToolWrapper<T> {
    fn metadata(&self) -> RmcpTool {
        use rmcp::model::ToolAnnotations;

        RmcpTool {
            name: Cow::Borrowed(T::name()),
            title: None,
            description: Some(Cow::Borrowed(T::description())),
            input_schema: T::input_schema(),
            output_schema: T::output_schema(),
            annotations: Some(
                ToolAnnotations::new()
                    .read_only(T::read_only())
                    .destructive(T::destructive())
                    .idempotent(T::idempotent())
                    .open_world(T::open_world()),
            ),
            icons: None,
        }
    }

    fn execute(
        &self,
        args: Value,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Value, RouterError>> + Send>> {
        // Deserialize args to tool's Args type
        let typed_args: Result<T::Args, _> = serde_json::from_value(args);

        match typed_args {
            Ok(args) => {
                let tool = Arc::clone(&self.tool);
                Box::pin(async move {
                    tool.execute(args)
                        .await
                        .map_err(|e| RouterError::ToolError(e.to_string()))
                })
            }
            Err(e) => Box::pin(async move {
                Err(RouterError::InvalidArguments(format!(
                    "Failed to deserialize arguments: {e}"
                )))
            }),
        }
    }
}

impl CandleToolRouter {
    /// Create new router with optional MCP client
    #[must_use]
    pub fn new(mcp_client: Option<KodegenClient>) -> Self {
        Self {
            mcp_client,
            local_tools: Arc::new(RwLock::new(HashMap::new())),
            cylo_config: None,
            tool_routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Configure Cylo backend for code execution
    #[must_use]
    pub fn with_cylo(mut self, backend_type: String, config: String) -> Self {
        self.cylo_config = Some(CyloBackendConfig {
            backend_type,
            config_value: config,
        });
        self
    }

    /// Register a local tool
    pub fn register_tool<T>(&self, tool: T)
    where
        T: kodegen_mcp_tool::Tool + 'static,
    {
        let name = T::name().to_string();
        let executor: Arc<dyn ToolExecutor> = Arc::new(ToolWrapper::new(tool));
        self.local_tools.write().insert(name.clone(), executor);
        self.tool_routes.write().insert(name, ToolRoute::Local);
    }

    /// Initialize router by discovering available tools
    ///
    /// This adds Cylo execution tools if configured.
    ///
    /// # Errors
    /// Currently never returns an error, but signature allows for future async initialization
    #[allow(clippy::unused_async)]
    pub async fn initialize(&mut self) -> Result<(), RouterError> {
        // Add native code execution tools if Cylo is configured
        self.add_native_execution_tools();
        Ok(())
    }

    /// List all available tools (local + remote + cylo)
    pub async fn get_available_tools(&self) -> Vec<RmcpTool> {
        let mut tools = Vec::new();

        // Add local tools
        for (_name, executor) in self.local_tools.read().iter() {
            tools.push(executor.metadata());
        }

        // Add remote MCP tools
        if let Some(client) = &self.mcp_client
            && let Ok(remote_tools) = client.list_tools().await
        {
            tools.extend(remote_tools);
        }

        // Add Cylo execution tools if configured
        if self.cylo_config.is_some() {
            tools.extend(Self::create_cylo_tool_metadata());
        }

        tools
    }

    /// Execute a tool by name
    ///
    /// # Errors
    /// Returns error if tool not found, execution fails, or invalid arguments provided
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, RouterError> {
        // Try local tools first
        let executor = self.local_tools.read().get(name).cloned();
        if let Some(executor) = executor {
            return executor.execute(args).await;
        }

        // Try remote MCP client
        if let Some(client) = &self.mcp_client {
            match client.call_tool(name, args.clone()).await {
                Ok(result) => return Self::call_result_to_json(&result),
                Err(kodegen_mcp_client::ClientError::ServiceError(_)) => {
                    // Tool might not exist on remote - try Cylo
                }
                Err(e) => return Err(RouterError::McpClientError(e.to_string())),
            }
        }

        // Try Cylo execution (for execute_* tools)
        if name.starts_with("execute_") && self.cylo_config.is_some() {
            // Get the route to know backend config
            let route = self.tool_routes.read().get(name).cloned();
            if let Some(ToolRoute::Cylo {
                backend_type,
                config,
            }) = route
            {
                return self
                    .execute_cylo_backend(&backend_type, &config, args)
                    .await;
            }
        }

        Err(RouterError::ToolNotFound(name.to_string()))
    }

    /// Execute tool and return stream
    #[must_use]
    pub fn call_tool_stream(
        &self,
        tool_name: &str,
        args: Value,
    ) -> Pin<Box<dyn Stream<Item = CandleJsonChunk> + Send>> {
        let router = self.clone();
        let tool_name = tool_name.to_string();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            tokio::spawn(async move {
                match router.call_tool(&tool_name, args).await {
                    Ok(result) => {
                        let _ = tx.send(CandleJsonChunk(result));
                    }
                    Err(e) => {
                        let error = serde_json::json!({"error": e.to_string()});
                        let _ = tx.send(CandleJsonChunk(error));
                    }
                }
            });
        }))
    }

    // ========================================================================
    // CYLO EXECUTION LOGIC (PRESERVED FROM ORIGINAL)
    // ========================================================================

    /// Add native code execution tools via Cylo (if configured)
    fn add_native_execution_tools(&self) {
        // Only add native execution tools if Cylo backend is configured
        let Some(cylo_config) = &self.cylo_config else {
            return; // No Cylo configured, skip native execution tools
        };

        // Add native execution tools for different languages
        let languages = vec![
            ("execute_python", "Python"),
            ("execute_javascript", "JavaScript"),
            ("execute_rust", "Rust"),
            ("execute_bash", "Bash"),
            ("execute_go", "Go"),
        ];

        let mut routes = self.tool_routes.write();
        for (tool_name, _language) in languages {
            // Use user-configured Cylo backend
            routes.insert(
                tool_name.to_string(),
                ToolRoute::Cylo {
                    backend_type: cylo_config.backend_type.clone(),
                    config: cylo_config.config_value.clone(),
                },
            );
        }
    }

    /// Create Cylo tool metadata
    fn create_cylo_tool_metadata() -> Vec<RmcpTool> {
        let languages = vec![
            ("execute_python", "Python"),
            ("execute_javascript", "JavaScript"),
            ("execute_rust", "Rust"),
            ("execute_bash", "Bash"),
            ("execute_go", "Go"),
        ];

        languages
            .into_iter()
            .map(|(tool_name, language)| RmcpTool {
                name: Cow::Owned(tool_name.to_string()),
                title: None,
                description: Some(Cow::Owned(format!(
                    "Execute {language} code securely via Cylo"
                ))),
                input_schema: Self::create_code_execution_schema(&format!(
                    "{language} code to execute"
                )),
                output_schema: None,
                annotations: None,
                icons: None,
            })
            .collect()
    }

    /// Execute via Cylo backend directly
    async fn execute_cylo_backend(
        &self,
        backend_type: &str,
        config: &str,
        args: Value,
    ) -> Result<Value, RouterError> {
        // Create appropriate Cylo environment
        let cylo_env = match backend_type {
            "Apple" => Cylo::Apple(config.to_string()),
            "LandLock" => Cylo::LandLock(config.to_string()),
            "FireCracker" => Cylo::FireCracker(config.to_string()),
            "SweetMcpPlugin" => Cylo::SweetMcpPlugin(config.to_string()),
            _ => {
                return Err(RouterError::BackendError(format!(
                    "Unknown backend type: {backend_type}"
                )));
            }
        };

        let backend_config = BackendConfig::new(backend_type);
        let backend = create_backend(&cylo_env, backend_config)
            .map_err(|e| RouterError::BackendError(e.to_string()))?;

        // Convert Value args to ExecutionRequest
        let request = Self::json_args_to_execution_request(&args)?;

        // Execute via backend
        let result_handle = backend.execute_code(request);
        let result = result_handle
            .await
            .map_err(|e| RouterError::ExecutionFailed(e.to_string()))?;

        // Convert ExecutionResult to JSON Value
        Ok(Self::execution_result_to_json(&result))
    }

    /// Convert Value arguments to `ExecutionRequest`
    fn json_args_to_execution_request(args: &Value) -> Result<ExecutionRequest, RouterError> {
        let code = args
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RouterError::InvalidArguments("Missing 'code' parameter".to_string()))?;

        let language = args
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("python");

        let mut request = ExecutionRequest::new(code, language);

        // Add optional parameters
        if let Some(input) = args.get("input").and_then(|v| v.as_str()) {
            request = request.with_input(input);
        }

        if let Some(env_obj) = args.get("env").and_then(|v| v.as_object()) {
            for (key, value) in env_obj {
                if let Some(val_str) = value.as_str() {
                    request = request.with_env(key.clone(), val_str);
                }
            }
        }

        Ok(request)
    }

    /// Convert `ExecutionResult` to JSON Value
    fn execution_result_to_json(result: &ExecutionResult) -> Value {
        serde_json::json!({
            "success": result.exit_code == 0,
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "duration_ms": result.duration.as_millis(),
            "resource_usage": {
                "peak_memory": result.resource_usage.peak_memory,
                "cpu_time_ms": result.resource_usage.cpu_time_ms,
                "process_count": result.resource_usage.process_count,
            }
        })
    }

    /// Create a code execution input schema
    fn create_code_execution_schema(description: &str) -> Arc<serde_json::Map<String, Value>> {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": description
                },
                "language": {
                    "type": "string",
                    "enum": ["python", "javascript", "rust", "bash", "go"],
                    "default": "python",
                    "description": "Programming language"
                }
            },
            "required": ["code"]
        });

        // Convert to Map and wrap in Arc
        if let Value::Object(map) = schema {
            Arc::new(map)
        } else {
            Arc::new(serde_json::Map::new())
        }
    }

    // ========================================================================
    // HELPER METHODS
    // ========================================================================

    /// Convert `CallToolResult` to JSON Value
    fn call_result_to_json(result: &rmcp::model::CallToolResult) -> Result<Value, RouterError> {
        // Extract text content from result
        let text_content = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .ok_or_else(|| {
                RouterError::ExecutionFailed("No text content in MCP response".to_string())
            })?;

        // Try to parse as JSON, or return as string if not JSON
        serde_json::from_str(&text_content.text)
            .or_else(|_| Ok(serde_json::json!({"result": text_content.text})))
    }
}

impl Default for CandleToolRouter {
    fn default() -> Self {
        Self::new(None)
    }
}
