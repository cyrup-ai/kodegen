//! Validation error types for command validation
//!
//! Provides comprehensive error types for all validation failure scenarios
//! with detailed context and user-friendly error messages.

/// Validation error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    /// Parameter cannot be empty
    #[error("Parameter '{parameter}' cannot be empty")]
    EmptyParameter {
        /// Parameter name
        parameter: String,
    },

    /// Parameter exceeds maximum length
    #[error("Parameter '{parameter}' is too long: {actual_length} > {max_length}")]
    ParameterTooLong {
        /// Parameter name
        parameter: String,
        /// Maximum allowed length
        max_length: usize,
        /// Actual length provided
        actual_length: usize,
    },

    /// Parameter value is out of range
    #[error("Parameter '{parameter}' is out of range: {value} (min: {min:?}, max: {max:?})")]
    ParameterOutOfRange {
        /// Parameter name
        parameter: String,
        /// Parameter value provided
        value: String,
        /// Minimum allowed value
        min: Option<String>,
        /// Maximum allowed value
        max: Option<String>,
    },

    /// Parameter has invalid enum value
    #[error("Parameter '{parameter}' has invalid value '{value}', allowed: {allowed_values:?}")]
    InvalidEnumValue {
        /// Parameter name
        parameter: String,
        /// Invalid value provided
        value: String,
        /// List of allowed values
        allowed_values: Vec<String>,
    },

    /// Parameter has invalid format
    #[error("Parameter '{parameter}' has invalid format '{value}', expected: {expected_format}")]
    InvalidParameterFormat {
        /// Parameter name
        parameter: String,
        /// Invalid value provided
        value: String,
        /// Expected format description
        expected_format: String,
    },

    /// Parameter has invalid file extension
    #[error(
        "Parameter '{parameter}' has invalid file extension '{extension}', allowed: {allowed_extensions:?}"
    )]
    InvalidFileExtension {
        /// Parameter name with invalid extension
        parameter: String,
        /// The invalid extension that was provided
        extension: String,
        /// List of allowed file extensions
        allowed_extensions: Vec<String>,
    },

    /// Too many parameters provided
    #[error("Too many parameters: {actual_count} > {max_count}")]
    TooManyParameters {
        /// Maximum allowed parameter count
        max_count: usize,
        /// Actual parameter count provided
        actual_count: usize,
    },

    /// Security violation detected in parameter
    #[error("Security violation in parameter '{parameter}': {detail}")]
    SecurityViolation {
        /// Parameter name where violation occurred
        parameter: String,
        /// Details of the security violation
        detail: String,
    },
}
