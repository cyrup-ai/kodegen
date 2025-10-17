//! Git branch operation with comprehensive options.
//!
//! This module provides the `BranchOpts` builder pattern and branch operation
//! implementation for the GitGix service.

mod types;
mod helpers;
mod create;
mod list;
mod delete;
mod rename;

// Re-export public types
pub use types::BranchOpts;

// Re-export public functions
pub use create::branch;
pub use list::list_branches;
pub use delete::delete_branch;
pub use rename::rename_branch;
