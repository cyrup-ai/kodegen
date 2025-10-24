//! Core command enumeration with zero allocation patterns
//!
//! Provides the main `ImmutableChatCommand` enum with owned strings allocated once
//! for maximum performance. No Arc usage, no locking, comprehensive command variants.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::actions::{
    BranchAction, DebugAction, HistoryAction, ImportType, MacroAction, SearchScope, SessionAction,
    StatsType, TemplateAction, ThemeAction, ToolAction,
};
use super::command_enums::{OutputType, SettingsCategory};

/// Immutable chat command with owned strings allocated once for blazing-fast performance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImmutableChatCommand {
    /// Show help information
    Help {
        /// Optional command to get help for (owned string allocated once)
        command: Option<String>,
        /// Show extended help
        extended: bool,
    },
    /// Clear chat history
    Clear {
        /// Confirm the action
        confirm: bool,
        /// Keep last N messages
        keep_last: Option<usize>,
    },
    /// Export conversation
    Export {
        /// Export format (json, markdown, pdf, html) - owned string allocated once
        format: String,
        /// Output file path - owned string allocated once
        output: Option<String>,
        /// Include metadata
        include_metadata: bool,
    },
    /// Modify configuration
    Config {
        /// Configuration key - owned string allocated once
        key: Option<String>,
        /// Configuration value - owned string allocated once
        value: Option<String>,
        /// Show current configuration
        show: bool,
        /// Reset to defaults
        reset: bool,
    },
    /// Template operations
    Template {
        /// Template action
        action: TemplateAction,
        /// Template name - owned string allocated once
        name: Option<String>,
        /// Template content - owned string allocated once
        content: Option<String>,
        /// Template variables - owned map allocated once
        variables: HashMap<String, String>,
    },
    /// Macro operations
    Macro {
        /// Macro action
        action: MacroAction,
        /// Macro name - owned string allocated once
        name: Option<String>,
        /// Auto-execute macro
        auto_execute: bool,
        /// Commands to execute in macro - owned strings allocated once
        commands: Vec<String>,
    },
    /// Search chat history
    Search {
        /// Search query - owned string allocated once
        query: String,
        /// Search scope
        scope: SearchScope,
        /// Maximum results
        limit: Option<usize>,
        /// Include context
        include_context: bool,
    },
    /// Branch conversation
    Branch {
        /// Branch action
        action: BranchAction,
        /// Branch name - owned string allocated once
        name: Option<String>,
        /// Source branch for merging - owned string allocated once
        source: Option<String>,
    },
    /// Session management
    Session {
        /// Session action
        action: SessionAction,
        /// Session name - owned string allocated once
        name: Option<String>,
        /// Include configuration
        include_config: bool,
    },
    /// Tool integration
    Tool {
        /// Tool action
        action: ToolAction,
        /// Tool name - owned string allocated once
        name: Option<String>,
        /// Tool arguments - owned map allocated once
        args: HashMap<String, String>,
    },
    /// Statistics and analytics
    Stats {
        /// Statistics type
        stat_type: StatsType,
        /// Time period - owned string allocated once
        period: Option<String>,
        /// Show detailed breakdown
        detailed: bool,
    },
    /// Theme and appearance
    Theme {
        /// Theme action
        action: ThemeAction,
        /// Theme name - owned string allocated once
        name: Option<String>,
        /// Theme properties - owned map allocated once
        properties: HashMap<String, String>,
    },
    /// Debugging and diagnostics
    Debug {
        /// Debug action
        action: DebugAction,
        /// Debug level - owned string allocated once
        level: Option<String>,
        /// Show system information
        system_info: bool,
    },
    /// Chat history operations
    History {
        /// History action
        action: HistoryAction,
        /// Number of messages to show
        limit: Option<usize>,
        /// Filter criteria - owned string allocated once
        filter: Option<String>,
    },
    /// Save conversation state
    Save {
        /// Save name - owned string allocated once
        name: Option<String>,
        /// Include configuration
        include_config: bool,
        /// Save location - owned string allocated once
        location: Option<String>,
    },
    /// Load conversation state
    Load {
        /// Load name - owned string allocated once
        name: String,
        /// Merge with current session
        merge: bool,
        /// Load location - owned string allocated once
        location: Option<String>,
    },
    /// Import data or configuration
    Import {
        /// Import type
        import_type: ImportType,
        /// Source file or URL - owned string allocated once
        source: String,
        /// Import options - owned map allocated once
        options: HashMap<String, String>,
    },
    /// Application settings
    Settings {
        /// Setting category
        category: SettingsCategory,
        /// Setting key - owned string allocated once
        key: Option<String>,
        /// Setting value - owned string allocated once
        value: Option<String>,
        /// Show current settings
        show: bool,
        /// Reset to defaults
        reset: bool,
    },
    /// Custom command
    Custom {
        /// Command name - owned string allocated once
        name: String,
        /// Command arguments - owned map allocated once
        args: HashMap<String, String>,
        /// Command metadata
        metadata: Option<serde_json::Value>,
    },
    /// Copy message or content
    Copy {
        /// Message ID to copy - owned string allocated once
        message_id: Option<String>,
        /// Content to copy - owned string allocated once
        content: Option<String>,
        /// Copy format
        format: OutputType,
    },
    /// Retry last command or message
    Retry {
        /// Command to retry - owned string allocated once
        command: Option<String>,
        /// Number of retry attempts
        attempts: Option<u32>,
        /// Retry with modifications
        modify: bool,
    },
    /// Undo last action
    Undo {
        /// Number of actions to undo
        count: Option<usize>,
        /// Confirm undo action
        confirm: bool,
        /// Show undo preview
        preview: bool,
    },
    /// Direct chat message
    Chat {
        /// Message content - owned string allocated once
        message: String,
        /// Message context - owned string allocated once
        context: Option<String>,
        /// Message priority
        priority: u8,
    },
}

impl Default for ImmutableChatCommand {
    fn default() -> Self {
        ImmutableChatCommand::Help {
            command: None,
            extended: false,
        }
    }
}
