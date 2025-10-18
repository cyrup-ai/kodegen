use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::net::SocketAddr;

/// KODEGEN MCP Server - Memory-efficient, blazing-fast tools for AI agents
///
/// Available tool categories:
/// - filesystem: File operations
/// - terminal: Terminal sessions
/// - process: Process management
/// - introspection: Usage tracking
/// - prompt: Prompt management
/// - sequential_thinking: Reasoning chains
/// - claude_agent: Sub-agent delegation
/// - citescrape: Web crawling and search
/// - git: Git operations
#[derive(Parser, Debug)]
#[command(name = "kodegen")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to run
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    /// Enable specific tool categories (comma-separated)
    /// 
    /// Example: --tools filesystem,terminal,citescrape
    /// 
    /// If not specified, all compiled tool categories are enabled.
    #[arg(long, value_delimiter = ',', conflicts_with = "tool")]
    pub tools: Option<Vec<String>>,

    /// Enable specific tool category (can be specified multiple times)
    /// 
    /// Example: --tool filesystem --tool terminal --tool citescrape
    /// 
    /// If not specified, all compiled tool categories are enabled.
    #[arg(long = "tool", conflicts_with = "tools")]
    pub tool: Vec<String>,

    /// Run as SSE server (HTTP transport) instead of stdio
    /// Example: --sse 127.0.0.1:30437
    #[arg(long, value_name = "ADDR", conflicts_with = "list_categories")]
    pub sse: Option<SocketAddr>,

    /// SSE server URL for proxy mode (stdio server only)
    /// Example: --proxy-sse http://localhost:8080/sse
    #[arg(long, value_name = "URL", conflicts_with = "sse")]
    pub proxy_sse: Option<String>,

    /// List available tool categories and exit
    #[arg(long)]
    pub list_categories: bool,

    /// Graceful shutdown timeout in seconds for SSE server (default: 30)
    /// Can also be set via KODEGEN_SHUTDOWN_TIMEOUT_SECS environment variable
    #[arg(long, value_name = "SECONDS", env = "KODEGEN_SHUTDOWN_TIMEOUT_SECS", default_value = "30")]
    pub shutdown_timeout: u64,

    /// SSE connection timeout in seconds (default: 5)
    /// Can also be set via KODEGEN_SSE_TIMEOUT_SECS environment variable
    #[arg(long, value_name = "SECONDS", env = "KODEGEN_SSE_TIMEOUT_SECS")]
    pub sse_timeout: Option<u64>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Automatically configure MCP-compatible editors
    Install,
}

/// Server mode selection
#[derive(Debug, Clone)]
pub enum ServerMode {
    /// Run as stdio server with optional SSE proxy
    Stdio { proxy_url: Option<String> },
    /// Run as SSE server on the specified address
    Sse(SocketAddr),
}

impl Cli {
    /// Get the set of enabled tool categories
    /// 
    /// Returns None if no filter specified (enable all compiled categories)
    /// Returns Some(HashSet) if filter specified (enable only these categories)
    pub fn enabled_categories(&self) -> Option<HashSet<String>> {
        // If --tools was used (comma-separated)
        if let Some(tools) = &self.tools {
            return Some(tools.iter().cloned().collect());
        }
        
        // If --tool was used (multiple flags)
        if !self.tool.is_empty() {
            return Some(self.tool.iter().cloned().collect());
        }
        
        // No filter specified - enable all
        None
    }

    /// Determine which server mode to use
    pub fn server_mode(&self) -> ServerMode {
        if let Some(addr) = self.sse {
            ServerMode::Sse(addr)
        } else {
            ServerMode::Stdio { 
                proxy_url: self.proxy_sse.clone() 
            }
        }
    }

    /// Get the shutdown timeout as a Duration
    pub fn shutdown_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.shutdown_timeout)
    }

    /// Get the SSE connection timeout with fallback to config
    pub fn sse_connection_timeout(&self, config_manager: &kodegen_config::ConfigManager) -> std::time::Duration {
        let seconds = self.sse_timeout.unwrap_or_else(|| config_manager.get_sse_connection_timeout_secs());
        std::time::Duration::from_secs(seconds)
    }
}

/// Get all available tool categories (based on compiled features)
pub fn available_categories() -> Vec<&'static str> {
    let categories = vec![
        #[cfg(feature = "filesystem")]
        "filesystem",

        #[cfg(feature = "terminal")]
        "terminal",

        #[cfg(feature = "process")]
        "process",

        #[cfg(feature = "introspection")]
        "introspection",

        #[cfg(feature = "prompt")]
        "prompt",

        #[cfg(feature = "sequential_thinking")]
        "sequential_thinking",

        #[cfg(feature = "claude_agent")]
        "claude_agent",

        #[cfg(feature = "citescrape")]
        "citescrape",

        #[cfg(feature = "git")]
        "git",

        #[cfg(feature = "github")]
        "github",

        #[cfg(feature = "config")]
        "config",
    ];

    categories
}
