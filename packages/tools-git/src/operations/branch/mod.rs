//! Git branch operation with comprehensive options.
//!
//! This module provides the `BranchOpts` builder pattern and branch operation
//! implementation for the `GitGix` service.

mod create;
mod delete;
mod helpers;
mod list;
mod rename;
mod types;

// Re-export public types
pub use types::BranchOpts;

// Re-export public functions
pub use create::branch;
pub use delete::delete_branch;
pub use list::list_branches;
pub use rename::rename_branch;
