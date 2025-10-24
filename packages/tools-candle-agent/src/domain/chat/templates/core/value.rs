//! Template value types and conversions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Template value type for variables
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateValue {
    /// String value stored as String for zero-allocation sharing
    String(String),
    /// Numeric value stored as 64-bit floating point
    Number(f64),
    /// Boolean true/false value
    Boolean(bool),
    /// Array of template values
    Array(Vec<TemplateValue>),
    /// Object/map of key-value pairs
    Object(HashMap<String, TemplateValue>),
    /// Null/empty value
    Null,
}

impl From<&str> for TemplateValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for TemplateValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<f64> for TemplateValue {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<bool> for TemplateValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}
