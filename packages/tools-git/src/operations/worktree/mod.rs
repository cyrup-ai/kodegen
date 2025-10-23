//! Git worktree operations with comprehensive options.
//!
//! This module provides worktree management functionality including creation,
//! listing, locking, and removal of worktrees using the gix (Gitoxide) library.

mod add;
mod helpers;
mod list;
mod lock;
mod prune;
mod remove;
mod types;

// Re-export public types
pub use types::{WorktreeAddOpts, WorktreeInfo, WorktreeLockOpts, WorktreeRemoveOpts};

// Re-export public functions
pub use add::worktree_add;
pub use list::list_worktrees;
pub use lock::{worktree_lock, worktree_unlock};
pub use prune::worktree_prune;
pub use remove::worktree_remove;
