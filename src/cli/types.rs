use anyhow::Context;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;

/// Toolset configuration loaded from YAML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsetConfig {
    /// List of individual tool names to enable
    pub tools: Vec<String>,
}

impl ToolsetConfig {
    /// Load toolset config from JSON file
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read toolset file: {}", path.display()))?;
        
        // Parse as JSON
        let config: ToolsetConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse toolset file as JSON: {}", path.display()))?;
        
        Ok(config)
    }
}

/// KODEGEN MCP Server - Memory-efficient, blazing-fast tools for AI agents
///
/// Available tool categories:
/// - filesystem: File operations
/// - terminal: Terminal sessions
/// - process: Process management
/// - introspection: Usage tracking
/// - prompt: Prompt management
/// - `sequential_thinking`: Reasoning chains
/// - `claude_agent`: Sub-agent delegation
/// - citescrape: Web crawling and search
/// - git: Git operations
#[derive(Parser, Debug)]
#[command(name = "kodegen")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to run
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable specific tools by name (comma-separated)
    ///
    /// Example: --tools fs_read_file,fs_write_file,start_terminal
    ///
    /// If not specified, all tools are enabled.
    #[arg(long, value_delimiter = ',', conflicts_with = "tool")]
    pub tools: Option<Vec<String>>,

    /// Enable specific tool by name (can be specified multiple times)
    ///
    /// Example: --tool fs_read_file --tool fs_write_file --tool start_terminal
    ///
    /// If not specified, all tools are enabled.
    #[arg(long = "tool", conflicts_with = "tools")]
    pub tool: Vec<String>,

    /// Load tool names from JSON file
    ///
    /// JSON format:
    /// ```json
    /// {
    ///   "tools": [
    ///     "fs_read_file",
    ///     "fs_write_file",
    ///     "start_terminal"
    ///   ]
    /// }
    /// ```
    ///
    /// Example: --toolset ~/.config/kodegen/toolset.json
    #[arg(long, value_name = "PATH", conflicts_with_all = ["tool", "tools"])]
    pub toolset: Option<std::path::PathBuf>,

    /// Run as HTTP server (streaming HTTP transport) instead of stdio
    /// Example: --http 127.0.0.1:30437
    #[arg(long, value_name = "ADDR", conflicts_with = "list_categories")]
    pub http: Option<SocketAddr>,

    /// Path to TLS certificate file (PEM format) for HTTPS
    /// Required for HTTPS support, must be used with --tls-key
    /// Example: --tls-cert /path/to/cert.pem
    #[arg(long, value_name = "PATH", requires = "tls_key")]
    pub tls_cert: Option<std::path::PathBuf>,

    /// Path to TLS private key file (PEM format) for HTTPS
    /// Required for HTTPS support, must be used with --tls-cert
    /// Example: --tls-key /path/to/key.pem
    #[arg(long, value_name = "PATH", requires = "tls_cert")]
    pub tls_key: Option<std::path::PathBuf>,

    /// HTTP server URL for proxy mode (stdio server only)
    ///
    /// When specified, the server will proxy all tool calls to the HTTP server.
    /// If the connection fails, the server will exit with an error rather than
    /// falling back to standalone mode. This ensures that when proxy mode is
    /// explicitly requested, it is always honored.
    ///
    /// Example: --proxy-http <http://localhost:8080/mcp>
    #[arg(long, value_name = "URL", conflicts_with = "http")]
    pub proxy_http: Option<String>,

    /// List available tool categories and exit
    #[arg(long)]
    pub list_categories: bool,

    /// List available tool names and exit
    #[arg(long)]
    pub list_tools: bool,

    /// Graceful shutdown timeout in seconds for HTTP server (default: 30)
    /// Can also be set via `KODEGEN_SHUTDOWN_TIMEOUT_SECS` environment variable
    #[arg(
        long,
        value_name = "SECONDS",
        env = "KODEGEN_SHUTDOWN_TIMEOUT_SECS",
        default_value = "30"
    )]
    pub shutdown_timeout: u64,

    /// HTTP connection timeout in seconds (default: 5)
    /// Can also be set via `KODEGEN_HTTP_TIMEOUT_SECS` environment variable
    #[arg(long, value_name = "SECONDS", env = "KODEGEN_HTTP_TIMEOUT_SECS")]
    pub http_timeout: Option<u64>,

    /// Maximum HTTP connection retry attempts (default: 3)
    /// Set to 1 to disable retries (fail fast)
    #[arg(long, value_name = "COUNT", default_value = "3")]
    pub http_retries: u32,

    /// Initial HTTP retry backoff in seconds (default: 1)
    /// Backoff doubles on each retry up to 10 seconds maximum
    #[arg(long, value_name = "SECONDS", default_value = "1")]
    pub http_retry_backoff: u64,

    /// Disable HTTP connection retries (fail fast on first failure)
    /// Useful for development to avoid waiting on retry delays
    #[arg(long)]
    pub http_no_retry: bool,

    /// HTTP server host
    /// Defaults to 127.0.0.1 when --no-tls is used (local testing)
    /// Defaults to mcp.kodegen.ai otherwise (production)
    #[arg(long, value_name = "HOST")]
    pub host: Option<String>,

    /// Disable TLS for HTTP connections (use HTTP instead of HTTPS)
    /// Useful for local testing without certificates
    #[arg(long)]
    pub no_tls: bool,

    // ============ Database Configuration ============
    /// Database connection string (DSN)
    ///
    /// Format varies by database type:
    /// - PostgreSQL: postgres://user:pass@host:5432/dbname
    /// - MySQL: mysql://user:pass@host:3306/dbname
    /// - SQLite: sqlite:///path/to/database.db
    /// - SQL Server: sqlserver://user:pass@host:1433/dbname
    ///
    /// If not provided, database tools will not be available.
    #[arg(long, env = "DATABASE_DSN")]
    pub database_dsn: Option<String>,

    /// Enable read-only mode (only SELECT/SHOW/EXPLAIN/DESCRIBE allowed)
    ///
    /// When enabled, any INSERT/UPDATE/DELETE/DROP statements will be rejected.
    /// Useful for safe database exploration by AI agents.
    #[arg(long, env = "DATABASE_READONLY")]
    pub database_readonly: bool,

    /// Maximum rows per SELECT query
    ///
    /// Automatically applies LIMIT clause to SELECT statements to prevent
    /// large result sets. If not set, queries can return unlimited rows.
    #[arg(long, env = "DATABASE_MAX_ROWS")]
    pub database_max_rows: Option<usize>,

    /// SSH tunnel host for database connection
    ///
    /// When specified, creates SSH tunnel to bastion host before connecting
    /// to database. Requires --ssh-user and either --ssh-key or --ssh-password.
    #[arg(long, env = "SSH_HOST")]
    pub ssh_host: Option<String>,

    /// SSH tunnel port
    #[arg(long, env = "SSH_PORT", default_value = "22")]
    pub ssh_port: u16,

    /// SSH username for tunnel authentication
    #[arg(long, env = "SSH_USER")]
    pub ssh_user: Option<String>,

    /// SSH private key path for key-based authentication
    ///
    /// If both --ssh-key and --ssh-password are provided, key is preferred.
    #[arg(long, env = "SSH_KEY")]
    pub ssh_key: Option<std::path::PathBuf>,

    /// SSH password for password-based authentication
    #[arg(long, env = "SSH_PASSWORD")]
    pub ssh_password: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Automatically configure MCP-compatible editors
    Install,
}

