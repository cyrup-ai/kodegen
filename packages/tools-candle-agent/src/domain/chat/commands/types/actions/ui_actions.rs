//! User interface configuration action types
//!
//! Provides action enums for theme customization.

use serde::{Deserialize, Serialize};

/// Theme action enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThemeAction {
    /// Set active theme
    Set,
    /// List available themes
    List,
    /// Create a new theme
    Create,
    /// Export theme data
    Export,
    /// Import theme data
    Import,
    /// Edit existing theme
    Edit,
    /// Delete theme
    Delete,
    /// Reset to default theme
    Reset,
    /// Preview theme changes
    Preview,
    /// Clone existing theme
    Clone,
}

impl ThemeAction {
    /// Get action name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::List => "list",
            Self::Create => "create",
            Self::Export => "export",
            Self::Import => "import",
            Self::Edit => "edit",
            Self::Delete => "delete",
            Self::Reset => "reset",
            Self::Preview => "preview",
            Self::Clone => "clone",
        }
    }

    /// Check if action requires theme name parameter
    #[inline]
    #[must_use]
    pub const fn requires_name(&self) -> bool {
        matches!(
            self,
            Self::Set
                | Self::Create
                | Self::Export
                | Self::Edit
                | Self::Delete
                | Self::Preview
                | Self::Clone
        )
    }

    /// Check if action modifies themes
    #[inline]
    #[must_use]
    pub const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Set
                | Self::Create
                | Self::Import
                | Self::Edit
                | Self::Delete
                | Self::Reset
                | Self::Clone
        )
    }
}
