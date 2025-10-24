//! Error types for message formatting operations

use thiserror::Error;

/// Formatting errors with owned strings
#[derive(Error, Debug, Clone)]
pub enum FormatError {
    /// Invalid markdown syntax encountered
    #[error("Invalid markdown syntax: {detail}")]
    InvalidMarkdown {
        /// Details about the syntax error
        detail: String,
    },
    /// Programming language not supported for syntax highlighting
    #[error("Unsupported language: {language}")]
    UnsupportedLanguage {
        /// The unsupported language identifier
        language: String,
    },
    /// Error occurred during parsing
    #[error("Parse error: {detail}")]
    ParseError {
        /// Details about the parsing error
        detail: String,
    },
    /// Error occurred during rendering
    #[error("Render error: {detail}")]
    RenderError {
        /// Details about the rendering error
        detail: String,
    },
    /// Content validation failed
    #[error("Invalid content: {detail}")]
    InvalidContent {
        /// Details about the content validation failure
        detail: String,
    },
    /// Configuration is invalid or missing
    #[error("Configuration error: {detail}")]
    ConfigurationError {
        /// Details about the configuration error
        detail: String,
    },
    /// Input/output operation failed
    #[error("IO error: {detail}")]
    IoError {
        /// Details about the IO error
        detail: String,
    },
    /// Operation timed out
    #[error("Timeout error")]
    Timeout,
    /// Required resource was not found
    #[error("Resource not found: {resource}")]
    ResourceNotFound {
        /// Name of the missing resource
        resource: String,
    },
    /// Internal system error occurred
    #[error("Internal error: {detail}")]
    InternalError {
        /// Details about the internal error
        detail: String,
    },
}

/// Result type for formatting operations
pub type FormatResult<T> = Result<T, FormatError>;
