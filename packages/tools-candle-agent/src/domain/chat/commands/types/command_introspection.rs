//! Command introspection and metadata methods
//!
//! Provides zero-allocation methods for querying command properties, requirements,
//! and resource estimations. All methods use const fn where possible.

use super::command_core::ImmutableChatCommand;

impl ImmutableChatCommand {
    /// Get command name as static string for zero allocation dispatch
    #[inline]
    #[must_use]
    pub const fn command_name(&self) -> &'static str {
        match self {
            Self::Help { .. } => "help",
            Self::Clear { .. } => "clear",
            Self::Export { .. } => "export",
            Self::Config { .. } => "config",
            Self::Template { .. } => "template",
            Self::Macro { .. } => "macro",
            Self::Search { .. } => "search",
            Self::Branch { .. } => "branch",
            Self::Session { .. } => "session",
            Self::Tool { .. } => "tool",
            Self::Stats { .. } => "stats",
            Self::Theme { .. } => "theme",
            Self::Debug { .. } => "debug",
            Self::History { .. } => "history",
            Self::Save { .. } => "save",
            Self::Load { .. } => "load",
            Self::Import { .. } => "import",
            Self::Settings { .. } => "settings",
            Self::Custom { .. } => "custom",
            Self::Copy { .. } => "copy",
            Self::Retry { .. } => "retry",
            Self::Undo { .. } => "undo",
            Self::Chat { .. } => "chat",
        }
    }

    /// Check if command requires confirmation - zero allocation check
    #[inline]
    #[must_use]
    pub const fn requires_confirmation(&self) -> bool {
        matches!(
            self,
            Self::Clear { .. }
                | Self::Load { .. }
                | Self::Import { .. }
                | Self::Undo { .. }
                | Self::Settings { reset: true, .. }
        )
    }

    /// Check if command modifies state - zero allocation check
    #[inline]
    #[must_use]
    pub const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Clear { .. }
                | Self::Config { .. }
                | Self::Template { .. }
                | Self::Macro { .. }
                | Self::Branch { .. }
                | Self::Session { .. }
                | Self::Save { .. }
                | Self::Load { .. }
                | Self::Import { .. }
                | Self::Settings { .. }
                | Self::Undo { .. }
        )
    }

    /// Check if command requires network access - zero allocation check
    #[inline]
    #[must_use]
    pub const fn requires_network(&self) -> bool {
        matches!(
            self,
            Self::Import { .. } | Self::Tool { .. } | Self::Stats { .. }
        )
    }

    /// Check if command can be executed offline - zero allocation check
    #[inline]
    #[must_use]
    pub const fn is_offline_capable(&self) -> bool {
        !self.requires_network()
    }

    /// Get command priority for execution scheduling - zero allocation
    #[inline]
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Chat { priority, .. } => *priority,
            Self::Help { .. } => 1, // High priority
            Self::Clear { .. } | Self::Undo { .. } => 2,
            Self::Config { .. } | Self::Settings { .. } => 3,
            Self::Debug { .. } => 4,
            Self::Stats { .. } => 5,
            _ => 10, // Standard priority
        }
    }

    /// Get estimated execution time in milliseconds for scheduling
    #[inline]
    #[must_use]
    pub const fn estimated_duration_ms(&self) -> u64 {
        match self {
            Self::Help { .. } => 100,
            Self::Clear { .. } => 50,
            Self::Export { .. } | Self::Chat { .. } => 5000,
            Self::Search { .. } | Self::Save { .. } => 2000,
            Self::Import { .. } => 10000,
            Self::Stats { .. } | Self::Load { .. } => 3000,
            // All commands with default duration
            Self::Debug { .. }
            | Self::History { .. }
            | Self::Config { .. }
            | Self::Template { .. }
            | Self::Macro { .. }
            | Self::Branch { .. }
            | Self::Session { .. }
            | Self::Tool { .. }
            | Self::Theme { .. }
            | Self::Settings { .. }
            | Self::Custom { .. }
            | Self::Copy { .. }
            | Self::Retry { .. }
            | Self::Undo { .. } => 1000,
        }
    }

    /// Get memory requirements in bytes for resource planning
    #[inline]
    #[must_use]
    pub const fn memory_requirement_bytes(&self) -> u64 {
        match self {
            Self::Export { .. } => 50 * 1024 * 1024, // 50MB for large exports
            Self::Import { .. } => 100 * 1024 * 1024, // 100MB for imports
            Self::Stats { .. } => 10 * 1024 * 1024,  // 10MB for statistics
            Self::History { .. } => 20 * 1024 * 1024, // 20MB for large history
            Self::Search { .. } => 5 * 1024 * 1024,  // 5MB for search operations
            Self::Help { .. }
            | Self::Clear { .. }
            | Self::Config { .. }
            | Self::Template { .. }
            | Self::Macro { .. }
            | Self::Branch { .. }
            | Self::Session { .. }
            | Self::Tool { .. }
            | Self::Theme { .. }
            | Self::Debug { .. }
            | Self::Save { .. }
            | Self::Load { .. }
            | Self::Settings { .. }
            | Self::Custom { .. }
            | Self::Copy { .. }
            | Self::Retry { .. }
            | Self::Undo { .. }
            | Self::Chat { .. } => {
                1024 * 1024 // 1MB for standard operations
            }
        }
    }

    /// Check if command can be parallelized with other commands
    #[inline]
    #[must_use]
    pub const fn is_parallelizable(&self) -> bool {
        matches!(
            self,
            Self::Help { .. }
                | Self::Stats { .. }
                | Self::Debug { .. }
                | Self::History { .. }
                | Self::Search { .. }
        )
    }
}
