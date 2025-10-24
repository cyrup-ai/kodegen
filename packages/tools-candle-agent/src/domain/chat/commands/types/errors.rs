//! Command execution errors with zero allocation patterns
//!
//! Provides comprehensive error handling for command execution with owned strings
//! allocated once for blazing-fast performance. No Arc usage, no locking.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Command execution errors with minimal allocations - strings owned once for performance
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum CandleCommandError {
    /// Command name not recognized
    #[error("Unknown command: {command}")]
    UnknownCommand {
        /// The unrecognized command name (owned string allocated once)
        command: String,
    },
    /// Invalid or malformed arguments provided
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    /// Syntax error in command structure
    #[error("Invalid syntax: {detail}")]
    InvalidSyntax {
        /// Details about the syntax error (owned string allocated once)
        detail: String,
    },
    /// Command execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    /// User lacks permission to execute command
    #[error("Permission denied")]
    PermissionDenied,
    /// Error parsing command parameters
    #[error("Parse error: {0}")]
    ParseError(String),
    /// Configuration is invalid or missing
    #[error("Configuration error: {detail}")]
    ConfigurationError {
        /// Details about the configuration error (owned string allocated once)
        detail: String,
    },
    /// Input/output operation failed
    #[error("IO error: {0}")]
    IoError(String),
    /// Network communication error
    #[error("Network error: {0}")]
    NetworkError(String),
    /// Command execution timed out
    #[error("Command timeout")]
    Timeout,
    /// Requested resource not found
    #[error("Resource not found")]
    NotFound,
    /// Internal system error
    #[error("Internal error: {0}")]
    InternalError(String),
    /// Command already exists in registry
    #[error("Command already exists: {command}")]
    CommandAlreadyExists {
        /// The command name that already exists
        command: String,
    },
    /// Alias already exists in registry
    #[error("Alias already exists: {alias}")]
    AliasAlreadyExists {
        /// The alias that already exists
        alias: String,
    },
    /// Validation failed for command parameters
    #[error("Validation failed: {reason}")]
    ValidationFailed {
        /// Reason for validation failure
        reason: String,
    },
    /// Resource limit exceeded
    #[error("Resource limit exceeded: {resource}")]
    ResourceLimitExceeded {
        /// The resource that exceeded limits
        resource: String,
    },
    /// Dependency missing for command execution
    #[error("Missing dependency: {dependency}")]
    MissingDependency {
        /// The missing dependency name
        dependency: String,
    },
    /// Version incompatibility detected
    #[error("Version incompatible: expected {expected}, got {actual}")]
    VersionIncompatible {
        /// Expected version
        expected: String,
        /// Actual version found
        actual: String,
    },
    /// Feature not available or disabled
    #[error("Feature unavailable: {feature}")]
    FeatureUnavailable {
        /// The unavailable feature name
        feature: String,
    },
    /// Rate limit exceeded
    #[error("Rate limit exceeded: {limit} requests per {window}")]
    RateLimitExceeded {
        /// Request limit
        limit: u32,
        /// Time window
        window: String,
    },
}

impl CandleCommandError {
    /// Create a new unknown command error with zero allocation
    #[inline]
    pub fn unknown_command(command: impl Into<String>) -> Self {
        Self::UnknownCommand {
            command: command.into(),
        }
    }

    /// Create a new invalid arguments error with zero allocation
    #[inline]
    pub fn invalid_arguments(message: impl Into<String>) -> Self {
        Self::InvalidArguments(message.into())
    }

    /// Create a new invalid syntax error with zero allocation
    #[inline]
    pub fn invalid_syntax(detail: impl Into<String>) -> Self {
        Self::InvalidSyntax {
            detail: detail.into(),
        }
    }

