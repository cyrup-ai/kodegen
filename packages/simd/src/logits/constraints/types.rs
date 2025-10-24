//! Shared types and error handling for generation constraints

use std::fmt;

/// Errors that can occur during constraint processing
#[derive(Debug, Clone)]
pub enum ConstraintError {
    /// Invalid token for current constraint state
    InvalidToken {
        /// The invalid token ID
        token: u32,
        /// Description of expected token(s)
        expected: String,
    },
    /// Constraint validation failed
    ValidationFailed(String),
    /// Tokenizer error during constraint processing
    TokenizerError(String),
    /// Internal constraint state error
    StateError(String),
}

impl fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken { token, expected } => {
                write!(f, "Invalid token {} (expected: {})", token, expected)
            }
            Self::ValidationFailed(msg) => write!(f, "Constraint validation failed: {}", msg),
            Self::TokenizerError(msg) => write!(f, "Tokenizer error: {}", msg),
            Self::StateError(msg) => write!(f, "Constraint state error: {}", msg),
        }
    }
}

impl std::error::Error for ConstraintError {}