impl Cli {
    /// Get the set of enabled tool names (individual tools, not categories)
    ///
    /// Returns None if no filter specified (enable all tools)
    /// Returns Some(HashSet) if filter specified (enable only these tools)
    pub fn enabled_tools(&self) -> anyhow::Result<Option<HashSet<String>>> {
        // Priority 1: --toolset (YAML file)
        if let Some(ref path) = self.toolset {
            let config = ToolsetConfig::from_file(path)?;
            return Ok(Some(config.tools.into_iter().collect()));
        }

        // Priority 2: --tools (comma-separated)
        if let Some(tools) = &self.tools {
            return Ok(Some(tools.iter().cloned().collect()));
        }

        // Priority 3: --tool (repeated flags)
        if !self.tool.is_empty() {
            return Ok(Some(self.tool.iter().cloned().collect()));
        }

        // No filter specified - enable all tools
        Ok(None)
    }

    /// Get the HTTP connection timeout with fallback to config
    pub fn http_connection_timeout(
        &self,
        config_manager: &kodegen_config_manager::ConfigManager,
    ) -> std::time::Duration {
        let seconds = self
            .http_timeout
            .unwrap_or_else(|| config_manager.get_http_connection_timeout_secs());
        std::time::Duration::from_secs(seconds)
    }

    /// Get the maximum number of HTTP connection retry attempts
    /// Returns 1 if --http-no-retry is set (no retries, fail fast)
    pub fn http_max_retries(&self) -> u32 {
        if self.http_no_retry {
            1
        } else {
            self.http_retries
        }
    }

    /// Get the initial HTTP retry backoff duration
    pub fn http_retry_backoff_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.http_retry_backoff)
    }

    /// Get the effective HTTP host based on --no-tls and --host flags
    ///
    /// Returns:
    /// - Explicit --host value if provided
    /// - "127.0.0.1" if --no-tls is set without explicit --host (local testing)
    /// - "mcp.kodegen.ai" otherwise (production default)
    pub fn effective_host(&self) -> &str {
        if let Some(ref host) = self.host {
            host
        } else if self.no_tls {
            "127.0.0.1"
        } else {
            "mcp.kodegen.ai"
        }
    }
}

/// Get all available tool categories (runtime filtering via --tool/--tools)
pub fn available_categories() -> Vec<&'static str> {
    vec![
        "filesystem",
        "terminal",
        "process",
        "introspection",
        "prompt",
        "reasoner",
        "sequential_thinking",
        "claude_agent",
        "candle_agent",
        "citescrape",
        "git",
        "github",
        "config",
        "database",
    ]
}

/// Get all available tool names (individual tools, not categories)
pub fn available_tools() -> Vec<&'static str> {
    crate::stdio::metadata::all_tool_metadata()
        .iter()
        .map(|tool| tool.name)
        .collect()
}
