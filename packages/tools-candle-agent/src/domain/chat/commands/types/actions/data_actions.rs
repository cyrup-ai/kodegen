//! Data query and operation action types
//!
//! Provides action enums for search, statistics, and import operations.

use serde::{Deserialize, Serialize};

/// Search scope enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchScope {
    /// Search all content
    All,
    /// Search current session only
    Session,
    /// Search current conversation
    Current,
    /// Search recent conversations
    Recent,
    /// Search bookmarked conversations
    Bookmarked,
    /// Search user messages only
    User,
    /// Search assistant messages only
    Assistant,
    /// Search system messages only
    System,
    /// Search current branch only
    Branch,
    /// Search specific time range
    TimeRange,
    /// Search by message type
    MessageType,
    /// Search by tags
    Tags,
    /// Search archived content
    Archived,
}

impl SearchScope {
    /// Get scope name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Session => "session",
            Self::Current => "current",
            Self::Recent => "recent",
            Self::Bookmarked => "bookmarked",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Branch => "branch",
            Self::TimeRange => "time_range",
            Self::MessageType => "message_type",
            Self::Tags => "tags",

            Self::Archived => "archived",
        }
    }

    /// Check if scope requires additional parameters
    #[inline]
    #[must_use]
    pub const fn requires_parameters(&self) -> bool {
        matches!(
            self,
            Self::Session | Self::Branch | Self::TimeRange | Self::MessageType | Self::Tags
        )
    }
}

/// Stats type enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatsType {
    /// Usage statistics
    Usage,
    /// Performance metrics
    Performance,
    /// History logs
    History,
    /// Token usage statistics
    Tokens,
    /// Cost statistics
    Costs,
    /// Error statistics
    Errors,
    /// Memory usage statistics
    Memory,
    /// Network statistics
    Network,
    /// Cache statistics
    Cache,
    /// Command execution statistics
    Commands,
}

impl StatsType {
    /// Get type name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::Performance => "performance",
            Self::History => "history",
            Self::Tokens => "tokens",
            Self::Costs => "costs",
            Self::Errors => "errors",
            Self::Memory => "memory",
            Self::Network => "network",
            Self::Cache => "cache",
            Self::Commands => "commands",
        }
    }

    /// Check if stats type requires time range
    #[inline]
    #[must_use]
    pub const fn supports_time_range(&self) -> bool {
        matches!(
            self,
            Self::Usage
                | Self::Performance
                | Self::History
                | Self::Tokens
                | Self::Costs
                | Self::Errors
                | Self::Commands
        )
    }
}

/// Import type enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImportType {
    /// Import chat data
    Chat,
    /// Import configuration
    Config,
    /// Import templates
    Templates,
    /// Import macros
    Macros,
    /// Import themes
    Themes,
    /// Import history
    History,
    /// Import sessions
    Sessions,
    /// Import tools
    Tools,
    /// Import all data
    All,
}

impl ImportType {
    /// Get type name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Config => "config",
            Self::Templates => "templates",
            Self::Macros => "macros",
            Self::Themes => "themes",
            Self::History => "history",
            Self::Sessions => "sessions",
            Self::Tools => "tools",
            Self::All => "all",
        }
    }

    /// Check if import type requires file path
    #[inline]
    #[must_use]
    pub const fn requires_file_path(&self) -> bool {
        true // All import types require a source file
    }

    /// Check if import type supports selective import
    #[inline]
    #[must_use]
    pub const fn supports_selective_import(&self) -> bool {
        matches!(
            self,
            Self::Templates | Self::Macros | Self::Themes | Self::Sessions | Self::Tools
        )
    }
}
