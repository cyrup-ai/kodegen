//! State persistence and lifecycle action types
//!
//! Provides action enums for session and history management.

use serde::{Deserialize, Serialize};

/// Session-related action enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionAction {
    /// List available sessions
    List,
    /// Create a new session
    New,
    /// Switch to a different session
    Switch,
    /// Delete a session
    Delete,
    /// Export session data
    Export,
    /// Import session data
    Import,
    /// Show session details
    Show,
    /// Archive session
    Archive,
    /// Backup session
    Backup,
    /// Restore session from backup
    Restore,
    /// Clone session with new name
    Clone,
    /// Rename session
    Rename,
}

impl SessionAction {
    /// Get action name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::List => "list",
            Self::New => "new",
            Self::Switch => "switch",
            Self::Delete => "delete",
            Self::Export => "export",
            Self::Import => "import",
            Self::Show => "show",
            Self::Archive => "archive",
            Self::Backup => "backup",
            Self::Restore => "restore",
            Self::Clone => "clone",
            Self::Rename => "rename",
        }
    }

    /// Check if action requires session name parameter
    #[inline]
    #[must_use]
    pub const fn requires_name(&self) -> bool {
        matches!(
            self,
            Self::New
                | Self::Switch
                | Self::Delete
                | Self::Export
                | Self::Show
                | Self::Archive
                | Self::Backup
                | Self::Clone
                | Self::Rename
        )
    }

    /// Check if action modifies sessions
    #[inline]
    #[must_use]
    pub const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::New
                | Self::Switch
                | Self::Delete
                | Self::Import
                | Self::Archive
                | Self::Restore
                | Self::Clone
                | Self::Rename
        )
    }
}

/// History action enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HistoryAction {
    /// Show history entries
    Show,
    /// Search through history
    Search,
    /// Clear history
    Clear,
    /// Export history data
    Export,
    /// Import history data
    Import,
    /// Backup history
    Backup,
    /// Restore from backup
    Restore,
    /// Compact history storage
    Compact,
    /// Analyze history patterns
    Analyze,
    /// Show history statistics
    Stats,
}

impl HistoryAction {
    /// Get action name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Search => "search",
            Self::Clear => "clear",
            Self::Export => "export",
            Self::Import => "import",
            Self::Backup => "backup",
            Self::Restore => "restore",
            Self::Compact => "compact",
            Self::Analyze => "analyze",
            Self::Stats => "stats",
        }
    }

    /// Check if action modifies history
    #[inline]
    #[must_use]
    pub const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Clear | Self::Import | Self::Restore | Self::Compact
        )
    }

    /// Check if action supports time range filtering
    #[inline]
    #[must_use]
    pub const fn supports_time_range(&self) -> bool {
        matches!(
            self,
            Self::Show | Self::Search | Self::Export | Self::Backup | Self::Analyze | Self::Stats
        )
    }
}
