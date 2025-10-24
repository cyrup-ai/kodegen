# Router Refactoring: SweetMcpRouter → CandleToolRouter

## Overview
The `SweetMcpRouter` needs to be refactored to use workspace MCP infrastructure while preserving Cylo backend integration for code execution.

## Architecture Decision

### Hybrid Router Design
The new `CandleToolRouter` will support **three tool execution methods**:

1. **Remote MCP Tools** - Via `KodegenClient` (from other MCP servers)
2. **Local Tools** - Via `Tool` trait (built into candle-agent)
3. **Cylo Execution** - Direct code execution in sandboxes (preserve existing)

## New Router Structure

```rust
// File: src/domain/tool/router.rs

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde_json::Value;
use std::pin::Pin;
use tokio_stream::Stream;

use kodegen_mcp_client::KodegenClient;
use kodegen_mcp_tool::Tool;
use rmcp::model::Tool as RmcpTool;
use cylo::{BackendConfig, Cylo, ExecutionRequest, ExecutionResult, create_backend};

use crate::domain::context::chunks::CandleJsonChunk;

/// Candle Tool Router - Unified routing for local, remote, and Cylo tools
#[derive(Clone)]
pub struct CandleToolRouter {
    /// Remote MCP client (optional - for connecting to external MCP servers)
    mcp_client: Option<KodegenClient>,
    
    /// Local tools registered via Tool trait
    local_tools: Arc<RwLock<HashMap<String, Arc<dyn ToolExecutor>>>>,
    
    /// Cylo backend configuration for code execution
    cylo_config: Option<CyloBackendConfig>,
}

/// Tool execution strategy
#[derive(Debug, Clone)]
enum ToolRoute {
    /// Remote MCP server tool
    Remote,
    /// Local tool via Tool trait
    Local,
    /// Cylo code execution
    Cylo { backend_type: String, config: String },
}

/// Unified tool executor trait (internal)
trait ToolExecutor: Send + Sync {
    fn execute(&self, args: Value) -> Pin<Box<dyn std::future::Future<Output = Result<Value, RouterError>> + Send>>;
}

/// Router error types
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
}

impl CandleToolRouter {
    /// Create new router with optional MCP client
    pub fn new(mcp_client: Option<KodegenClient>) -> Self {
        Self {
            mcp_client,
            local_tools: Arc::new(RwLock::new(HashMap::new())),
            cylo_config: None,
        }
    }
    
    /// Configure Cylo backend for code execution
    pub fn with_cylo(mut self, backend_type: String, config: String) -> Self {
        self.cylo_config = Some(CyloBackendConfig { backend_type, config_value: config });
        self
    }
    
    /// Register a local tool
    pub fn register_tool<T>(&self, tool: T)
    where
        T: Tool + 'static,
    {
        let executor = Arc::new(LocalToolWrapper::new(tool));
        self.local_tools.write().insert(T::name().to_string(), executor);
    }
    
    /// List all available tools (local + remote + cylo)
    pub async fn get_available_tools(&self) -> Vec<RmcpTool> {
        let mut tools = Vec::new();
        
        // Add local tools
        for (name, executor) in self.local_tools.read().iter() {
            tools.push(executor.metadata());
        }
        
        // Add remote MCP tools
        if let Some(client) = &self.mcp_client {
            if let Ok(remote_tools) = client.list_tools().await {
                tools.extend(remote_tools);
            }
        }
        
        // Add Cylo execution tools if configured
        if self.cylo_config.is_some() {
            tools.extend(self.create_cylo_tool_metadata());
        }
        
        tools
    }
    
    /// Execute a tool by name
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, RouterError> {
        // Try local tools first
        if let Some(executor) = self.local_tools.read().get(name).cloned() {
            return executor.execute(args).await;
        }
        
        // Try remote MCP client
        if let Some(client) = &self.mcp_client {
            match client.call_tool(name, args.clone()).await {
                Ok(result) => return Ok(self.call_result_to_json(result)),
                Err(kodegen_mcp_client::ClientError::ServiceError(_)) => {
                    // Tool might not exist on remote - try Cylo
                }
                Err(e) => return Err(RouterError::McpClientError(e.to_string())),
            }
        }
        
        // Try Cylo execution (for execute_* tools)
        if name.starts_with("execute_") && self.cylo_config.is_some() {
            return self.execute_cylo(name, args).await;
        }
        
        Err(RouterError::ToolNotFound(name.to_string()))
    }
    
    /// Execute tool and return stream
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
    
    // Private helper methods
    
    fn execute_cylo(&self, name: &str, args: Value) -> /* impl Future */ { /* ... */ }
    fn create_cylo_tool_metadata(&self) -> Vec<RmcpTool> { /* ... */ }
    fn call_result_to_json(&self, result: rmcp::model::CallToolResult) -> Value { /* ... */ }
}

// Helper wrapper for local tools
struct LocalToolWrapper<T: Tool> {
    tool: T,
}

impl<T: Tool> ToolExecutor for LocalToolWrapper<T> {
    fn execute(&self, args: Value) -> Pin<Box<dyn std::future::Future<Output = Result<Value, RouterError>> + Send>> {
        // Deserialize args and execute tool
        // ...
    }
}
```

## Migration Steps

### 1. Create New File Structure
- Keep `src/domain/tool/router.rs` (rewrite contents)
- Keep `src/domain/tool/mod.rs` (update exports)

### 2. Implement CandleToolRouter
- Copy Cylo execution logic from old `SweetMcpRouter`
- Add `KodegenClient` integration for remote tools
- Add local tool registration via `Tool` trait

### 3. Update Public API
```rust
// src/domain/tool/mod.rs

pub mod router;

pub use router::{CandleToolRouter, RouterError};
pub use kodegen_mcp_tool::Tool;
pub use rmcp::model::Tool as ToolInfo;  // Type alias for compatibility
```

### 4. Update Usage Sites
- `src/domain/agent/core.rs` - Change `SweetMcpRouter` to `CandleToolRouter`
- `src/domain/chat/orchestration.rs` - Update tool calling

## Backwards Compatibility

### Type Alias for ToolInfo
```rust
// Maintain compatibility with existing code
pub type ToolInfo = rmcp::model::Tool;
```

### Builder Pattern Preservation
```rust
impl CandleToolRouter {
    pub fn new() -> Self { /* ... */ }
    pub fn with_mcp_client(mut self, client: KodegenClient) -> Self { /* ... */ }
    pub fn with_cylo(mut self, backend: String, config: String) -> Self { /* ... */ }
}
```

## Testing Strategy
1. Unit test each execution path (local, remote, cylo)
2. Integration test with mock MCP server
3. Verify Cylo backend still works
4. Test error handling for each path
