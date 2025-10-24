//! `kodegen_git` - A Git facade over the gix (Gitoxide) library
//!
//! This library provides an async-first Git service layer with comprehensive
//! operation support using the modern gix crate. Each Git operation is
//! implemented in its own module with builder patterns for ergonomic usage.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use gix::hash::ObjectId;
use thiserror::Error;

// Module declarations
pub mod operations;
pub mod runtime;
pub mod tools;

// Re-export runtime types
pub use runtime::{AsyncStream, AsyncTask, EmitterBuilder};

// Re-export Git operations
pub use operations::{
    AddOpts, BranchInfo, BranchOpts, CheckoutOpts, CloneOpts, CommitOpts, FetchOpts, LogOpts,
    MergeOpts, MergeOutcome, PushOpts, PushResult, RemoteInfo, RepositoryInfo, ResetMode,
    ResetOpts, Signature, TagInfo, TagOpts, WorktreeAddOpts, WorktreeInfo, WorktreeLockOpts,
    WorktreeRemoveOpts, add, branch, check_remote_branch_exists, check_remote_tag_exists, checkout,
    clone_repo, commit, create_tag, current_branch, delete_branch, delete_remote_branch,
    delete_remote_tag, delete_tag, discover_repo, fetch, head_commit, init_bare_repo, init_repo,
    is_clean, is_detached, is_repository, list_branches, list_remotes, list_tags, list_worktrees,
    log, merge, open_repo, probe_repository, push, push_current_branch, push_tags, remote_exists,
    rename_branch, reset, reset_hard, reset_mixed, reset_soft, tag_exists, worktree_add,
    worktree_lock, worktree_prune, worktree_remove, worktree_unlock,
};

// Re-export MCP tools
pub use tools::{
    GitAddTool, GitBranchCreateTool,
    GitBranchDeleteTool, GitBranchListTool,
    GitBranchRenameTool, GitCheckoutTool, GitCloneTool,
    GitCommitTool, GitDiscoverTool, GitFetchTool,
    GitInitTool, GitLogTool, GitMergeTool,
    GitOpenTool, GitWorktreeAddTool, GitWorktreeListTool,
    GitWorktreeLockTool, GitWorktreePruneTool,
    GitWorktreeRemoveTool, GitWorktreeUnlockTool,
};

/// Error types for `GitGix` operations
#[derive(Debug, Error)]
pub enum GitError {
    #[error("Gix error: {0}")]
    Gix(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Repository not found at path: {0}")]
    RepoNotFound(PathBuf),

    #[error("Remote `{0}` not found")]
    RemoteNotFound(String),

    #[error("Branch `{0}` not found")]
    BranchNotFound(String),

    #[error("Reference `{0}` not found")]
    ReferenceNotFound(String),

    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(&'static str),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Channel closed prematurely")]
    ChannelClosed,

    #[error("Operation aborted by user")]
    Aborted,

    #[error("Worktree already exists at path: {0}")]
    WorktreeAlreadyExists(PathBuf),

    #[error("Worktree not found: {0}")]
    WorktreeNotFound(String),

    #[error("Worktree is locked: {0}")]
    WorktreeLocked(String),

    #[error("Branch '{0}' is already checked out in another worktree")]
    BranchInUse(String),

    #[error("Cannot modify main worktree")]
    CannotModifyMainWorktree,

    #[error("Invalid worktree name: {0}")]
    InvalidWorktreeName(String),
}

impl From<gix::open::Error> for GitError {
    fn from(e: gix::open::Error) -> Self {
        GitError::Gix(Box::new(e))
    }
}

impl From<gix::discover::Error> for GitError {
    fn from(e: gix::discover::Error) -> Self {
        GitError::Gix(Box::new(e))
    }
}

impl From<gix::init::Error> for GitError {
    fn from(e: gix::init::Error) -> Self {
        GitError::Gix(Box::new(e))
    }
}

impl From<gix::clone::Error> for GitError {
    fn from(e: gix::clone::Error) -> Self {
        GitError::Gix(Box::new(e))
    }
}

/// Convenience result alias.
pub type GitResult<T> = Result<T, GitError>;

/// Strong-typed repository wrapper with cheap cloning.
///
/// Wraps a single `gix::Repository` instance. Cloning this handle creates
/// a new repository instance that shares underlying data structures (refs, objects)
/// but has independent thread-local buffers, making it Send-safe.
///
/// # Performance
///
/// Cloning this handle is cheap - it shares the underlying ODB and refs database
/// but clears thread-local buffers. This is the design of `gix::Repository` itself.
///
/// # Thread Safety
///
/// The wrapped `gix::Repository` is `Send` but not `Sync`.
/// Each clone can be safely moved to a different thread.
#[derive(Debug, Clone)]
pub struct RepoHandle {
    inner: gix::Repository,
}

impl RepoHandle {
    /// Create from an existing `gix::Repository`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let repo = gix::open("/path/to/repo")?;
    /// let handle = RepoHandle::new(repo);
    /// let handle2 = handle.clone(); // Cheap clone with shared data!
    /// ```
    #[inline]
    pub fn new(inner: gix::Repository) -> Self {
        Self { inner }
    }

    /// Access the underlying `gix::Repository` with zero cost.
    ///
    /// Returns a reference to the repository. No Result needed
    /// because the repository is already open and validated.
    ///
    /// # Example
    ///
    /// ```rust
    /// let head = handle.raw().head()?;
    /// let config = handle.raw().config_snapshot();
    /// ```
    #[inline]
    pub fn raw(&self) -> &gix::Repository {
        &self.inner
    }

    /// Clone the underlying repository for use in async tasks.
    ///
    /// Creates a cheap clone of the repository with shared refs and objects
    /// but with thread-local buffers cleared. This makes the cloned repository
    /// Send-safe for use in `spawn_blocking`.
    ///
    /// This is equivalent to cloning the `RepoHandle` itself, provided for
    /// API compatibility.
    ///
    /// # Example
    ///
    /// ```rust
    /// let repo_clone = handle.clone_inner();
    /// tokio::task::spawn_blocking(move || {
    ///     // Use repo_clone safely in blocking task
    /// });
    /// ```
    #[inline]
    pub fn clone_inner(&self) -> gix::Repository {
        self.inner.clone()
    }
}

/// A unique commit identifier.
pub type CommitId = ObjectId;

/// Lightweight commit metadata for streaming logs.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: CommitId,
    pub author: Signature,
    pub summary: String,
    pub time: DateTime<Utc>,
}

/// Backward compatibility module providing nested namespace for git operations.
///
/// This module re-exports all operations in their original nested structure
/// for compatibility with existing code that expects `kodegen_git::git::*` paths.
pub mod git {
    /// Add operation re-exports
    pub mod add {
        pub use crate::operations::add::*;
    }

    /// Branch operation re-exports
    pub mod branch {
        pub use crate::operations::branch::*;
    }

    /// Checkout operation re-exports
    pub mod checkout {
        pub use crate::operations::checkout::*;
    }

    /// Clone operation re-exports
    pub mod clone {
        pub use crate::operations::clone::*;
    }

    /// Commit operation re-exports
    pub mod commit {
        pub use crate::operations::commit::*;
    }

    /// Fetch operation re-exports
    pub mod fetch {
        pub use crate::operations::fetch::*;
    }

    /// Log operation re-exports
    pub mod log {
        pub use crate::operations::log::*;
    }

    /// Merge operation re-exports
    pub mod merge {
        pub use crate::operations::merge::*;
    }

    /// Repository open/init operation re-exports
    pub mod open {
        pub use crate::operations::open::*;
    }
}
