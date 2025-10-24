# SweetMCP Migration Task Documentation

This directory contains comprehensive documentation for migrating `tools-candle-agent` from external sweetMCP dependencies to the workspace's unified MCP infrastructure.

## 📋 Task Files

### Start Here
**[00-quick-reference.md](./00-quick-reference.md)** - One-page summary with TL;DR, common patterns, and success criteria

### Detailed Guides (Read in Order)
1. **[01-migration-overview.md](./01-migration-overview.md)** - Project overview, current state, migration strategy
2. **[02-type-mapping.md](./02-type-mapping.md)** - Detailed type conversions between sweetMCP and workspace MCP
3. **[03-cargo-toml-updates.md](./03-cargo-toml-updates.md)** - Dependency changes and Cargo.toml modifications
4. **[04-router-refactor.md](./04-router-refactor.md)** - New CandleToolRouter architecture and implementation design
5. **[05-code-changes.md](./05-code-changes.md)** - File-by-file refactoring guide with code examples
6. **[06-implementation-checklist.md](./06-implementation-checklist.md)** - Comprehensive step-by-step checklist

## 🎯 Quick Start

```bash
# 1. Read the quick reference
cat task/00-quick-reference.md

# 2. Start with Cargo.toml changes
# Follow: task/03-cargo-toml-updates.md

# 3. Refactor the router
# Follow: task/04-router-refactor.md

# 4. Update all domain modules
# Follow: task/05-code-changes.md

# 5. Use checklist to track progress
# Follow: task/06-implementation-checklist.md
```

## 🔑 Key Changes

### Dependencies
- ❌ Remove: 4 sweetMCP packages
- ✅ Add: 3 workspace packages (`kodegen_mcp_tool`, `kodegen_mcp_client`, `rmcp`)

### Types
- `sweet_mcp_type::JsonValue` → `serde_json::Value`
- `sweet_mcp_type::ToolInfo` → `rmcp::model::Tool`
- `SweetMcpRouter` → `CandleToolRouter`

### Files Affected
8 core files need updates:
1. `src/domain/tool/router.rs` (major rewrite)
2. `src/domain/tool/mod.rs`
3. `src/domain/agent/role.rs`
4. `src/domain/agent/types.rs`
5. `src/domain/agent/core.rs`
6. `src/domain/completion/types.rs`
7. `src/domain/completion/request.rs`
8. `src/domain/chat/orchestration.rs`

## 📊 Migration Phases

| Phase | Description | Files | Est. Time |
|-------|-------------|-------|-----------|
| 1 | Cargo.toml updates | 1 | 5 min |
| 2 | Router refactor | 1 | 2-3 hrs |
| 3 | Domain updates | 7 | 1-2 hrs |
| 4 | Testing & fixes | All | 1-2 hrs |
| 5 | Cleanup | All | 30 min |

**Total**: 5-8 hours

## ✅ Success Criteria

- [ ] Zero sweetMCP dependencies in Cargo.toml
- [ ] Zero compilation errors
- [ ] All tests passing
- [ ] Tool routing functional
- [ ] Cylo backend preserved
- [ ] Examples working
- [ ] Documentation updated

## 🚀 New Capabilities After Migration

### Workspace Integration
- ✅ Use any tool from other workspace packages
- ✅ Call remote MCP servers via KodegenClient
- ✅ Register local tools via Tool trait
- ✅ Unified error handling with McpError
- ✅ Tool history tracking
- ✅ Schema caching for performance

### Architecture Benefits
- **Three execution modes**: Local tools, Remote MCP, Cylo backends
- **Type safety**: Strong types from workspace infrastructure
- **Performance**: Schema caching, Arc-based sharing
- **Observability**: Tool history and metrics
- **Flexibility**: Easy to add new tool sources

## 📝 Notes

- **Preserve Cylo logic**: The migration should not change Cylo backend execution
- **Backwards compatibility**: Type aliases maintain API compatibility where possible
- **No simd-json for MCP**: Use standard serde_json for all MCP types
- **Tool trait**: Optional - can use remote MCP servers without implementing Tool trait

## 🐛 Common Issues & Solutions

### Issue: Cannot find type `ToolInfo`
**Solution**: Add `use rmcp::model::Tool as ToolInfo;`

### Issue: Type mismatch with `input_schema`
**Solution**: Wrap in Arc: `Arc::new(schema)` and dereference when needed: `&*tool.input_schema`

### Issue: Conversion function not found
**Solution**: Remove conversion - use `serde_json::Value` directly everywhere

### Issue: SweetMcpRouter not found
**Solution**: Update to `CandleToolRouter` and follow 04-router-refactor.md

## 📚 Additional Resources

- Workspace Tool trait: `../../mcp-tool/src/tool.rs`
- Workspace Client: `../../mcp-client/src/lib.rs`
- RMCP SDK: https://github.com/anthropics/rmcp
- Project CLAUDE.md: `../../CLAUDE.md`

## 💡 Tips

1. **Incremental approach**: Update one file at a time, compile after each change
2. **Use checklist**: Track progress in `06-implementation-checklist.md`
3. **Test frequently**: Run `cargo check` after each significant change
4. **Preserve Cylo**: Don't modify Cylo execution logic - just migrate the wrapper
5. **Type aliases**: Use `pub type ToolInfo = rmcp::model::Tool` for compatibility

---

**Created**: 2025-10-23  
**Purpose**: Guide migration from sweetMCP to workspace MCP infrastructure  
**Estimated completion**: 5-8 hours  
**Status**: Ready for implementation
