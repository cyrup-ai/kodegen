# Code Changes - File by File Migration Guide

## File 1: src/domain/tool/mod.rs

### Current Code
```rust
pub mod router;

// Re-export SweetMCP types
pub use router::{SweetMcpRouter, RouterError, ToolRoute, ToolInfo};

// Re-export MCP client traits
pub use mcp_client_traits::{McpClient, McpToolOperations};
pub use sweet_mcp_type::ToolInfo;
```

### Updated Code
```rust
pub mod router;

// Re-export workspace MCP types
pub use router::{CandleToolRouter, RouterError};
pub use kodegen_mcp_tool::Tool;
pub use kodegen_mcp_client::KodegenClient;
pub use rmcp::model::Tool as ToolInfo;  // Type alias for compatibility
```

---

## File 2: src/domain/agent/role.rs

### Changes Required

**Remove these imports:**
```rust
use sweet_mcp_type::{JsonValue, ToolInfo};
use simd_json::value::owned::Object as JsonObject;
```

**Add these imports:**
```rust
use serde_json::Value as JsonValue;
use rmcp::model::Tool as ToolInfo;
```

**Remove these conversion functions:**
```rust
// DELETE: convert_sweet_json_to_serde
pub fn convert_sweet_json_to_serde(value: SweetJsonValue) -> Value { /* ... */ }

// DELETE: convert_serde_to_sweet_json  
pub fn convert_serde_to_sweet_json(value: Value) -> SweetJsonValue { /* ... */ }
```

**Update all function signatures:**
- Replace `SweetJsonValue` → `serde_json::Value`
- Remove all conversion calls
- Simplify JSON handling (no conversion needed)

---

## File 3: src/domain/agent/types.rs

### Current Code
```rust
// Re-export from sweet_mcp_type
pub use sweet_mcp_type::ToolInfo;
```

### Updated Code
```rust
// Re-export from workspace MCP
pub use rmcp::model::Tool as ToolInfo;
```

### Update ToolArgs Trait
```rust
pub trait ToolArgs {
    fn add_to(self, tools: &mut ZeroOneOrMany<ToolInfo>);
}

// Update implementations to use rmcp::model::Tool
impl ToolArgs for ToolInfo {
    fn add_to(self, tools: &mut ZeroOneOrMany<ToolInfo>) {
        tools.push(self);
    }
}
```

---

## File 4: src/domain/agent/core.rs

### Changes Required

**Update imports:**
```rust
// OLD
use crate::domain::tool::SweetMcpRouter;
use sweet_mcp_type::ToolInfo;

// NEW
use crate::domain::tool::CandleToolRouter;
use rmcp::model::Tool as ToolInfo;
```

**Update struct field:**
```rust
pub struct CandleAgent {
    // OLD
    tool_router: Arc<SweetMcpRouter>,
    
    // NEW
    tool_router: Arc<CandleToolRouter>,
    
    // ... other fields
}
```

**Update method calls:**
```rust
// Tool execution - API stays similar
let result = self.tool_router
    .call_tool(tool_name, args)
    .await?;

// Tool listing - update to use new return type
let tools: Vec<ToolInfo> = self.tool_router
    .get_available_tools()
    .await;
```

---

## File 5: src/domain/completion/types.rs

### Changes Required

**Update re-export:**
```rust
// OLD
pub use sweet_mcp_type::ToolInfo;

// NEW
pub use rmcp::model::Tool as ToolInfo;
```

---

## File 6: src/domain/completion/request.rs

### Changes Required

**Update imports:**
```rust
// OLD
use sweet_mcp_type::ToolInfo;

// NEW
use rmcp::model::Tool as ToolInfo;
```

**Update field types:**
```rust
pub struct CandleCompletionRequest {
    // ... other fields
    
    // OLD
    pub tools: Option<Vec<ToolInfo>>,
    
    // NEW - same type via alias
    pub tools: Option<Vec<ToolInfo>>,  // Now rmcp::model::Tool
}
```

---

## File 7: src/domain/chat/orchestration.rs

### Changes Required

**Update imports:**
```rust
// OLD
use sweet_mcp_type::{JsonValue, ToolInfo};
use crate::domain::tool::SweetMcpRouter;

// NEW
use serde_json::Value as JsonValue;
use rmcp::model::Tool as ToolInfo;
use crate::domain::tool::CandleToolRouter;
```

**Update function signatures:**
```rust
// OLD
pub async fn execute_tool_call(
    router: &SweetMcpRouter,
    tool_name: &str,
    args: JsonValue,
) -> Result<Value>

// NEW - same signature, just different router type
pub async fn execute_tool_call(
    router: &CandleToolRouter,
    tool_name: &str,
    args: JsonValue,
) -> Result<Value>
```

**Update tool metadata extraction:**
```rust
// OLD - using sweet_mcp_type::ToolInfo fields
let name = tool.name.clone();
let description = tool.description.clone();
let schema = tool.input_schema.clone();

// NEW - using rmcp::model::Tool fields
let name = tool.name.to_string();
let description = tool.description.as_ref().map(|s| s.to_string());
let schema = (*tool.input_schema).clone();  // Dereference Arc
```

---

## File 8: src/lib.rs (Prelude Updates)

### Changes Required

**Update re-exports:**
```rust
pub mod prelude {
    // ... existing exports
    
    // OLD
    pub use crate::domain::tool::{RouterError, SweetMcpRouter, ToolInfo, ToolRoute};
    
    // NEW
    pub use crate::domain::tool::{RouterError, CandleToolRouter};
    pub use rmcp::model::Tool as ToolInfo;
    pub use kodegen_mcp_tool::Tool;
}
```

---

## Common Patterns

### Pattern 1: JSON Conversion
**OLD:**
```rust
let sweet_json = convert_serde_to_sweet_json(serde_value);
some_function(sweet_json);
```

**NEW:**
```rust
// No conversion needed
some_function(serde_value);
```

### Pattern 2: Tool Schema Access
**OLD:**
```rust
let schema = tool.input_schema;  // JsonValue
```

**NEW:**
```rust
let schema = &*tool.input_schema;  // Dereference Arc<Map<String, Value>>
// Or clone if ownership needed:
let schema = (*tool.input_schema).clone();
```

### Pattern 3: Tool Creation
**OLD:**
```rust
use sweet_mcp_type::ToolInfo;

let tool = ToolInfo {
    name: "my_tool".to_string(),
    description: Some("Description".to_string()),
    input_schema: schema,
};
```

**NEW:**
```rust
use rmcp::model::Tool;
use std::borrow::Cow;
use std::sync::Arc;

let tool = Tool {
    name: Cow::Borrowed("my_tool"),
    title: None,
    description: Some(Cow::Borrowed("Description")),
    input_schema: Arc::new(schema),
    output_schema: None,
    annotations: None,
    icons: None,
};
```

### Pattern 4: Router Initialization
**OLD:**
```rust
let router = SweetMcpRouter::new();
let router = SweetMcpRouter::with_configs(plugins, cylo_config);
```

**NEW:**
```rust
let router = CandleToolRouter::new(None);
let router = CandleToolRouter::new(Some(mcp_client))
    .with_cylo("Apple".to_string(), "swift:latest".to_string());
```

---

## Compilation Verification

After each file update, verify:
```bash
cargo check --package kodegen_candle_agent
```

Expected progression:
1. First file: Many errors remaining
2. Middle files: Errors decreasing
3. Last file: Zero errors ✓
