//! Streaming events for message formatting

use std::time::Duration;

use cyrup_sugars::prelude::MessageChunk;

use crate::domain::chat::formatting::content::ImmutableMessageContent;
use crate::domain::chat::formatting::error::FormatError;

/// Formatting event for streaming operations
#[derive(Debug, Clone)]
pub enum FormattingEvent {
    /// Formatting started
    Started {
        /// Unique identifier for the content being formatted
        content_id: u64,
        /// Type of content being formatted
        content_type: String,
        /// Timestamp when formatting started (nanoseconds)
        timestamp_nanos: u64,
    },
    /// Formatting progress
    Progress {
        /// Unique identifier for the content being formatted
        content_id: u64,
        /// Progress percentage (0.0 to 100.0)
        progress_percent: f32,
        /// Current formatting stage description
        stage: String,
    },
    /// Formatting completed
    Completed {
        /// Unique identifier for the content that was formatted
        content_id: u64,
        /// Final formatted content result
        result: ImmutableMessageContent,
        /// Total formatting duration
        duration: Duration,
    },
    /// Formatting failed
    Failed {
        /// Unique identifier for the content that failed to format
        content_id: u64,
        /// Error that caused the formatting to fail
        error: FormatError,
        /// Duration before failure occurred
        duration: Duration,
    },
    /// Partial result available
    PartialResult {
        /// Unique identifier for the content being formatted
        content_id: u64,
        /// Partially formatted content available so far
        partial_content: String,
    },
}

impl Default for FormattingEvent {
    fn default() -> Self {
        Self::Started {
            content_id: 0,
            content_type: "default".to_string(),
            timestamp_nanos: 0,
        }
    }
}

impl MessageChunk for FormattingEvent {
    fn bad_chunk(error: String) -> Self {
        Self::Failed {
            content_id: 0,
            error: FormatError::RenderError { detail: error },
            duration: Duration::ZERO,
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            Self::Failed { .. } => Some("Formatting failed"),
            _ => None,
        }
    }
}
