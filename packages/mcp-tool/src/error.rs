use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Operation not supported in read-only mode: {0}")]
    ReadOnlyViolation(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Search index error (missing, corrupted, query execution failed)
    #[error("Search index error: {0}")]
    SearchIndex(String),

    /// Manifest file error (missing, parse error, write failed)
    #[error("Manifest error: {0}")]
    Manifest(String),

    /// Invalid URL (parse error, missing host, invalid format)
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Search engine initialization or operation error
    #[error("Search engine error: {0}")]
    SearchEngine(String),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

impl McpError {
    /// Helper to create search index error
    pub fn search_index(msg: impl Into<String>) -> Self {
        Self::SearchIndex(msg.into())
    }

    /// Helper to create manifest error
    pub fn manifest(msg: impl Into<String>) -> Self {
        Self::Manifest(msg.into())
    }

    /// Helper to create invalid URL error
    pub fn invalid_url(msg: impl Into<String>) -> Self {
        Self::InvalidUrl(msg.into())
    }

    /// Helper to create search engine error
    pub fn search_engine(msg: impl Into<String>) -> Self {
        Self::SearchEngine(msg.into())
    }

    /// Helper to create invalid arguments error
    pub fn invalid_arguments(msg: impl Into<String>) -> Self {
        Self::InvalidArguments(msg.into())
    }

    /// Helper to create resource not found error
    pub fn resource_not_found(msg: impl Into<String>) -> Self {
        Self::ResourceNotFound(msg.into())
    }
}

impl From<McpError> for rmcp::ErrorData {
    fn from(err: McpError) -> Self {
        match err {
            McpError::InvalidArguments(msg) => Self::invalid_params(msg, None),
            McpError::ToolNotFound(msg) => {
                Self::new(rmcp::model::ErrorCode::METHOD_NOT_FOUND, msg, None)
            }
            McpError::ResourceNotFound(msg) => Self::resource_not_found(msg, None),
            McpError::PermissionDenied(msg) | McpError::ReadOnlyViolation(msg) => {
                Self::internal_error(format!("Unauthorized: {msg}"), None)
            }
            // New variants map to internal_error (semantics for Rust, not MCP)
            McpError::SearchIndex(msg)
            | McpError::Manifest(msg)
            | McpError::InvalidUrl(msg)
            | McpError::SearchEngine(msg) => Self::internal_error(msg, None),
            _ => Self::internal_error(err.to_string(), None),
        }
    }
}
