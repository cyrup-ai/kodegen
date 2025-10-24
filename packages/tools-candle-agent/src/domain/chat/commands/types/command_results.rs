//! Command execution result types with zero allocation patterns
//!
//! Provides blazing-fast result enumeration with owned strings allocated once
//! for maximum performance. Rich constructors and query methods included.

use serde::{Deserialize, Serialize};

use super::command_enums::OutputType;

/// Command execution result with zero allocation patterns where possible
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandExecutionResult {
    /// Simple success message (owned string allocated once)
    Success(String),
    /// Data result with structured output
    Data(serde_json::Value),
    /// File result with path and metadata (owned strings allocated once)
    File {
        /// File path
        path: String,
        /// File size in bytes
        size_bytes: u64,
        /// MIME type of the file
        mime_type: String,
    },
    /// Multiple results (owned collection allocated once)
    Multiple(Vec<CommandExecutionResult>),
    /// Stream result for continuous output
    Stream {
        /// Stream identifier
        stream_id: String,
        /// Stream type
        stream_type: OutputType,
        /// Initial data if available
        initial_data: Option<String>,
    },
    /// Error result (owned string allocated once)
    Error(String),
}

impl CommandExecutionResult {
    /// Create success result with zero allocation constructor
    #[inline]
    pub fn success(message: impl Into<String>) -> Self {
        Self::Success(message.into())
    }

    /// Create data result with JSON value
    #[inline]
    #[must_use]
    pub fn data(value: serde_json::Value) -> Self {
        Self::Data(value)
    }

    /// Create file result with zero allocation constructor
    #[inline]
    pub fn file(path: impl Into<String>, size_bytes: u64, mime_type: impl Into<String>) -> Self {
        Self::File {
            path: path.into(),
            size_bytes,
            mime_type: mime_type.into(),
        }
    }

    /// Create multiple results
    #[inline]
    #[must_use]
    pub fn multiple(results: Vec<CommandExecutionResult>) -> Self {
        Self::Multiple(results)
    }

    /// Create stream result with zero allocation constructor
    #[inline]
    pub fn stream(
        stream_id: impl Into<String>,
        stream_type: OutputType,
        initial_data: Option<String>,
    ) -> Self {
        Self::Stream {
            stream_id: stream_id.into(),
            stream_type,
            initial_data,
        }
    }

    /// Create error result with zero allocation constructor
    #[inline]
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error(message.into())
    }

    /// Check if result indicates success
    #[inline]
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            Self::Success(_)
                | Self::Data(_)
                | Self::File { .. }
                | Self::Multiple(_)
                | Self::Stream { .. }
        )
    }

    /// Check if result indicates error
    #[inline]
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}
