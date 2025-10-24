//! Content and workflow management action types
//!
//! Provides action enums for templates, macros, and conversation branches.

use serde::{Deserialize, Serialize};

/// Template-related action enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TemplateAction {
    /// List available templates
    List,
    /// Create a new template
    Create,
    /// Delete a template
    Delete,
    /// Edit an existing template
    Edit,
    /// Use a template
    Use,
    /// Show template details
    Show,
    /// Validate template syntax
    Validate,
    /// Export template to file
    Export,
    /// Import template from file
    Import,
    /// Copy template to new name
    Copy,
    /// Rename existing template
    Rename,
}

impl TemplateAction {
    /// Get action name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Create => "create",
            Self::Delete => "delete",
            Self::Edit => "edit",
            Self::Use => "use",
            Self::Show => "show",
            Self::Validate => "validate",
            Self::Export => "export",
            Self::Import => "import",
            Self::Copy => "copy",
            Self::Rename => "rename",
        }
    }

    /// Check if action requires template name parameter
    #[inline]
    #[must_use]
    pub const fn requires_name(&self) -> bool {
        matches!(
            self,
            Self::Create
                | Self::Delete
                | Self::Edit
                | Self::Use
                | Self::Show
                | Self::Validate
                | Self::Export
                | Self::Copy
                | Self::Rename
        )
    }

    /// Check if action modifies templates
    #[inline]
    #[must_use]
    pub const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Create | Self::Delete | Self::Edit | Self::Import | Self::Copy | Self::Rename
        )
    }
}

/// Macro-related action enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MacroAction {
    /// List available macros
    List,
    /// Create a new macro
    Create,
    /// Delete a macro
    Delete,
    /// Edit an existing macro
    Edit,
    /// Execute a macro
    Execute,
    /// Show macro details
    Show,
    /// Record new macro from commands
    Record,
    /// Stop macro recording
    Stop,
    /// Pause macro execution
    Pause,
    /// Resume paused macro
    Resume,
    /// Export macro to file
    Export,
    /// Import macro from file
    Import,
}

impl MacroAction {
    /// Get action name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Create => "create",
            Self::Delete => "delete",
            Self::Edit => "edit",
            Self::Execute => "execute",
            Self::Show => "show",
            Self::Record => "record",
            Self::Stop => "stop",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Export => "export",
            Self::Import => "import",
        }
    }

    /// Check if action requires macro name parameter
    #[inline]
    #[must_use]
    pub const fn requires_name(&self) -> bool {
        matches!(
            self,
            Self::Create
                | Self::Delete
                | Self::Edit
                | Self::Execute
                | Self::Show
                | Self::Export
                | Self::Record
        )
    }

    /// Check if action modifies macros
    #[inline]
    #[must_use]
    pub const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Create | Self::Delete | Self::Edit | Self::Record | Self::Stop | Self::Import
        )
    }

    /// Check if action affects running macros
    #[inline]
    #[must_use]
    pub const fn affects_execution(&self) -> bool {
        matches!(
            self,
            Self::Execute | Self::Pause | Self::Resume | Self::Stop
        )
    }
}

/// Branch-related action enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BranchAction {
    /// List available branches
    List,
    /// Create a new branch
    Create,
    /// Switch to a different branch
    Switch,
    /// Merge branches
    Merge,
    /// Delete a branch
    Delete,
    /// Show branch details
    Show,
    /// Compare branches
    Compare,
    /// Rename branch
    Rename,
    /// Archive branch
    Archive,
    /// Restore archived branch
    Restore,
}

impl BranchAction {
    /// Get action name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Create => "create",
            Self::Switch => "switch",
            Self::Merge => "merge",
            Self::Delete => "delete",
            Self::Show => "show",
            Self::Compare => "compare",
            Self::Rename => "rename",
            Self::Archive => "archive",
            Self::Restore => "restore",
        }
    }

    /// Check if action requires branch name parameter
    #[inline]
    #[must_use]
    pub const fn requires_name(&self) -> bool {
        matches!(
            self,
            Self::Create
                | Self::Switch
                | Self::Delete
                | Self::Show
                | Self::Rename
                | Self::Archive
                | Self::Restore
        )
    }

    /// Check if action modifies branches
    #[inline]
    #[must_use]
    pub const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Create
                | Self::Switch
                | Self::Merge
                | Self::Delete
                | Self::Rename
                | Self::Archive
                | Self::Restore
        )
    }
}
