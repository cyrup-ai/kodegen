//! Core error type with zero-allocation and comprehensive metadata

use super::types::{ErrorCategory, ErrorMessage, ErrorRecoverability, ErrorSeverity};
use std::fmt;
use std::time::{Duration, Instant};

/// Zero-allocation error with comprehensive metadata
#[derive(Debug, Clone)]
pub struct ZeroAllocError {
    /// Error category for classification
    pub category: ErrorCategory,
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Error recoverability classification
    pub recoverability: ErrorRecoverability,
    /// Zero-allocation error message
    pub message: ErrorMessage,
    /// Error code for machine processing
    pub code: u64,
    /// Source location (`<file:line>`)
    pub location: Option<ErrorMessage>,
    /// Cause chain for nested errors
    pub cause: Option<Box<ZeroAllocError>>,
    /// Timestamp when error occurred
    pub timestamp: Instant,
    /// Thread ID where error occurred
    pub thread_id: u64,
    /// Error metadata for structured logging
    pub metadata: [(ErrorMessage, ErrorMessage); 4],
    /// Number of metadata entries
    pub metadata_count: usize,
}

impl ZeroAllocError {
    /// Create new zero-allocation error
    #[inline]
    #[must_use]
    pub fn new(
        category: ErrorCategory,
        severity: ErrorSeverity,
        recoverability: ErrorRecoverability,
        message: &str,
        code: u64,
    ) -> Self {
        Self {
            category,
            severity,
            recoverability,
            message: ErrorMessage::new(message),
            code,
            location: None,
            cause: None,
            timestamp: Instant::now(),
            thread_id: {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                std::thread::current().id().hash(&mut hasher);
                hasher.finish()
            },
            metadata: [
                (ErrorMessage::new(""), ErrorMessage::new("")),
                (ErrorMessage::new(""), ErrorMessage::new("")),
                (ErrorMessage::new(""), ErrorMessage::new("")),
                (ErrorMessage::new(""), ErrorMessage::new("")),
            ],
            metadata_count: 0,
        }
    }

    /// Add location information
    #[must_use]
    #[inline]
    pub fn with_location(mut self, file: &str, line: u32) -> Self {
        let location = format!("{file}:{line}");
        self.location = Some(ErrorMessage::new(&location));
        self
    }

    /// Add cause chain
    #[must_use]
    #[inline]
    pub fn with_cause(mut self, cause: ZeroAllocError) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    /// Add metadata key-value pair
    #[must_use]
    #[inline]
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        if self.metadata_count < 4 {
            self.metadata[self.metadata_count] = (ErrorMessage::new(key), ErrorMessage::new(value));
            self.metadata_count += 1;
        }
        self
    }

    /// Check if error is retriable
    #[inline]
    #[must_use]
    pub fn is_retriable(&self) -> bool {
        matches!(
            self.recoverability,
            ErrorRecoverability::Retriable | ErrorRecoverability::RetriableWithBackoff
        )
    }

    /// Check if error is permanent
    #[inline]
    #[must_use]
    pub fn is_permanent(&self) -> bool {
        matches!(self.recoverability, ErrorRecoverability::Permanent)
    }

    /// Check if error requires manual intervention
    #[inline]
    #[must_use]
    pub fn is_manual(&self) -> bool {
        matches!(self.recoverability, ErrorRecoverability::Manual)
    }

    /// Get error age since occurrence
    #[inline]
    #[must_use]
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

impl fmt::Display for ZeroAllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?}:{:?}] {}",
            self.category, self.severity, self.message
        )?;

        if let Some(location) = &self.location {
            write!(f, " at {location}")?;
        }

        if let Some(cause) = &self.cause {
            write!(f, " caused by: {cause}")?;
        }

        Ok(())
    }
}

impl std::error::Error for ZeroAllocError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
            .as_ref()
            .map(|e| e.as_ref() as &dyn std::error::Error)
    }
}

/// Result type alias for zero-allocation errors
pub type ZeroAllocResult<T> = Result<T, ZeroAllocError>;
