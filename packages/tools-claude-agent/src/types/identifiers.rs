//! Newtype wrappers for type safety
//!
//! This module contains newtype wrappers that provide type safety by wrapping
//! primitive types (like String) into distinct types.

use serde::{Deserialize, Serialize};

// ============================================================================
// Newtype Wrappers for Type Safety
// ============================================================================

/// Session ID newtype for type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    /// Create a new session ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the session ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self("default".to_string())
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Tool name newtype
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolName(String);

impl ToolName {
    /// Create a new tool name
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the tool name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ToolName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ToolName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Request ID newtype for control protocol
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(String);

impl RequestId {
    /// Create a new request ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the request ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
