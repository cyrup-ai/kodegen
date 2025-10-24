# Candle Agent MCP Tool Migration - Implementation Guide

**Date**: 2025-10-23
**Status**: Ready for Implementation
**Priority**: Critical - Blocking compilation

## Objective

Fix 21 compilation errors in `kodegen_candle_agent` by completing the sweetMCP → workspace MCP migration.

## Current State

**Command**: `cargo check --package kodegen_candle_agent`
**Result**: 21 errors across 4 categories (plus 14 jsonwebtoken errors from stale Cargo.lock)

## What Already Exists

The codebase already has all necessary infrastructure:

- **[CandleToolRouter](../../src/domain/tool/router.rs)** - Lines 162-170: `register_tool()` method, Lines 182-203: `get_available_tools()`
- **[ToolWrapper](../../src/domain/tool/router.rs)** - Lines 97-115: Automatic Tool → rmcp::model::Tool conversion
- **[Helper architecture](../../src/builders/agent_role/helpers.rs)** - Line 54: Creates fresh router each inference, Line 70-74: Combines state.tools + router tools
- **Domain concurrency** - [mod.rs](../../src/domain/concurrency/mod.rs) exports Channel, OneshotChannel (NOT AsyncTask)

## Required Changes

### Change 1: Fix jsonwebtoken Version in SurrealDB Fork

**File**: [`forks/surrealdb/Cargo.toml:96`](../../../forks/surrealdb/Cargo.toml)

**Current**:
```toml
jsonwebtoken = { version = "10.0", default-features = false, features = ["aws_lc_rs", "pem"] }
```

**Change to**:
```toml
jsonwebtoken = "9.3"
```

**Status**: ✅ Already completed

---

### Change 2: Fix AsyncTask Imports in Embedding Builder

**File**: [`src/builders/embedding.rs`](../../src/builders/embedding.rs)

**Line 9 - Remove**:
```rust
use crate::domain::concurrency::{AsyncTask, spawn_task as spawn_async};
```

**Line 9 - Add**:
```rust
use tokio::task::JoinHandle;
```

**Line 25 - Change return type**:
```rust
// FROM:
fn embed(self) -> AsyncTask<Result<Embedding, Box<dyn std::error::Error + Send + Sync>>>;

// TO:
fn embed(self) -> JoinHandle<Result<Embedding, Box<dyn std::error::Error + Send + Sync>>>;
```

**Line 69 - Change spawn call**:
```rust
// FROM:
spawn_async(async move {

// TO:
tokio::task::spawn(async move {
```

**Rationale**: `AsyncTask` and `spawn_task` were removed from domain::concurrency. The module now only provides Channel/OneshotChannel. Using tokio::task directly is the correct pattern.

---

### Change 3: Fix Manual ToolInfo Construction (3 Files)

The architecture creates CandleToolRouter fresh each inference ([helpers.rs:54](../../src/builders/agent_role/helpers.rs)), so `state.tools` is the primary source of default tools. We need to fix the manual construction to include all required rmcp::model::Tool fields.

#### File 1: [`src/builders/agent_role/role_builder.rs:50-95`](../../src/builders/agent_role/role_builder.rs)

**Required changes for BOTH tool definitions** (thinking_tool and reasoner_tool):

1. Add missing fields:
```rust
title: None,
output_schema: None,
annotations: None,
icons: None,
```

2. Fix input_schema type (expects `Arc<Map<String, Value>>` not `Value`):
```rust
// FROM:
input_schema: serde_json::json!({ ... }),

// TO:
input_schema: std::sync::Arc::new(
    serde_json::json!({ ... })
        .as_object()
        .unwrap()
        .clone()
),
```

