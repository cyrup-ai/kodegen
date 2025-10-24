//! Command parsing errors

use thiserror::Error;

/// Command parsing errors with owned strings
#[derive(Error, Debug, Clone)]
pub enum ParseError {
    /// Invalid command syntax
    #[error("Invalid command syntax: {detail}")]
    InvalidSyntax {
        /// Details about the syntax error
        detail: String,
    },

    /// Required parameter is missing
    #[error("Missing required parameter: {parameter}")]
    MissingParameter {
        /// Name of the missing parameter
        parameter: String,
    },

    /// Parameter has invalid value
    #[error("Invalid parameter value: {parameter} = {value}")]
    InvalidParameterValue {
        /// Name of the parameter
        parameter: String,
        /// The invalid value provided
        value: String,
    },

    /// Parameter name is not recognized
    #[error("Unknown parameter: {parameter}")]
    UnknownParameter {
        /// Name of the unknown parameter
        parameter: String,
    },

    /// Parameter type doesn't match expected type
    #[error("Parameter type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Expected parameter type
        expected: String,
        /// Actual parameter type provided
        actual: String,
    },
}

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;
