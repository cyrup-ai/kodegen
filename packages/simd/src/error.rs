//! Error types for SIMD operations

use thiserror::Error;

/// Error type for SIMD operations
#[derive(Debug, Clone, Error)]
pub enum SimdError {
    /// Invalid input parameter
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Processing error
    #[error("Processing error: {0}")]
    ProcessingError(String),

    /// Numerical error (overflow, underflow, etc.)
    #[error("Numerical error: {0}")]
    NumericalError(String),

    /// Unsupported operation for current platform
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Invalid input length
    #[error("Invalid input length: expected {expected}, got {actual}")]
    InvalidInputLength {
        /// Expected length
        expected: usize,
        /// Actual length
        actual: usize,
    },

    /// Invalid probability distribution
    #[error("Invalid probability distribution: {0}")]
    InvalidProbabilities(String),

    /// Platform-specific error
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Memory allocation error
    #[error("Memory allocation error: {0}")]
    MemoryError(String),
}

/// Result type for SIMD operations
pub type SimdResult<T> = Result<T, SimdError>;

impl From<Box<dyn std::error::Error + Send + Sync>> for SimdError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        SimdError::ProcessingError(err.to_string())
    }
}

impl From<std::num::ParseFloatError> for SimdError {
    fn from(err: std::num::ParseFloatError) -> Self {
        SimdError::NumericalError(format!("Float parsing error: {err}"))
    }
}

impl From<std::num::ParseIntError> for SimdError {
    fn from(err: std::num::ParseIntError) -> Self {
        SimdError::NumericalError(format!("Integer parsing error: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = SimdError::InvalidInput("test input".to_string());
        assert_eq!(error.to_string(), "Invalid input: test input");

        let error = SimdError::InvalidInputLength {
            expected: 10,
            actual: 5,
        };
        assert_eq!(
            error.to_string(),
            "Invalid input length: expected 10, got 5"
        );
    }

    #[test]
    fn test_error_conversion() {
        let parse_error: Result<f32, _> = "invalid".parse();
        let simd_error: SimdError = parse_error.unwrap_err().into();
        assert!(matches!(simd_error, SimdError::NumericalError(_)));
    }
}
