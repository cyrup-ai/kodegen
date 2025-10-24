//! Executor struct definitions for all domain commands
//!
//! Each executor is a lightweight struct containing only command metadata.
//! Actual execution logic is implemented via the `DomainCommandExecutor` trait.

use super::CommandInfo;

/// Help command executor - displays command documentation
#[derive(Debug, Clone)]
pub struct DomainHelpExecutor {
    pub(super) info: CommandInfo,
}

/// Clear command executor - clears conversation history
#[derive(Debug, Clone)]
pub struct DomainClearExecutor {
    pub(super) info: CommandInfo,
}

/// Export command executor - exports domain data
#[derive(Debug, Clone)]
pub struct DomainExportExecutor {
    pub(super) info: CommandInfo,
}

/// Config command executor - manages configuration
#[derive(Debug, Clone)]
pub struct DomainConfigExecutor {
    pub(super) info: CommandInfo,
}

/// Template command executor - processes templates
#[derive(Debug, Clone)]
pub struct DomainTemplateExecutor {
    pub(super) info: CommandInfo,
}

/// Macro command executor - executes macros
#[derive(Debug, Clone)]
pub struct DomainMacroExecutor {
    pub(super) info: CommandInfo,
}

/// Search command executor - performs domain searches
#[derive(Debug, Clone)]
pub struct DomainSearchExecutor {
    pub(super) info: CommandInfo,
}

/// Branch command executor - handles branching operations
#[derive(Debug, Clone)]
pub struct DomainBranchExecutor {
    pub(super) info: CommandInfo,
}

/// Session command executor - manages sessions
#[derive(Debug, Clone)]
pub struct DomainSessionExecutor {
    pub(super) info: CommandInfo,
}

/// Tool command executor - executes tools
#[derive(Debug, Clone)]
pub struct DomainToolExecutor {
    pub(super) info: CommandInfo,
}

/// Stats command executor - provides statistics
#[derive(Debug, Clone)]
pub struct DomainStatsExecutor {
    pub(super) info: CommandInfo,
}

/// Theme command executor - manages themes
#[derive(Debug, Clone)]
pub struct DomainThemeExecutor {
    pub(super) info: CommandInfo,
}

/// Debug command executor - provides debugging information
#[derive(Debug, Clone)]
pub struct DomainDebugExecutor {
    pub(super) info: CommandInfo,
}

/// History command executor - manages command history
#[derive(Debug, Clone)]
pub struct DomainHistoryExecutor {
    pub(super) info: CommandInfo,
}

/// Save command executor - saves domain data
#[derive(Debug, Clone)]
pub struct DomainSaveExecutor {
    pub(super) info: CommandInfo,
}

/// Load command executor - loads domain data
#[derive(Debug, Clone)]
pub struct DomainLoadExecutor {
    pub(super) info: CommandInfo,
}

/// Import command executor - imports domain data
#[derive(Debug, Clone)]
pub struct DomainImportExecutor {
    pub(super) info: CommandInfo,
}

/// Settings command executor - manages settings
#[derive(Debug, Clone)]
pub struct DomainSettingsExecutor {
    pub(super) info: CommandInfo,
}

/// Custom command executor - handles custom commands
#[derive(Debug, Clone)]
pub struct DomainCustomExecutor {
    pub(super) info: CommandInfo,
}

/// Copy command executor - performs copy operations
#[derive(Debug, Clone)]
pub struct DomainCopyExecutor {
    pub(super) info: CommandInfo,
}

/// Retry command executor - retries failed operations
#[derive(Debug, Clone)]
pub struct DomainRetryExecutor {
    pub(super) info: CommandInfo,
}

/// Undo command executor - undoes previous operations
#[derive(Debug, Clone)]
pub struct DomainUndoExecutor {
    pub(super) info: CommandInfo,
}

/// Chat command executor - handles chat interactions
#[derive(Debug, Clone)]
pub struct DomainChatExecutor {
    pub(super) info: CommandInfo,
}
