//! Error types for the Claude Agent SDK

use thiserror::Error;

/// Main error type for the Claude Agent SDK
#[derive(Error, Debug)]
pub enum ClaudeError {
    /// Claude Code CLI not found or not installed
    #[error("Claude Code CLI not found: {0}")]
    CliNotFound(String),

    /// Connection error when communicating with Claude Code
    #[error("Connection error: {0}")]
    Connection(String),

    /// Process execution error with exit code and stderr
    #[error("Process error (exit code {exit_code}): {message}")]
    Process {
        /// Error message
        message: String,
        /// Process exit code
        exit_code: i32,
        /// Standard error output
        stderr: Option<String>,
    },

    /// JSON decode error when parsing CLI output
    #[error("JSON decode error: {0}")]
    JsonDecode(#[from] serde_json::Error),

    /// Message parse error with optional raw data
    #[error("Message parse error: {message}")]
    MessageParse {
        /// Error message
        message: String,
        /// Raw message data that failed to parse
        data: Option<serde_json::Value>,
    },

    /// Transport layer error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Control protocol error
    #[error("Control protocol error: {0}")]
    ControlProtocol(String),

    /// Hook execution error
    #[error("Hook error: {0}")]
    Hook(String),

    /// MCP (Model Context Protocol) error
    #[error("MCP error: {0}")]
    Mcp(String),

    /// I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Timeout error
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Agent session not found
    #[error("Agent session not found: {0}")]
    SessionNotFound(String),

    /// Agent session already complete (cannot send more messages)
    #[error("Agent session {0} is already complete")]
    SessionComplete(String),

    /// Maximum active sessions limit reached
    #[error("Maximum active sessions reached: {0}")]
    MaxSessionsReached(usize),

    /// Invalid agent session configuration
    #[error("Invalid agent configuration: {0}")]
    InvalidAgentConfiguration(String),

    /// Failed to render prompt template
    #[error("Failed to render prompt template '{template}': {message}")]
    PromptTemplateError {
        /// Template name
        template: String,
        /// Error message
        message: String,
    },
}

/// Result type alias for Claude SDK operations
pub type Result<T> = std::result::Result<T, ClaudeError>;

impl ClaudeError {
    /// Create a CLI not found error
    #[must_use]
    pub fn cli_not_found() -> Self {
        Self::CliNotFound(
            "Claude Code not found. Install with:\n\
             npm install -g @anthropic-ai/claude-code\n\
             \n\
             If already installed locally, try:\n\
             export PATH=\"$HOME/node_modules/.bin:$PATH\"\n\
             \n\
             Or specify the path when creating transport"
                .to_string(),
        )
    }

    /// Create a connection error
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Connection(msg.into())
    }

    /// Create a process error
    pub fn process(msg: impl Into<String>, exit_code: i32, stderr: Option<String>) -> Self {
        Self::Process {
            message: msg.into(),
            exit_code,
            stderr,
        }
    }

    /// Create a message parse error
    pub fn message_parse(msg: impl Into<String>, data: Option<serde_json::Value>) -> Self {
        Self::MessageParse {
            message: msg.into(),
            data,
        }
    }

    /// Create a transport error
    pub fn transport(msg: impl Into<String>) -> Self {
        Self::Transport(msg.into())
    }

    /// Create a control protocol error
    pub fn control_protocol(msg: impl Into<String>) -> Self {
        Self::ControlProtocol(msg.into())
    }

    /// Alias for `control_protocol` - used by protocol handler
    pub fn protocol_error(msg: impl Into<String>) -> Self {
        Self::ControlProtocol(msg.into())
    }

    /// Create a JSON encode error
    pub fn json_encode(msg: impl Into<String>) -> Self {
        Self::JsonDecode(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            msg.into(),
        )))
    }

    /// Create a JSON decode error from string
    pub fn json_decode(msg: impl Into<String>) -> Self {
        Self::JsonDecode(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            msg.into(),
        )))
    }

    /// Create a hook error
    pub fn hook(msg: impl Into<String>) -> Self {
        Self::Hook(msg.into())
    }

    /// Create an MCP error
    pub fn mcp(msg: impl Into<String>) -> Self {
        Self::Mcp(msg.into())
    }

    /// Create a timeout error
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create an invalid configuration error
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        Self::InvalidConfig(msg.into())
    }

    /// Create a session not found error
    pub fn session_not_found(session_id: impl Into<String>) -> Self {
        Self::SessionNotFound(session_id.into())
    }

    /// Create a session complete error
    pub fn session_complete(session_id: impl Into<String>) -> Self {
        Self::SessionComplete(session_id.into())
    }

    /// Create a max sessions error
    #[must_use]
    pub fn max_sessions_reached(max: usize) -> Self {
        Self::MaxSessionsReached(max)
    }

    /// Create an invalid agent configuration error
    pub fn invalid_agent_config(msg: impl Into<String>) -> Self {
        Self::InvalidAgentConfiguration(msg.into())
    }
}

// Conversion to kodegen_tool McpError
impl From<ClaudeError> for kodegen_mcp_tool::error::McpError {
    fn from(err: ClaudeError) -> Self {
        use kodegen_mcp_tool::error::McpError;

        match err {
            ClaudeError::CliNotFound(msg) => McpError::ResourceNotFound(msg),
            ClaudeError::Connection(msg) => McpError::Network(msg),
            ClaudeError::Transport(msg) => McpError::Network(msg),
            ClaudeError::JsonDecode(e) => McpError::Serialization(e),
            ClaudeError::Io(e) => McpError::Io(e),
            ClaudeError::InvalidConfig(msg) => McpError::InvalidArguments(msg),
            ClaudeError::Timeout(msg) => McpError::Other(anyhow::anyhow!("Timeout: {msg}")),
            ClaudeError::Process { message, .. } => {
                McpError::Other(anyhow::anyhow!("Process error: {message}"))
            }
            ClaudeError::MessageParse { message, .. } => {
                McpError::Other(anyhow::anyhow!("Parse error: {message}"))
            }
            ClaudeError::ControlProtocol(msg) => {
                McpError::Other(anyhow::anyhow!("Protocol error: {msg}"))
            }
            ClaudeError::Hook(msg) => McpError::Other(anyhow::anyhow!("Hook error: {msg}")),
            ClaudeError::Mcp(msg) => McpError::Other(anyhow::anyhow!("MCP error: {msg}")),
            ClaudeError::SessionNotFound(msg) => McpError::ResourceNotFound(msg),
            ClaudeError::SessionComplete(msg) => {
                McpError::InvalidArguments(format!("Session complete: {msg}"))
            }
            ClaudeError::MaxSessionsReached(max) => {
                McpError::Other(anyhow::anyhow!("Max sessions reached: {max}"))
            }
            ClaudeError::InvalidAgentConfiguration(msg) => McpError::InvalidArguments(msg),
            ClaudeError::PromptTemplateError { template, message } => {
                McpError::Other(anyhow::anyhow!("Template '{template}' error: {message}"))
            }
        }
    }
}