**Complete corrected structure**:
```rust
use std::sync::Arc;

let thinking_tool = ToolInfo {
    name: "thinking".into(),
    title: None,
    description: Some("Use this tool for all thinking and reasoning tasks...".into()),
    input_schema: Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": {
                "messages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "role": { "type": "string", "enum": ["user", "assistant"] },
                            "content": { "type": "string" }
                        },
                        "required": ["role", "content"]
                    }
                }
            },
            "required": ["messages"]
        }).as_object().unwrap().clone()
    ),
    output_schema: None,
    annotations: None,
    icons: None,
};

let reasoner_tool = ToolInfo {
    name: "mcp-reasoner".into(),
    title: None,
    description: Some("Advanced reasoning tool with Beam Search and MCTS strategies...".into()),
    input_schema: Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": {
                "thought": {"type": "string", "description": "Current reasoning step"},
                "thoughtNumber": {"type": "integer", "description": "Current step number", "minimum": 1},
                "totalThoughts": {"type": "integer", "description": "Total expected steps", "minimum": 1},
                "nextThoughtNeeded": {"type": "boolean", "description": "Whether another step is needed"},
                "parentId": {"type": ["string", "null"], "description": "Optional parent thought ID for branching"},
                "strategyType": {"type": ["string", "null"], "enum": ["beam_search", "mcts", "mcts_002_alpha", "mcts_002alt_alpha", null], "description": "Reasoning strategy to use"},
                "beamWidth": {"type": ["integer", "null"], "description": "Number of top paths to maintain", "minimum": 1, "maximum": 10},
                "numSimulations": {"type": ["integer", "null"], "description": "Number of MCTS simulations", "minimum": 1, "maximum": 150}
            },
            "required": ["thought", "thoughtNumber", "totalThoughts", "nextThoughtNeeded"]
        }).as_object().unwrap().clone()
    ),
    output_schema: None,
    annotations: None,
    icons: None,
};
```

#### File 2: [`src/domain/memory/tool.rs:125-139`](../../src/domain/memory/tool.rs)

**Same fixes as above**:

1. Add at line 126 (after `name:` field):
```rust
title: None,
```

2. Change line 127 description type:
```rust
// FROM:
description: Some("Memory management tool...".to_string()),

// TO:
description: Some("Memory management tool...".into()),
```

3. Fix input_schema (line 130):
```rust
// FROM:
input_schema: serde_json::json!({ ... }),

// TO:
input_schema: Arc::new(
    serde_json::json!({ ... })
        .as_object()
        .unwrap()
        .clone()
),
```

4. Add after input_schema (before closing brace):
```rust
output_schema: None,
annotations: None,
icons: None,
```

5. Add import at top of file:
```rust
use std::sync::Arc;
```

---

### Change 4: Fix Router Lifetime Issue

**File**: [`src/domain/tool/router.rs:117`](../../src/domain/tool/router.rs)

**Current**:
```rust
fn execute(&self, args: Value) -> Pin<Box<dyn std::future::Future<Output = Result<Value, RouterError>> + Send>> {
```

**Change to**:
```rust
fn execute(&self, args: Value) -> Pin<Box<dyn std::future::Future<Output = Result<Value, RouterError>> + Send + '_>> {
```

**Explanation**: The returned future borrows from `&self`, so it needs the `+ '_` lifetime bound to indicate it cannot outlive the borrow.

---

## Architecture Notes

### Why state.tools Instead of Router Registration?

From [helpers.rs:54](../../src/builders/agent_role/helpers.rs):
```rust
let tool_router = Some(CandleToolRouter::new(None));
```

The router is created **fresh each inference cycle** with no MCP client and no registered tools. The architecture combines two tool sources:

```rust
// Line 70: Static tools from agent configuration
let mut all_tools: Vec<ToolInfo> = state.tools.clone().into();

// Line 73: Dynamic tools from router (remote MCP servers)
let auto_generated_tools = router.get_available_tools().await;
all_tools.extend(auto_generated_tools);
```

Since the router is ephemeral, **state.tools is the correct place for default tools**. These must be properly constructed rmcp::model::Tool structs.

### rmcp::model::Tool Structure

