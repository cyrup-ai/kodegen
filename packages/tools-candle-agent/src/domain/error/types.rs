//! Fundamental error types and classification

use std::fmt;

/// Maximum length for error messages to ensure zero allocation
pub const MAX_ERROR_MESSAGE_LEN: usize = 256;

/// Error category for structured error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Memory-related errors
    Memory,
    /// Network and communication errors
    Network,
    /// Configuration and initialization errors
    Config,
    /// System-level errors
    System,
    /// User input and validation errors
    User,
    /// Timeout and deadline errors
    Timeout,
    /// Resource exhaustion errors
    Resource,
    /// Serialization and data format errors
    Serialization,
    /// Authentication and authorization errors
    Auth,
    /// Unknown or unclassified errors
    Unknown,
}

/// Error severity levels for prioritization
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Low severity - informational
    Info,
    /// Medium severity - warning
    Warning,
    /// High severity - error
    Error,
    /// Critical severity - system failure
    Critical,
}

/// Error recoverability classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorRecoverability {
    /// Error can be retried immediately
    Retriable,
    /// Error requires exponential backoff
    RetriableWithBackoff,
    /// Error is permanent and should not be retried
    Permanent,
    /// Error requires manual intervention
    Manual,
}

/// Zero-allocation error message with const generic length
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeroAllocMessage<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> ZeroAllocMessage<N> {
    /// Create new zero-allocation message
    #[inline]
    #[must_use]
    pub const fn new(message: &str) -> Self {
        let bytes = message.as_bytes();
        let len = if bytes.len() > N { N } else { bytes.len() };

        let mut data = [0u8; N];
        let mut i = 0;
        while i < len {
            data[i] = bytes[i];
            i += 1;
        }

        Self { data, len }
    }

    /// Get message as string slice with safe UTF-8 validation
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        // Safe UTF-8 validation - returns valid UTF-8 or replacement string
        std::str::from_utf8(&self.data[..self.len]).unwrap_or("Invalid UTF-8 in error message")
    }

    /// Check if message is empty
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get message length
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }
}

impl<const N: usize> fmt::Display for ZeroAllocMessage<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Default error message type
pub type ErrorMessage = ZeroAllocMessage<MAX_ERROR_MESSAGE_LEN>;
