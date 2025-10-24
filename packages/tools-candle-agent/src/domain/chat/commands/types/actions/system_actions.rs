//! System-level operation action types
//!
//! Provides action enums for tool management and debugging.

use serde::{Deserialize, Serialize};

/// Tool-related action enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolAction {
    /// List available tools
    List,
    /// Install a new tool
    Install,
    /// Remove a tool
    Remove,
    /// Configure tool settings
    Configure,
    /// Update a tool
    Update,
    /// Execute a tool
    Execute,
    /// Show tool details
    Show,
    /// Enable tool
    Enable,
    /// Disable tool
    Disable,
    /// Reset tool configuration
    Reset,
    /// Test tool functionality
    Test,
    /// Get tool help
    Help,
}

impl ToolAction {
    /// Get action name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Install => "install",
            Self::Remove => "remove",
            Self::Configure => "configure",
            Self::Update => "update",
            Self::Execute => "execute",
            Self::Show => "show",
            Self::Enable => "enable",
            Self::Disable => "disable",
            Self::Reset => "reset",
            Self::Test => "test",
            Self::Help => "help",
        }
    }

    /// Check if action requires tool name parameter
    #[inline]
    #[must_use]
    pub const fn requires_name(&self) -> bool {
        matches!(
            self,
            Self::Install
                | Self::Remove
                | Self::Configure
                | Self::Update
                | Self::Execute
                | Self::Show
                | Self::Enable
                | Self::Disable
                | Self::Reset
                | Self::Test
                | Self::Help
        )
    }

    /// Check if action modifies tools
    #[inline]
    #[must_use]
    pub const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Install
                | Self::Remove
                | Self::Configure
                | Self::Update
                | Self::Enable
                | Self::Disable
                | Self::Reset
        )
    }
}

/// Debug action enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DebugAction {
    /// Show debug information
    Info,
    /// Display system logs
    Logs,
    /// Show performance metrics
    Performance,
    /// Display memory usage
    Memory,
    /// Show network statistics
    Network,
    /// Display cache statistics
    Cache,
    /// Show thread information
    Threads,
    /// Display environment variables
    Environment,
    /// Show configuration details
    Config,
    /// Display trace information
    Trace,
    /// Show profiling data
    Profile,
    /// Run system diagnostics
    Diagnostics,
}

impl DebugAction {
    /// Get action name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Logs => "logs",
            Self::Performance => "performance",
            Self::Memory => "memory",
            Self::Network => "network",
            Self::Cache => "cache",
            Self::Threads => "threads",
            Self::Environment => "environment",
            Self::Config => "config",
            Self::Trace => "trace",
            Self::Profile => "profile",
            Self::Diagnostics => "diagnostics",
        }
    }

    /// Check if action supports filtering parameters
    #[inline]
    #[must_use]
    pub const fn supports_filtering(&self) -> bool {
        matches!(
            self,
            Self::Logs
                | Self::Performance
                | Self::Memory
                | Self::Network
                | Self::Trace
                | Self::Profile
        )
    }
}