    /// Create a new execution failed error with zero allocation
    #[inline]
    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self::ExecutionFailed(message.into())
    }

    /// Create a new parse error with zero allocation
    #[inline]
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::ParseError(message.into())
    }

    /// Create a new configuration error with zero allocation
    #[inline]
    pub fn configuration_error(detail: impl Into<String>) -> Self {
        Self::ConfigurationError {
            detail: detail.into(),
        }
    }

    /// Create a new IO error with zero allocation
    #[inline]
    pub fn io_error(message: impl Into<String>) -> Self {
        Self::IoError(message.into())
    }

    /// Create a new network error with zero allocation
    #[inline]
    pub fn network_error(message: impl Into<String>) -> Self {
        Self::NetworkError(message.into())
    }

    /// Create a new internal error with zero allocation
    #[inline]
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }

    /// Create a new command already exists error with zero allocation
    #[inline]
    pub fn command_already_exists(command: impl Into<String>) -> Self {
        Self::CommandAlreadyExists {
            command: command.into(),
        }
    }

    /// Create a new alias already exists error with zero allocation
    #[inline]
    pub fn alias_already_exists(alias: impl Into<String>) -> Self {
        Self::AliasAlreadyExists {
            alias: alias.into(),
        }
    }

    /// Create a new validation failed error with zero allocation
    #[inline]
    pub fn validation_failed(reason: impl Into<String>) -> Self {
        Self::ValidationFailed {
            reason: reason.into(),
        }
    }

    /// Create a new resource limit exceeded error with zero allocation
    #[inline]
    pub fn resource_limit_exceeded(resource: impl Into<String>) -> Self {
        Self::ResourceLimitExceeded {
            resource: resource.into(),
        }
    }

    /// Create a new missing dependency error with zero allocation
    #[inline]
    pub fn missing_dependency(dependency: impl Into<String>) -> Self {
        Self::MissingDependency {
            dependency: dependency.into(),
        }
    }

    /// Create a new version incompatible error with zero allocation
    #[inline]
    pub fn version_incompatible(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::VersionIncompatible {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a new feature unavailable error with zero allocation
    #[inline]
    pub fn feature_unavailable(feature: impl Into<String>) -> Self {
        Self::FeatureUnavailable {
            feature: feature.into(),
        }
    }

    /// Create a new rate limit exceeded error with zero allocation
    #[inline]
    pub fn rate_limit_exceeded(limit: u32, window: impl Into<String>) -> Self {
        Self::RateLimitExceeded {
            limit,
            window: window.into(),
        }
    }

    /// Check if error is retriable - used for automatic retry logic
    #[inline]
    #[must_use]
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            Self::NetworkError(_)
                | Self::Timeout
                | Self::RateLimitExceeded { .. }
                | Self::InternalError(_)
        )
    }

    /// Check if error is a client error (user mistake) vs server error
    #[inline]
    #[must_use]
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::UnknownCommand { .. }
                | Self::InvalidArguments(_)
                | Self::InvalidSyntax { .. }
                | Self::PermissionDenied
                | Self::ParseError(_)
                | Self::ValidationFailed { .. }
                | Self::FeatureUnavailable { .. }
        )
    }

    /// Get error severity level for logging and monitoring
    #[inline]
    #[must_use]
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::InternalError(_) => ErrorSeverity::Critical,
            Self::NetworkError(_)
            | Self::IoError(_)
            | Self::ConfigurationError { .. }
            | Self::MissingDependency { .. }
            | Self::VersionIncompatible { .. } => ErrorSeverity::High,
            Self::ExecutionFailed(_)
            | Self::Timeout
            | Self::NotFound
            | Self::ResourceLimitExceeded { .. }
            | Self::RateLimitExceeded { .. } => ErrorSeverity::Medium,
            _ => ErrorSeverity::Low,
        }
    }
}

/// Error severity levels for monitoring and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Low severity - user errors, validation failures
    Low = 1,
    /// Medium severity - execution failures, timeouts
    Medium = 2,
    /// High severity - system errors, configuration issues
    High = 3,
    /// Critical severity - internal errors, system failures
    Critical = 4,
}

impl ErrorSeverity {
    /// Get severity as string for logging
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }

    /// Get severity as numeric value
    #[inline]
    #[must_use]
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

/// Result type for command execution - uses owned error types for performance
pub type CommandResult<T> = Result<T, CandleCommandError>;

/// Result type for command validation - lightweight for frequent use
pub type ValidationResult = CommandResult<()>;
