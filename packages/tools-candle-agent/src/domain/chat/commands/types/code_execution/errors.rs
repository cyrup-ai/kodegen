//! Validation error types for code execution

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Validation error for code execution with comprehensive error handling
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    /// Syntax validation failed
    #[error("Syntax validation failed: {message}")]
    SyntaxValidationFailed { message: String },

    /// Security validation failed
    #[error("Security validation failed: {message}")]
    SecurityValidationFailed { message: String },

    /// Resource limit validation failed
    #[error("Resource limit validation failed: {message}")]
    ResourceLimitValidationFailed { message: String },

    /// Language not supported
    #[error("Language not supported: {language}")]
    LanguageNotSupported { language: String },

    /// Code contains prohibited patterns
    #[error("Code contains prohibited patterns: {patterns:?}")]
    ProhibitedPatterns { patterns: Vec<String> },

    /// Code exceeds size limits
    #[error("Code exceeds size limit: {size} bytes (max: {limit} bytes)")]
    CodeSizeExceeded { size: usize, limit: usize },

    /// Invalid encoding detected
    #[error("Invalid encoding detected: {encoding}")]
    InvalidEncoding { encoding: String },

    /// Timeout value invalid
    #[error("Invalid timeout value: {timeout} seconds (max: {max_timeout} seconds)")]
    InvalidTimeout { timeout: u64, max_timeout: u64 },

    /// Memory limit invalid
    #[error("Invalid memory limit: {memory} bytes (max: {max_memory} bytes)")]
    InvalidMemoryLimit { memory: u64, max_memory: u64 },

    /// CPU limit invalid
    #[error("Invalid CPU limit: {cpu}% (max: {max_cpu}%)")]
    InvalidCpuLimit { cpu: u8, max_cpu: u8 },

    /// Environment variable validation failed
    #[error("Environment variable validation failed: {variable} = {value}")]
    InvalidEnvironmentVariable { variable: String, value: String },

    /// Working directory validation failed
    #[error("Working directory validation failed: {directory}")]
    InvalidWorkingDirectory { directory: String },
}

impl ValidationError {
    /// Create a syntax validation error
    #[inline]
    pub fn syntax_failed(message: impl Into<String>) -> Self {
        Self::SyntaxValidationFailed {
            message: message.into(),
        }
    }

    /// Create a security validation error
    #[inline]
    pub fn security_failed(message: impl Into<String>) -> Self {
        Self::SecurityValidationFailed {
            message: message.into(),
        }
    }

    /// Create a resource limit validation error
    #[inline]
    pub fn resource_limit_failed(message: impl Into<String>) -> Self {
        Self::ResourceLimitValidationFailed {
            message: message.into(),
        }
    }

    /// Create a language not supported error
    #[inline]
    pub fn language_not_supported(language: impl Into<String>) -> Self {
        Self::LanguageNotSupported {
            language: language.into(),
        }
    }

    /// Create a prohibited patterns error
    #[inline]
    #[must_use]
    pub fn prohibited_patterns(patterns: Vec<String>) -> Self {
        Self::ProhibitedPatterns { patterns }
    }

    /// Create a code size exceeded error
    #[inline]
    #[must_use]
    pub fn code_size_exceeded(size: usize, limit: usize) -> Self {
        Self::CodeSizeExceeded { size, limit }
    }
}
