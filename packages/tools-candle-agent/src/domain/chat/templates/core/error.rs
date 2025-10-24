//! Template error types

use thiserror::Error;

/// Core template error types
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TemplateError {
    /// Template was not found
    #[error("Template not found: {name}")]
    NotFound {
        /// Name of the template that was not found
        name: String,
    },

    /// Error occurred during template parsing
    #[error("Parse error: {message}")]
    ParseError {
        /// Details about the parsing error
        message: String,
    },

    /// Error occurred during template compilation
    #[error("Compile error: {message}")]
    CompileError {
        /// Details about the compilation error
        message: String,
    },

    /// Error occurred during template rendering
    #[error("Render error: {message}")]
    RenderError {
        /// Details about the rendering error
        message: String,
    },

    /// Error related to template variables
    #[error("Variable error: {message}")]
    VariableError {
        /// Details about the variable error
        message: String,
    },

    /// Operation was denied due to insufficient permissions
    #[error("Permission denied: {message}")]
    PermissionDenied {
        /// Details about the permission denial
        message: String,
    },

    /// Error occurred during storage operations
    #[error("Storage error: {message}")]
    StorageError {
        /// Details about the storage error
        message: String,
    },
}

/// Template result type
pub type TemplateResult<T> = Result<T, TemplateError>;
