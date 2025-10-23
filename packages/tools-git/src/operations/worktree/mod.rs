//! Git worktree operations with comprehensive options.
//!
//! This module provides worktree management functionality including creation,
//! listing, locking, and removal of worktrees using the gix (Gitoxide) library.

mod types;
mod helpers;
mod list;
mod add;
mod lock;
mod remove;
mod prune;

// Re-export public types
pub use types::{WorktreeInfo, WorktreeAddOpts, WorktreeLockOpts, WorktreeRemoveOpts};

// Re-export public functions
pub use list::list_worktrees;
pub use add::worktree_add;
pub use lock::{worktree_lock, worktree_unlock};
pub use remove::worktree_remove;
pub use prune::worktree_prune;
