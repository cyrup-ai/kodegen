//! GitHub API error types

use thiserror::Error;

/// Error types for GitHub API operations
#[derive(Debug, Error)]
pub enum GitHubError {
    /// Octocrab library error
    #[error("Octocrab error: {0}")]
    Octocrab(#[from] octocrab::Error),

    /// Generic GitHub API error
    #[error("GitHub API error: {0}")]
    Api(String),

    /// Invalid input parameters
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Resource not found (404)
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Authentication required or failed
    #[error("Authentication required")]
    AuthRequired,

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Client setup/configuration error
    #[error("Client setup failed: {0}")]
    ClientSetup(String),

    /// Custom error with message
    #[error("{0}")]
    Custom(String),

    /// Other error with message
    #[error("{0}")]
    Other(String),
}

/// Convenience result alias for GitHub operations
pub type GitHubResult<T> = Result<T, GitHubError>;

// Convenience conversions
impl From<String> for GitHubError {
    fn from(s: String) -> Self {
        GitHubError::Api(s)
    }
}

impl From<&str> for GitHubError {
    fn from(s: &str) -> Self {
        GitHubError::Api(s.to_string())
    }
}
