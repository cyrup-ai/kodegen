//! MCP Tools for Git operations
//!
//! This module provides Model Context Protocol (MCP) tool wrappers around
//! the core Git operations for use in AI agent systems.

// Repository Operations
pub mod init;
pub mod open;
pub mod clone;
pub mod discover;

// Branch Operations
pub mod branch_create;
pub mod branch_delete;
pub mod branch_list;
pub mod branch_rename;

// Commit & Staging Operations
pub mod commit;
pub mod log;
pub mod add;
pub mod checkout;

// Remote Operations
pub mod fetch;
pub mod merge;

// Worktree Operations
pub mod worktree_add;
pub mod worktree_remove;
pub mod worktree_list;
pub mod worktree_lock;
pub mod worktree_unlock;
pub mod worktree_prune;

// Re-export tools and their argument types
pub use init::{GitInitTool, GitInitArgs};
pub use open::{GitOpenTool, GitOpenArgs};
pub use clone::{GitCloneTool, GitCloneArgs};
pub use discover::{GitDiscoverTool, GitDiscoverArgs};

pub use branch_create::{GitBranchCreateTool, GitBranchCreateArgs};
pub use branch_delete::{GitBranchDeleteTool, GitBranchDeleteArgs};
pub use branch_list::{GitBranchListTool, GitBranchListArgs};
pub use branch_rename::{GitBranchRenameTool, GitBranchRenameArgs};

pub use commit::{GitCommitTool, GitCommitArgs};
pub use log::{GitLogTool, GitLogArgs};
pub use add::{GitAddTool, GitAddArgs};
pub use checkout::{GitCheckoutTool, GitCheckoutArgs};

pub use fetch::{GitFetchTool, GitFetchArgs};
pub use merge::{GitMergeTool, GitMergeArgs};

pub use worktree_add::{GitWorktreeAddTool, GitWorktreeAddArgs};
pub use worktree_remove::{GitWorktreeRemoveTool, GitWorktreeRemoveArgs};
pub use worktree_list::{GitWorktreeListTool, GitWorktreeListArgs};
pub use worktree_lock::{GitWorktreeLockTool, GitWorktreeLockArgs};
pub use worktree_unlock::{GitWorktreeUnlockTool, GitWorktreeUnlockArgs};
pub use worktree_prune::{GitWorktreePruneTool, GitWorktreePruneArgs};
