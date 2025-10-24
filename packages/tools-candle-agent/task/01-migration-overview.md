# SweetMCP to Workspace MCP Migration - Overview

## Goal
Remove external sweetMCP dependencies from `tools-candle-agent` and integrate it into the workspace's unified MCP infrastructure using `packages/mcp-client` and `packages/mcp-tool`.

## Current State

### SweetMCP Dependencies (to be removed)
```toml
sweetmcp-json-client = { path = "../sweetmcp/packages/json-client" }
sweetmcp-stdio-client = { path = "../sweetmcp/packages/stdio-client" }
mcp-client-traits = { path = "../sweetmcp/packages/mcp-client-traits" }
sweet_mcp_type = { path = "../sweetmcp/packages/sweet-mcp-type" }
```

### Files Using SweetMCP (64 matches found)
1. `src/domain/tool/router.rs` - **Primary**: `SweetMcpRouter` implementation
2. `src/domain/tool/mod.rs` - Re-exports MCP traits and types
3. `src/domain/agent/role.rs` - JSON conversion utilities, `ToolInfo`
4. `src/domain/agent/types.rs` - `ToolInfo` type alias
5. `src/domain/agent/core.rs` - Uses `SweetMcpRouter` for tool execution
6. `src/domain/completion/types.rs` - Re-exports `ToolInfo`
7. `src/domain/completion/request.rs` - Uses `ToolInfo`
8. `src/domain/chat/orchestration.rs` - Tool calling orchestration with `SweetMcpRouter`

## Workspace MCP Infrastructure

### Available Packages
- **`packages/mcp-tool`** - Core `Tool` trait, `McpError`, tool history tracking
- **`packages/mcp-client`** - `KodegenClient` for MCP operations
- **`packages/mcp-server`** - Main server binary with tool router

### Key Types & Traits
- `kodegen_mcp_tool::Tool` - Tool trait with execute/prompt methods
- `kodegen_mcp_tool::McpError` - Error type for tool operations
- `kodegen_mcp_client::KodegenClient` - Client for calling MCP tools
- `rmcp::model::Tool` - RMCP tool metadata (name, description, schema)
- `rmcp::model::CallToolResult` - Tool execution result

## Migration Strategy

### Phase 1: Type Mapping & Analysis
- Map sweetMCP types to workspace equivalents
- Identify all usage patterns
- Document conversion requirements

### Phase 2: Cargo.toml Updates
- Remove sweetMCP dependencies
- Add workspace MCP dependencies
- Update feature flags if needed

### Phase 3: Code Refactoring
- Replace `sweet_mcp_type::JsonValue` with `serde_json::Value`
- Replace `sweet_mcp_type::ToolInfo` with `rmcp::model::Tool`
- Replace `SweetMcpRouter` with `KodegenClient` or custom router
- Update JSON conversion utilities
- Remove MCP client trait implementations

### Phase 4: Integration & Testing
- Ensure compilation succeeds
- Test tool routing and execution
- Verify Cylo backend integration still works
- Update examples if needed

## Success Criteria
✅ All sweetMCP dependencies removed from Cargo.toml
✅ No compilation errors
✅ Tool routing works with workspace MCP infrastructure
✅ Cylo backend execution preserved
✅ Existing agent/chat functionality maintained
