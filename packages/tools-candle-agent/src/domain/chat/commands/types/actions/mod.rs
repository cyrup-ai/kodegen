//! Action type definitions for command variants with zero allocation patterns
//!
//! Provides blazing-fast action enumeration dispatch with owned strings for
//! maximum performance. No Arc usage, no locking, pure enum-based dispatch.

// Module declarations
pub mod content_actions;
pub mod data_actions;
pub mod state_actions;
pub mod system_actions;
pub mod ui_actions;

// Re-export all public types to maintain existing API
pub use content_actions::{BranchAction, MacroAction, TemplateAction};
pub use data_actions::{ImportType, SearchScope, StatsType};
pub use state_actions::{HistoryAction, SessionAction};
pub use system_actions::{DebugAction, ToolAction};
pub use ui_actions::ThemeAction;
