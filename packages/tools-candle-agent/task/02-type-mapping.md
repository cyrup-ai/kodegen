# Type Mapping: SweetMCP → Workspace MCP

## Core Type Conversions

### 1. ToolInfo Type
**SweetMCP:**
```rust
use sweet_mcp_type::ToolInfo;

pub struct ToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: JsonValue,  // sweet_mcp_type::JsonValue
}
```

**Workspace MCP:**
```rust
use rmcp::model::Tool;

pub struct Tool {
    pub name: Cow<'static, str>,
    pub title: Option<String>,
    pub description: Option<Cow<'static, str>>,
    pub input_schema: Arc<serde_json::Map<String, Value>>,
    pub output_schema: Option<Arc<serde_json::Map<String, Value>>>,
    pub annotations: Option<ToolAnnotations>,
    pub icons: Option<Vec<Icon>>,
}
```

**Migration Notes:**
- `ToolInfo` → `rmcp::model::Tool`
- Add `title` field (can be None)
- Wrap schemas in `Arc` for performance
- Add optional `annotations` for tool metadata
- `input_schema` changes from `JsonValue` to `Arc<Map<String, Value>>`

### 2. JsonValue Type
**SweetMCP:**
```rust
use sweet_mcp_type::JsonValue;  // simd_json::Value alias
use simd_json::StaticNode;

// enum with variants: Static, String, Array, Object
```

**Workspace MCP:**
```rust
use serde_json::Value;

// Standard serde_json Value (already used throughout workspace)
```

**Migration Notes:**
- Direct replacement: `sweet_mcp_type::JsonValue` → `serde_json::Value`
- Remove `simd_json` conversion utilities
- Can keep `simd-json` crate for other purposes, just not for MCP types

### 3. Tool Router
**SweetMCP:**
```rust
pub struct SweetMcpRouter {
    available_tools: Arc<RwLock<Vec<ToolInfo>>>,
    tool_routes: Arc<RwLock<HashMap<String, ToolRoute>>>,
    // ...
}

impl SweetMcpRouter {
    pub async fn call_tool(&self, name: &str, args: JsonValue) -> Result<Value, RouterError>;
    pub async fn get_available_tools(&self) -> Vec<ToolInfo>;
}
```

**Workspace MCP:**
```rust
// Option 1: Use KodegenClient directly
use kodegen_mcp_client::KodegenClient;

impl KodegenClient {
    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) 
        -> Result<CallToolResult, ClientError>;
    pub async fn list_tools(&self) -> Result<Vec<Tool>, ClientError>;
}

// Option 2: Create a ToolRouter that wraps KodegenClient
pub struct CandleToolRouter {
    mcp_client: Option<KodegenClient>,
    cylo_config: Option<CyloBackendConfig>,
    native_tools: Arc<RwLock<HashMap<String, Box<dyn Tool>>>>,
}
```

**Migration Strategy:**
- Replace `SweetMcpRouter` with custom `CandleToolRouter`
- Use `KodegenClient` for remote MCP server tools
- Keep Cylo backend execution logic
- Support both local (via Tool trait) and remote (via MCP client) tools

### 4. MCP Client Traits
**SweetMCP:**
```rust
use mcp_client_traits::{McpClient, McpToolOperations};

pub trait McpClient { /* ... */ }
pub trait McpToolOperations { /* ... */ }
```

**Workspace MCP:**
```rust
// No trait needed - use KodegenClient directly
use kodegen_mcp_client::KodegenClient;

// Or implement Tool trait for local tools
use kodegen_mcp_tool::Tool;
```

**Migration Notes:**
- Remove trait-based abstraction
- Use concrete `KodegenClient` type
- For local tools, implement `Tool` trait from `kodegen_mcp_tool`

## Conversion Functions

### JSON Conversion (to be removed)
```rust
// REMOVE: sweet_mcp_type specific conversions
fn convert_sweet_json_to_serde(value: JsonValue) -> serde_json::Value { /* ... */ }
fn convert_serde_to_sweet_json(value: Value) -> SweetJsonValue { /* ... */ }
```

**After Migration:**
- No conversion needed - use `serde_json::Value` everywhere
- Remove all conversion utilities
- Simplify JSON handling throughout codebase

## Files Requiring Updates

1. **`src/domain/tool/router.rs`** (Major refactor)
   - Replace `SweetMcpRouter` with `CandleToolRouter`
   - Use `serde_json::Value` for all JSON
   - Use `rmcp::model::Tool` for tool metadata

2. **`src/domain/tool/mod.rs`** (Simple)
   - Remove sweetMCP re-exports
   - Add workspace MCP re-exports

3. **`src/domain/agent/role.rs`** (Moderate)
   - Remove JSON conversion functions
   - Use `serde_json::Value` directly
   - Update `ToolInfo` references

4. **`src/domain/agent/types.rs`** (Simple)
   - Update `ToolInfo` type alias
   - Use `rmcp::model::Tool`

5. **`src/domain/agent/core.rs`** (Moderate)
   - Update router usage
   - Change from `SweetMcpRouter` to `CandleToolRouter`

6. **`src/domain/completion/types.rs`** (Simple)
   - Update re-exports
   - Use workspace types

7. **`src/domain/completion/request.rs`** (Simple)
   - Update `ToolInfo` usage
   - Use `rmcp::model::Tool`

8. **`src/domain/chat/orchestration.rs`** (Moderate)
   - Update tool calling logic
   - Use new router interface
