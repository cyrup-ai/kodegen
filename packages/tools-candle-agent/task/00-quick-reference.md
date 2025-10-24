# Quick Reference - SweetMCP Migration

## TL;DR - One-Pager

### Dependencies Change
```toml
# REMOVE these 4 lines:
sweetmcp-json-client = { path = "../sweetmcp/packages/json-client" }
sweetmcp-stdio-client = { path = "../sweetmcp/packages/stdio-client" }
mcp-client-traits = { path = "../sweetmcp/packages/mcp-client-traits" }
sweet_mcp_type = { path = "../sweetmcp/packages/sweet-mcp-type" }

# ADD these 3 lines:
kodegen_mcp_tool = { path = "../mcp-tool" }
kodegen_mcp_client = { path = "../mcp-client" }
rmcp = { git = "https://github.com/anthropics/rmcp", branch = "main" }
```

### Type Replacements (Find & Replace)

| Find | Replace |
|------|---------|
| `sweet_mcp_type::JsonValue` | `serde_json::Value` |
| `sweet_mcp_type::ToolInfo` | `rmcp::model::Tool` |
| `SweetMcpRouter` | `CandleToolRouter` |
| `use mcp_client_traits::` | `use kodegen_mcp_client::` |

### Functions to Delete
- `convert_serde_to_sweet_json()` in `src/domain/agent/role.rs`
- `convert_sweet_json_to_serde()` in `src/domain/agent/role.rs`
- `convert_sweet_json_to_serde()` in `src/domain/tool/router.rs`

### Key Files to Refactor (8 files)

1. **`src/domain/tool/router.rs`** - Major rewrite (SweetMcpRouter → CandleToolRouter)
2. **`src/domain/tool/mod.rs`** - Update re-exports
3. **`src/domain/agent/role.rs`** - Remove JSON conversion utilities
4. **`src/domain/agent/types.rs`** - Update ToolInfo type alias
5. **`src/domain/agent/core.rs`** - Update router usage
6. **`src/domain/completion/types.rs`** - Update re-exports
7. **`src/domain/completion/request.rs`** - Update imports
8. **`src/domain/chat/orchestration.rs`** - Update tool calling

### New Router API

```rust
// Old
let router = SweetMcpRouter::new();

// New
let router = CandleToolRouter::new(None)  // No remote MCP client
    .with_cylo("Apple".to_string(), "swift:latest".to_string());

// With remote MCP client
let client = kodegen_mcp_client::create_sse_client(url).await?;
let router = CandleToolRouter::new(Some(client.client()))
    .with_cylo("Apple".to_string(), "swift:latest".to_string());
```

### Tool Metadata Changes

```rust
// Old: sweet_mcp_type::ToolInfo
struct ToolInfo {
    name: String,
    description: Option<String>,
    input_schema: JsonValue,
}

// New: rmcp::model::Tool
struct Tool {
    name: Cow<'static, str>,          // Changed
    title: Option<String>,             // New field
    description: Option<Cow<'static, str>>,  // Changed
    input_schema: Arc<Map<String, Value>>,   // Changed (Arc-wrapped)
    output_schema: Option<Arc<Map<String, Value>>>,  // New field
    annotations: Option<ToolAnnotations>,    // New field
    icons: Option<Vec<Icon>>,          // New field
}
```

### Execution Flow

**Before (SweetMCP):**
```
User → Agent → SweetMcpRouter → [SweetMCP Plugin | Cylo Backend]
```

**After (Workspace MCP):**
```
User → Agent → CandleToolRouter → [Local Tool | Remote MCP | Cylo Backend]
                                       ↓            ↓
                                   Tool trait   KodegenClient
```

### Test Command Sequence

```bash
# 1. Update Cargo.toml
# 2. Check compilation (expect errors)
cargo check --package kodegen_candle_agent

# 3. Refactor code files (use checklist)
# 4. Verify compilation
cargo check --package kodegen_candle_agent

# 5. Build
cargo build --package kodegen_candle_agent

# 6. Test
cargo test --package kodegen_candle_agent

# 7. Lint
cargo clippy --package kodegen_candle_agent
```

## Task Files

Read in order:
1. **01-migration-overview.md** - Big picture, strategy, success criteria
2. **02-type-mapping.md** - Detailed type conversions and rationale
3. **03-cargo-toml-updates.md** - Dependency changes
4. **04-router-refactor.md** - New router architecture and design
5. **05-code-changes.md** - File-by-file refactoring guide
6. **06-implementation-checklist.md** - Step-by-step progress tracking

## Common Pitfalls

❌ **Don't** keep both sweetMCP and workspace MCP dependencies
❌ **Don't** try to convert between JSON types (use serde_json::Value everywhere)
❌ **Don't** forget to dereference Arc when accessing schemas (`&*tool.input_schema`)
❌ **Don't** forget to update test files
❌ **Don't** change Cylo execution logic (preserve as-is)

✅ **Do** update imports before refactoring logic
✅ **Do** compile after each major file change
✅ **Do** use type aliases for backwards compatibility
✅ **Do** test Cylo backend after migration
✅ **Do** update examples and documentation

## Estimated Time

- **Cargo.toml changes**: 5 minutes
- **Router refactor**: 2-3 hours
- **Domain module updates**: 1-2 hours
- **Testing & fixes**: 1-2 hours
- **Documentation**: 30 minutes

**Total**: ~5-8 hours for complete migration

## Success Indicators

When done correctly:
- ✅ `cargo check` passes with 0 errors
- ✅ `cargo build` succeeds
- ✅ All tests pass
- ✅ No references to `sweet_mcp_type` or `mcp_client_traits` in src/
- ✅ Cylo code execution still works
- ✅ Tool routing functions correctly
- ✅ Examples compile and run

## Questions?

Refer to specific task files for detailed guidance:
- Architecture questions → `04-router-refactor.md`
- Type confusion → `02-type-mapping.md`
- Specific file changes → `05-code-changes.md`
- Track progress → `06-implementation-checklist.md`
