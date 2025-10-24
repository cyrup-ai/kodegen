use thiserror::Error;

use crate::agent::AgentError;
use crate::browser::BrowserError;

/// Errors that can occur in utility functions
#[derive(Error, Debug)]
pub enum UtilsError {
    #[error("Missing API key: {0}")]
    #[allow(dead_code)] // Reserved for future LLM provider integrations
    MissingApiKey(String),

    #[error("Unsupported provider: {0}")]
    #[allow(dead_code)] // Reserved for future LLM provider integrations
    UnsupportedProvider(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Model error: {0}")]
    #[allow(dead_code)] // Reserved for future model-specific error handling
    ModelError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Browser error: {0}")]
    BrowserError(String),

    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("JSON parse error: {0}")]
    JsonParseError(String),

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}

/// Implement From<BrowserError> for UtilsError
impl From<BrowserError> for UtilsError {
    fn from(err: BrowserError) -> Self {
        UtilsError::BrowserError(err.to_string())
    }
}

/// Implement From<AgentError> for UtilsError
impl From<AgentError> for UtilsError {
    fn from(err: AgentError) -> Self {
        UtilsError::AgentError(err.to_string())
    }
}

/// Implement From<serde_json::Error> for UtilsError
impl From<serde_json::Error> for UtilsError {
    fn from(err: serde_json::Error) -> Self {
        UtilsError::JsonParseError(err.to_string())
    }
}

/// Implement From<std::io::Error> for UtilsError
impl From<std::io::Error> for UtilsError {
    fn from(err: std::io::Error) -> Self {
        UtilsError::IoError(err.to_string())
    }
}
