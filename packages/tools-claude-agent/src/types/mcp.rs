//! MCP (Model Context Protocol) server configuration types
//!
//! This module contains types for configuring various types of MCP servers
//! including stdio, SSE, HTTP, and SDK-based servers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// MCP Server Types
// ============================================================================

/// MCP stdio server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStdioServerConfig {
    /// Server type (stdio)
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub server_type: Option<String>,
    /// Command to execute
    pub command: String,
    /// Command arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// MCP SSE server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSseServerConfig {
    /// Server type (sse)
    #[serde(rename = "type")]
    pub server_type: String,
    /// Server URL
    pub url: String,
    /// HTTP headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// MCP HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHttpServerConfig {
    /// Server type (http)
    #[serde(rename = "type")]
    pub server_type: String,
    /// Server URL
    pub url: String,
    /// HTTP headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// SDK MCP server marker (not serialized directly)
#[derive(Debug, Clone)]
pub struct SdkMcpServerMarker {
    /// Server name
    pub name: String,
}

/// MCP server configuration enum
#[derive(Debug, Clone)]
pub enum McpServerConfig {
    /// Stdio-based MCP server
    Stdio(McpStdioServerConfig),
    /// SSE-based MCP server
    Sse(McpSseServerConfig),
    /// HTTP-based MCP server
    Http(McpHttpServerConfig),
    /// SDK-based in-process MCP server
    Sdk(SdkMcpServerMarker),
}

/// MCP servers container
#[derive(Debug, Clone, Default)]
pub enum McpServers {
    /// No MCP servers
    #[default]
    None,
    /// Dictionary of MCP servers
    Dict(HashMap<String, McpServerConfig>),
    /// Path to MCP servers configuration file
    Path(PathBuf),
}
