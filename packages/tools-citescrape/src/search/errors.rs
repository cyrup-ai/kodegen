//! Error types for search operations
//!
//! This module defines tantivy-specific error types with retry logic,
//! logging, and fallback behaviors for robust search operations.

use std::time::Duration;
use tantivy::TantivyError;
use thiserror::Error;

/// Result type alias for search operations
pub type SearchResult<T> = Result<T, SearchError>;

/// Error types for search operations
#[derive(Debug, Error)]
pub enum SearchError {
    /// Index initialization failed
    #[error("Failed to initialize search index: {0}")]
    IndexInitialization(String),

    /// Index corruption detected
    #[error("Search index corruption detected: {0}")]
    IndexCorruption(String),

    /// Query parsing failed
    #[error("Invalid search query: {0}")]
    QueryParsing(String),

    /// Search execution failed
    #[error("Search execution failed: {0}")]
    SearchExecution(String),

    /// Indexing operation failed
    #[error("Indexing failed for document {doc_id}: {message}")]
    IndexingFailed { doc_id: String, message: String },

    /// Index writer acquisition failed (transient)
    #[error("Failed to acquire index writer (retry recommended): {0}")]
    WriterAcquisition(String),

    /// Index commit failed
    #[error("Failed to commit index changes: {0}")]
    CommitFailed(String),

    /// Field not found in schema
    #[error("Field '{0}' not found in index schema")]
    FieldNotFound(String),

    /// Document not found
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Tantivy error wrapper
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] TantivyError),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for SearchError {
    fn from(error: anyhow::Error) -> Self {
        SearchError::Other(error.to_string())
    }
}

impl SearchError {
    /// Check if error is transient and should be retried
    #[must_use]
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            SearchError::WriterAcquisition(_) | SearchError::Io(_) | SearchError::CommitFailed(_)
        )
    }

    /// Get suggested retry delay for transient errors
    #[must_use]
    pub fn retry_delay(&self) -> Option<Duration> {
        if self.is_transient() {
            Some(Duration::from_millis(100))
        } else {
            None
        }
    }

    /// Check if index rebuild is recommended
    #[must_use]
    pub fn needs_index_rebuild(&self) -> bool {
        matches!(self, SearchError::IndexCorruption(_))
    }
}

/// Retry configuration for search operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Maximum retry delay
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(5),
        }
    }
}

impl RetryConfig {
    /// Calculate delay for given attempt number (0-based)
    #[must_use]
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let multiplier = self.backoff_multiplier.powi(attempt as i32);
        let delay_ms = (self.initial_delay.as_millis() as f64 * multiplier) as u64;
        let delay = Duration::from_millis(delay_ms);

        // Cap at max_delay
        if delay > self.max_delay {
            self.max_delay
        } else {
            delay
        }
    }
}

/// Helper macro for logging search operations with performance metrics
#[macro_export]
macro_rules! log_search_operation {
    ($op:expr, $query:expr) => {{
        let start = std::time::Instant::now();
        let result = $op;
        let duration = start.elapsed();

        match &result {
            Ok(_) => {
                tracing::debug!(
                    query = %$query,
                    duration_ms = duration.as_millis(),
                    "Search operation completed successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    query = %$query,
                    duration_ms = duration.as_millis(),
                    error = %e,
                    "Search operation failed"
                );
            }
        }

        result
    }};
}

// Note: retry_with_config and with_fallback have been moved to runtime_helpers.rs
// as pure async functions. Use retry_task and fallback_task instead.