From [router.rs:97-115](../../src/domain/tool/router.rs), the complete Tool structure is:

```rust
RmcpTool {
    name: Cow::Borrowed(T::name()),                    // Required: Tool name
    title: None,                                        // Optional: Display title
    description: Some(Cow::Borrowed(T::description())), // Optional: Description
    input_schema: T::input_schema(),                    // Required: Arc<Map<String, Value>>
    output_schema: T::output_schema(),                  // Optional: Arc<Map<String, Value>>
    annotations: Some(                                   // Optional: Metadata
        ToolAnnotations::new()
            .read_only(T::read_only())
            .destructive(T::destructive())
            .idempotent(T::idempotent())
            .open_world(T::open_world())
    ),
    icons: None,                                        // Optional: Icon URLs
}
```

Type alias from [domain/completion/types.rs:104](../../src/domain/completion/types.rs):
```rust
pub use rmcp::model::Tool as ToolInfo;
```

---

## Implementation Steps

### Step 1: Add Required Imports

**Files needing `use std::sync::Arc;`**:
- `src/builders/agent_role/role_builder.rs` (if not already present)
- `src/domain/memory/tool.rs`

### Step 2: Fix embedding.rs
1. Replace AsyncTask import with JoinHandle
2. Update return type on line 25
3. Update spawn call on line 69

### Step 3: Fix role_builder.rs
1. Add missing fields to both tool definitions (lines 50-95)
2. Wrap input_schema with Arc::new()

### Step 4: Fix domain/memory/tool.rs  
1. Add missing fields to tool definition (lines 125-139)
2. Wrap input_schema with Arc::new()
3. Add Arc import

### Step 5: Fix router.rs
1. Add `+ '_` to return type on line 117

### Step 6: Verify
```bash
cd /Users/davidmaple/kodegen
cargo check --package kodegen_candle_agent
```

---

## Definition of Done

1. ✅ `cargo check --package kodegen_candle_agent` returns **0 errors**
2. ✅ All 5 files modified correctly:
   - `forks/surrealdb/Cargo.toml` - jsonwebtoken = "9.3"
   - `src/builders/embedding.rs` - JoinHandle and tokio::task::spawn
   - `src/builders/agent_role/role_builder.rs` - Complete ToolInfo structs
   - `src/domain/memory/tool.rs` - Complete ToolInfo struct
   - `src/domain/tool/router.rs` - Lifetime bound added
3. ✅ Cargo.lock will auto-update on next clean build to use jsonwebtoken 9.3.x

---

## File Reference Map

### Files to Modify
| File | Lines | Change |
|------|-------|--------|
| [`forks/surrealdb/Cargo.toml`](../../../forks/surrealdb/Cargo.toml) | 96 | Set jsonwebtoken = "9.3" ✅ |
| [`src/builders/embedding.rs`](../../src/builders/embedding.rs) | 9, 25, 69 | Replace AsyncTask with JoinHandle |
| [`src/builders/agent_role/role_builder.rs`](../../src/builders/agent_role/role_builder.rs) | 50-95 | Fix 2 ToolInfo structs |
| [`src/domain/memory/tool.rs`](../../src/domain/memory/tool.rs) | 125-139 | Fix 1 ToolInfo struct |
| [`src/domain/tool/router.rs`](../../src/domain/tool/router.rs) | 117 | Add `+ '_` lifetime |

### Reference Files (Read-Only)
- [Tool trait](../../../packages/mcp-tool/src/tool.rs) - Workspace Tool trait definition
- [CandleToolRouter](../../src/domain/tool/router.rs) - Router implementation with ToolWrapper
- [Helper architecture](../../src/builders/agent_role/helpers.rs) - Tool combination logic
- [Domain concurrency](../../src/domain/concurrency/mod.rs) - Available async primitives
- [Completion types](../../src/domain/completion/types.rs) - ToolInfo type alias

---

**END OF IMPLEMENTATION GUIDE**
