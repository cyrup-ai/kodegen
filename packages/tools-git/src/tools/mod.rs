//! MCP Tools for Git operations
//!
//! This module provides Model Context Protocol (MCP) tool wrappers around
//! the core Git operations for use in AI agent systems.

// Repository Operations
pub mod clone;
pub mod discover;
pub mod init;
pub mod open;

// Branch Operations
pub mod branch_create;
pub mod branch_delete;
pub mod branch_list;
pub mod branch_rename;

// Commit & Staging Operations
pub mod add;
pub mod checkout;
pub mod commit;
pub mod log;

// Remote Operations
pub mod fetch;
pub mod merge;

// Worktree Operations
pub mod worktree_add;
pub mod worktree_list;
pub mod worktree_lock;
pub mod worktree_prune;
pub mod worktree_remove;
pub mod worktree_unlock;

// Re-export tools and their argument types
pub use clone::{GitCloneArgs, GitCloneTool};
pub use discover::{GitDiscoverArgs, GitDiscoverTool};
pub use init::{GitInitArgs, GitInitTool};
pub use open::{GitOpenArgs, GitOpenTool};

pub use branch_create::{GitBranchCreateArgs, GitBranchCreateTool};
pub use branch_delete::{GitBranchDeleteArgs, GitBranchDeleteTool};
pub use branch_list::{GitBranchListArgs, GitBranchListTool};
pub use branch_rename::{GitBranchRenameArgs, GitBranchRenameTool};

pub use add::{GitAddArgs, GitAddTool};
pub use checkout::{GitCheckoutArgs, GitCheckoutTool};
pub use commit::{GitCommitArgs, GitCommitTool};
pub use log::{GitLogArgs, GitLogTool};

pub use fetch::{GitFetchArgs, GitFetchTool};
pub use merge::{GitMergeArgs, GitMergeTool};

pub use worktree_add::{GitWorktreeAddArgs, GitWorktreeAddTool};
pub use worktree_list::{GitWorktreeListArgs, GitWorktreeListTool};
pub use worktree_lock::{GitWorktreeLockArgs, GitWorktreeLockTool};
pub use worktree_prune::{GitWorktreePruneArgs, GitWorktreePruneTool};
pub use worktree_remove::{GitWorktreeRemoveArgs, GitWorktreeRemoveTool};
pub use worktree_unlock::{GitWorktreeUnlockArgs, GitWorktreeUnlockTool};
