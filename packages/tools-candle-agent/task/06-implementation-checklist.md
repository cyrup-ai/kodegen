# Implementation Checklist

Use this checklist to track progress through the migration.

## Phase 1: Preparation ✓

- [x] Analyze sweetMCP usage patterns
- [x] Document type mappings
- [x] Create migration task files
- [x] Review workspace MCP infrastructure

## Phase 2: Cargo.toml Changes

- [ ] Remove sweetMCP dependencies
  - [ ] Remove `sweetmcp-json-client`
  - [ ] Remove `sweetmcp-stdio-client`
  - [ ] Remove `mcp-client-traits`
  - [ ] Remove `sweet_mcp_type`
- [ ] Add workspace MCP dependencies
  - [ ] Add `kodegen_mcp_tool = { path = "../mcp-tool" }`
  - [ ] Add `kodegen_mcp_client = { path = "../mcp-client" }`
  - [ ] Add `rmcp = { git = "...", branch = "main" }`
- [ ] Run `cargo check --package kodegen_candle_agent` (expect errors)

## Phase 3: Router Implementation

- [ ] Create new `CandleToolRouter` structure
  - [ ] Add `mcp_client: Option<KodegenClient>` field
  - [ ] Add `local_tools: Arc<RwLock<HashMap>>` field
  - [ ] Add `cylo_config: Option<CyloBackendConfig>` field
- [ ] Implement constructor methods
  - [ ] `new(mcp_client: Option<KodegenClient>)`
  - [ ] `with_cylo(backend_type, config)`
  - [ ] `register_tool<T: Tool>(tool)`
- [ ] Implement core methods
  - [ ] `get_available_tools() -> Vec<rmcp::model::Tool>`
  - [ ] `call_tool(name, args) -> Result<Value>`
  - [ ] `call_tool_stream(name, args) -> Stream`
- [ ] Port Cylo execution logic
  - [ ] Copy `execute_cylo()` from old router
  - [ ] Copy `create_cylo_tool_metadata()`
  - [ ] Copy `json_args_to_execution_request()`
  - [ ] Copy `execution_result_to_json()`
- [ ] Add MCP client integration
  - [ ] Remote tool listing
  - [ ] Remote tool execution
  - [ ] Error handling for client errors
- [ ] Remove `SweetMcpRouter` struct
- [ ] Remove `convert_sweet_json_to_serde()` function
- [ ] Update `RouterError` enum if needed

## Phase 4: Type Aliases & Re-exports

- [ ] Update `src/domain/tool/mod.rs`
  - [ ] Change `SweetMcpRouter` → `CandleToolRouter`
  - [ ] Remove `sweet_mcp_type` imports
  - [ ] Add `pub use rmcp::model::Tool as ToolInfo`
  - [ ] Add `pub use kodegen_mcp_tool::Tool`
  - [ ] Add `pub use kodegen_mcp_client::KodegenClient`

## Phase 5: Domain Module Updates

### src/domain/agent/role.rs
- [ ] Remove `sweet_mcp_type::JsonValue` import
- [ ] Remove `sweet_mcp_type::ToolInfo` import
- [ ] Add `serde_json::Value` import
- [ ] Add `rmcp::model::Tool as ToolInfo` import
- [ ] Delete `convert_serde_to_sweet_json()` function
- [ ] Delete `convert_sweet_json_to_serde()` function
- [ ] Update all function signatures to use `serde_json::Value`

### src/domain/agent/types.rs
- [ ] Change `ToolInfo` type alias
- [ ] Update `ToolArgs` trait if needed
- [ ] Update trait implementations

### src/domain/agent/core.rs
- [ ] Change `SweetMcpRouter` → `CandleToolRouter`
- [ ] Update `tool_router` field type
- [ ] Update router initialization
- [ ] Update tool calling code
- [ ] Update tool listing code

### src/domain/completion/types.rs
- [ ] Update `ToolInfo` re-export

### src/domain/completion/request.rs
- [ ] Update `ToolInfo` import
- [ ] Verify `tools` field usage

### src/domain/chat/orchestration.rs
- [ ] Update imports (JsonValue, ToolInfo, router type)
- [ ] Update function signatures
- [ ] Update tool metadata access patterns
- [ ] Handle Arc<Map> for schemas

## Phase 6: Prelude & Public API

- [ ] Update `src/lib.rs` prelude
  - [ ] Change router re-export
  - [ ] Update ToolInfo re-export
  - [ ] Add Tool trait re-export

## Phase 7: Build & Test

- [ ] Run `cargo check --package kodegen_candle_agent`
  - [ ] Fix any remaining compilation errors
  - [ ] Verify no warnings (except known num-bigint-dig)
- [ ] Run `cargo build --package kodegen_candle_agent`
- [ ] Run `cargo test --package kodegen_candle_agent`
  - [ ] Fix any test failures
  - [ ] Update test expectations if needed
- [ ] Test specific functionality
  - [ ] Cylo backend execution still works
  - [ ] Tool routing logic is correct
  - [ ] Error handling is appropriate

## Phase 8: Examples & Documentation

- [ ] Check example files in `examples/`
  - [ ] Update any SweetMCP references
  - [ ] Verify examples compile
  - [ ] Test examples run correctly
- [ ] Update inline documentation
  - [ ] Update doc comments referencing SweetMCP
  - [ ] Update TOOL_CALLING.md if exists
  - [ ] Update any README sections

## Phase 9: Integration Testing

- [ ] Test with agents
  - [ ] Create agent with CandleToolRouter
  - [ ] Call tools via agent
  - [ ] Verify tool results
- [ ] Test with chat loop
  - [ ] Run chat orchestration
  - [ ] Verify tool calling works
  - [ ] Check streaming responses
- [ ] Test Cylo execution
  - [ ] Execute Python code
  - [ ] Execute JavaScript code
  - [ ] Verify sandbox isolation

## Phase 10: Cleanup

- [ ] Remove any dead code
- [ ] Remove unused imports
- [ ] Remove commented-out sweetMCP code
- [ ] Run `cargo fmt --package kodegen_candle_agent`
- [ ] Run `cargo clippy --package kodegen_candle_agent`
- [ ] Address any clippy warnings

## Final Verification

- [ ] `cargo build --package kodegen_candle_agent --release`
- [ ] All tests pass
- [ ] All examples work
- [ ] No sweetMCP references remain (except in task docs)
- [ ] Performance is acceptable
- [ ] Error messages are clear
- [ ] Public API is ergonomic

## Success Criteria

✅ Zero sweetMCP dependencies in Cargo.toml
✅ Zero compilation errors
✅ Zero test failures
✅ All examples functional
✅ Tool routing works with workspace MCP
✅ Cylo execution preserved
✅ Agent/chat functionality maintained
