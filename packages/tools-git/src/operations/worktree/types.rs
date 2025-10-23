//! Worktree types and builder patterns.
//!
//! This module contains all the data structures used for worktree operations
//! including information structures and option builders.

use std::path::PathBuf;

use gix::hash::ObjectId;

/// Comprehensive metadata about a worktree.
///
/// Contains all relevant information about a git worktree including its location,
/// status, and current HEAD information.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Worktree checkout path
    pub path: PathBuf,
    /// Git directory (.git/worktrees/<name> or .git for main)
    pub git_dir: PathBuf,
    /// True if this is the main worktree
    pub is_main: bool,
    /// True if the repository is bare
    pub is_bare: bool,
    /// Current HEAD commit ID
    pub head_commit: Option<ObjectId>,
    /// Current branch name (None if detached)
    pub head_branch: Option<String>,
    /// True if the worktree is locked
    pub is_locked: bool,
    /// Lock reason if the worktree is locked
    pub lock_reason: Option<String>,
    /// True if HEAD is detached
    pub is_detached: bool,
}

/// Options for `worktree add` operation with builder pattern.
///
/// # Examples
///
/// ```ignore
/// let opts = WorktreeAddOpts::new("/path/to/worktree")
///     .committish("feature-branch")
///     .force(true);
/// ```
#[derive(Debug, Clone)]
pub struct WorktreeAddOpts {
    /// Path where the worktree will be created
    pub path: PathBuf,
    /// Branch or commit to checkout (defaults to HEAD)
    pub committish: Option<String>,
    /// Force creation even if path exists
    pub force: bool,
    /// Create with detached HEAD
    pub detach: bool,
}

impl WorktreeAddOpts {
    /// Create new worktree add options with the given path.
    #[inline]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            committish: None,
            force: false,
            detach: false,
        }
    }

    /// Set the branch or commit to checkout in the new worktree.
    #[inline]
    pub fn committish(mut self, committish: impl Into<String>) -> Self {
        self.committish = Some(committish.into());
        self
    }

    /// Enable force mode (overwrite existing path).
    #[inline]
    #[must_use]
    pub fn force(mut self, enabled: bool) -> Self {
        self.force = enabled;
        self
    }

    /// Create worktree with detached HEAD.
    #[inline]
    #[must_use]
    pub fn detach(mut self, enabled: bool) -> Self {
        self.detach = enabled;
        self
    }
}

/// Options for `worktree lock` operation with builder pattern.
///
/// # Examples
///
/// ```ignore
/// let opts = WorktreeLockOpts::new("/path/to/worktree")
///     .reason("Worktree on external drive");
/// ```
#[derive(Debug, Clone)]
pub struct WorktreeLockOpts {
    /// Path to the worktree to lock
    pub path: PathBuf,
    /// Optional reason for locking
    pub reason: Option<String>,
}

impl WorktreeLockOpts {
    /// Create new worktree lock options with the given path.
    #[inline]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            reason: None,
        }
    }

    /// Set the reason for locking the worktree.
    #[inline]
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Options for `worktree remove` operation with builder pattern.
///
/// # Examples
///
/// ```ignore
/// let opts = WorktreeRemoveOpts::new("/path/to/worktree")
///     .force(true);
/// ```
#[derive(Debug, Clone)]
pub struct WorktreeRemoveOpts {
    /// Path to the worktree to remove
    pub path: PathBuf,
    /// Force removal even if locked
    pub force: bool,
}

impl WorktreeRemoveOpts {
    /// Create new worktree remove options with the given path.
    #[inline]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            force: false,
        }
    }

    /// Enable force mode (remove even if locked).
    #[inline]
    #[must_use]
    pub fn force(mut self, enabled: bool) -> Self {
        self.force = enabled;
        self
    }
}
