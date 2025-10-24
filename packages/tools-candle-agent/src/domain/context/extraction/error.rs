//! Error types for the extraction module

use std::time::Duration;

use thiserror::Error;

/// Error types for extraction operations
#[derive(Debug, Error)]
pub enum ExtractionError {
    /// JSON parsing error during extraction
    #[error("Failed to parse JSON response: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Model completion error
    #[error("Model completion failed: {0}")]
    CompletionError(String),

    /// Timeout during extraction
    #[error("Extraction timeout after {duration:?}")]
    Timeout { duration: Duration },

    /// Invalid response format
    #[error("Invalid response format: expected JSON object, got {actual}")]
    InvalidFormat { actual: String },

    /// Missing required fields in response
    #[error("Response missing required fields: {fields:?}")]
    MissingFields { fields: Vec<String> },

    /// Validation error for extracted data
    #[error("Validation failed: {reason}")]
    ValidationFailed { reason: String },

    /// Generic error for other cases
    #[error("Extraction error: {0}")]
    Other(String),
}

/// Result type for extraction operations (planned API)
pub type _ExtractionResult<T> = Result<T, ExtractionError>;

impl ExtractionError {
    /// Create a new validation error
    pub fn validation_failed(reason: impl Into<String>) -> Self {
        Self::ValidationFailed {
            reason: reason.into(),
        }
    }

    /// Create a new missing fields error
    #[must_use]
    pub fn missing_fields(fields: &[&str]) -> Self {
        Self::MissingFields {
            fields: fields.iter().map(|&s| s.to_string()).collect(),
        }
    }
}
