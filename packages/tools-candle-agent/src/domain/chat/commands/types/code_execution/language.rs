//! Programming language definitions and utilities

use serde::{Deserialize, Serialize};

/// Programming language enumeration for code execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodeLanguage {
    /// Python programming language
    Python,
    /// JavaScript programming language
    JavaScript,
    /// Rust programming language
    Rust,
    /// Go programming language
    Go,
    /// Bash shell scripting
    Bash,
}

impl CodeLanguage {
    /// Get language name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Bash => "bash",
        }
    }

    /// Get file extension for the language
    #[inline]
    #[must_use]
    pub const fn file_extension(&self) -> &'static str {
        match self {
            Self::Python => "py",
            Self::JavaScript => "js",
            Self::Rust => "rs",
            Self::Go => "go",
            Self::Bash => "sh",
        }
    }

    /// Get MIME type for the language
    #[inline]
    #[must_use]
    pub const fn mime_type(&self) -> &'static str {
        match self {
            Self::Python => "text/x-python",
            Self::JavaScript => "text/javascript",
            Self::Rust => "text/x-rust",
            Self::Go => "text/x-go",
            Self::Bash => "text/x-shellscript",
        }
    }
}
