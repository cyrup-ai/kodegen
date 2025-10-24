# Cargo.toml Dependency Updates

## Step 1: Remove SweetMCP Dependencies

**Current lines to REMOVE:**
```toml
# SweetMCP dependencies for proper MCP implementation
sweetmcp-json-client = { path = "../sweetmcp/packages/json-client" }
sweetmcp-stdio-client = { path = "../sweetmcp/packages/stdio-client" }
mcp-client-traits = { path = "../sweetmcp/packages/mcp-client-traits" }
sweet_mcp_type = { path = "../sweetmcp/packages/sweet-mcp-type" }
```

## Step 2: Add Workspace MCP Dependencies

**Add to `[dependencies]` section:**
```toml
# Workspace MCP infrastructure
kodegen_mcp_tool = { path = "../mcp-tool" }
kodegen_mcp_client = { path = "../mcp-client" }
rmcp = { git = "https://github.com/anthropics/rmcp", branch = "main" }
```

**Rationale:**
- `kodegen_mcp_tool` - Core `Tool` trait for implementing local tools
- `kodegen_mcp_client` - `KodegenClient` for calling remote MCP servers
- `rmcp` - Official Anthropic MCP SDK (already used in workspace)

## Step 3: Verify Existing Compatible Dependencies

**Keep these (already present and compatible):**
```toml
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.145"
schemars = { version = "1.0", features = ["derive"] }
tokio = { version = "1.47.1", features = ["full"] }
anyhow = { version = "1.0.100" }
```

**Optional - Keep for other performance needs (not MCP):**
```toml
simd-json = { version = "0.16.0", default-features = false, features = [...] }
```
*Note: Can keep simd-json if used elsewhere, just not for MCP types*

## Step 4: Update Package Metadata

**Update `[package]` name to match workspace convention:**
```toml
[package]
name = "kodegen_candle_agent"  # Already correct ✓
```

## Step 5: Verify No Transitive Conflicts

**Check for any other crates that might depend on sweetMCP:**
```bash
cargo tree --package kodegen_candle_agent | grep -i sweet
cargo tree --package kodegen_candle_agent | grep -i mcp-client-traits
```

Expected: No matches after migration

## Step 6: Build Verification

**Test compilation after dependency changes:**
```bash
# From workspace root
cargo check --package kodegen_candle_agent

# Expected errors at this stage (before code refactor):
# - Cannot find type `ToolInfo` in crate `sweet_mcp_type`
# - Cannot find crate `mcp_client_traits`
# - etc.
```

## Complete Updated Dependencies Section

```toml
[dependencies]
bitflags = { version = "2.9.4", features = ["serde"] }
kodegen_simd = { path = "../simd" }
cylo = { path = "../cylo" }
ctrlc = { version = "3.4", features = ["termination"] }

# Workspace MCP infrastructure
kodegen_mcp_tool = { path = "../mcp-tool" }
kodegen_mcp_client = { path = "../mcp-client" }
rmcp = { git = "https://github.com/anthropics/rmcp", branch = "main" }

# JSON handling (keep simd-json for performance if needed elsewhere)
simd-json = { version = "0.16.0", default-features = false, features = ["known-key", "runtime-detection", "swar-number-parsing", "value-no-dup-keys"] }
value-trait = "0.11.0"

# ... rest of dependencies unchanged ...
```

## Notes

- **No feature flags needed** - workspace MCP packages don't require special features
- **No version conflicts** - `rmcp` is the same version used by workspace
- **Path dependencies** - Use relative paths for workspace packages
- **Git dependency** - `rmcp` uses git dependency like other workspace packages
