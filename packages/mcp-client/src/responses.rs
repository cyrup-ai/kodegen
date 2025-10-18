//! Typed response structures for MCP tool calls
//!
//! This module provides strongly-typed response structures for parsing tool call results.
//! Using these types instead of manual JSON parsing provides:
//! - Type safety with compiler checks
//! - Clear error messages with context
//! - Support for multiple field name conventions (camelCase/snake_case)
//! - Prevention of silent failures from missing or mistyped fields

use serde::Deserialize;

/// Response from starting a web crawl session
#[derive(Debug, Deserialize)]
pub struct StartCrawlResponse {
    /// The session ID for this crawl
    /// Supports both `sessionId` (camelCase) and `session_id` (snake_case)
    #[serde(alias = "sessionId")]
    pub session_id: String,
}

/// Response from starting a file/content search
#[derive(Debug, Deserialize)]
pub struct StartSearchResponse {
    /// The session ID for this search
    /// Supports both `sessionId` (camelCase) and `session_id` (snake_case)
    #[serde(alias = "sessionId")]
    pub session_id: String,
}

/// Response from spawning a Claude agent sub-session
#[derive(Debug, Deserialize)]
pub struct SpawnClaudeAgentResponse {
    /// The session ID for this Claude agent
    /// Supports both `sessionId` (camelCase) and `session_id` (snake_case)
    #[serde(alias = "sessionId")]
    pub session_id: String,
}

/// Response from starting a terminal command
#[derive(Debug, Deserialize)]
pub struct StartTerminalCommandResponse {
    /// The process ID (PID) of the started command
    pub pid: i64,
    
    /// Optional status information
    #[serde(default)]
    pub status: Option<String>,
}
