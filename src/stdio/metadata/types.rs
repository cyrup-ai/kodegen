//! Core types for static tool metadata.

use rmcp::schemars::{schema_for, JsonSchema};
use serde_json::Value;

/// Metadata for a single tool.
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub schema: Value,
}

/// Helper to build schema from Args type.
pub fn build_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).unwrap_or(Value::Null)
}
