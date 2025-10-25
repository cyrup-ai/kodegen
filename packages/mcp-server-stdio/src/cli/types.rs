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

    /// SSE server URL for proxy mode (stdio server only)
    ///
    /// When specified, the server will proxy all tool calls to the SSE server.
    /// If the connection fails, the server will exit with an error rather than
    /// falling back to standalone mode. This ensures that when proxy mode is
    /// explicitly requested, it is always honored.
    ///
    /// Example: --proxy-sse <http://localhost:8080/sse>
    #[arg(long, value_name = "URL", conflicts_with = "sse")]
    pub proxy_sse: Option<String>,

    /// List available tool categories and exit
    #[arg(long)]
    pub list_categories: bool,

    /// Graceful shutdown timeout in seconds for SSE server (default: 30)
    /// Can also be set via `KODEGEN_SHUTDOWN_TIMEOUT_SECS` environment variable
    #[arg(
        long,
        value_name = "SECONDS",
        env = "KODEGEN_SHUTDOWN_TIMEOUT_SECS",
        default_value = "30"
    )]
    pub shutdown_timeout: u64,

    /// SSE connection timeout in seconds (default: 5)
    /// Can also be set via `KODEGEN_SSE_TIMEOUT_SECS` environment variable
    #[arg(long, value_name = "SECONDS", env = "KODEGEN_SSE_TIMEOUT_SECS")]
    pub sse_timeout: Option<u64>,

    /// Maximum SSE connection retry attempts (default: 3)
    /// Set to 1 to disable retries (fail fast)
    #[arg(long, value_name = "COUNT", default_value = "3")]
    pub sse_retries: u32,

    /// Initial SSE retry backoff in seconds (default: 1)
    /// Backoff doubles on each retry up to 10 seconds maximum
    #[arg(long, value_name = "SECONDS", default_value = "1")]
    pub sse_retry_backoff: u64,

    /// Disable SSE connection retries (fail fast on first failure)
    /// Useful for development to avoid waiting on retry delays
    #[arg(long)]
    pub sse_no_retry: bool,

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

    /// Get the SSE connection timeout with fallback to config
    pub fn sse_connection_timeout(
        &self,
        config_manager: &kodegen_tools_config::ConfigManager,
    ) -> std::time::Duration {
        let seconds = self
            .sse_timeout
            .unwrap_or_else(|| config_manager.get_sse_connection_timeout_secs());
        std::time::Duration::from_secs(seconds)
    }

    /// Get the maximum number of SSE connection retry attempts
    /// Returns 1 if --sse-no-retry is set (no retries, fail fast)
    pub fn sse_max_retries(&self) -> u32 {
        if self.sse_no_retry {
            1
        } else {
            self.sse_retries
        }
    }

    /// Get the initial SSE retry backoff duration
    pub fn sse_retry_backoff_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.sse_retry_backoff)
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
