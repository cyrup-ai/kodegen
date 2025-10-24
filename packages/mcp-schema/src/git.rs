//! Git tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// GIT INIT
// ============================================================================

/// Arguments for `git_init` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitInitArgs {
    /// Path where to initialize the repository
    pub path: String,

    /// Create a bare repository (no working directory)
    #[serde(default)]
    pub bare: bool,

    /// Name of the initial branch (informational only, gix uses default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_branch: Option<String>,
}

/// Prompt arguments for `git_init` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitInitPromptArgs {}

// ============================================================================
// GIT OPEN
// ============================================================================

/// Arguments for `git_open` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitOpenArgs {
    /// Path to the existing repository
    pub path: String,
}

/// Prompt arguments for `git_open` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitOpenPromptArgs {}

// ============================================================================
// GIT CLONE
// ============================================================================

/// Arguments for `git_clone` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitCloneArgs {
    /// Git URL to clone from (https:// or git://)
    pub url: String,

    /// Local path to clone into
    pub path: String,

    /// Specific branch to checkout (defaults to repository default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Shallow clone depth (minimum: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
}

/// Prompt arguments for `git_clone` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitClonePromptArgs {}

// ============================================================================
// GIT DISCOVER
// ============================================================================

/// Arguments for `git_discover` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitDiscoverArgs {
    /// Path to search from (can be subdirectory within a repo)
    pub path: String,
}

/// Prompt arguments for `git_discover` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitDiscoverPromptArgs {}

// ============================================================================
// GIT ADD
// ============================================================================

/// Arguments for `git_add` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitAddArgs {
    /// Path to repository
    pub path: String,

    /// Specific file paths to stage
    #[serde(default)]
    pub paths: Vec<String>,

    /// Stage all modified files
    #[serde(default)]
    pub all: bool,

    /// Force add files even if in .gitignore
    #[serde(default)]
    pub force: bool,
}

/// Prompt arguments for `git_add` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitAddPromptArgs {}

// ============================================================================
// GIT COMMIT
// ============================================================================

/// Arguments for `git_commit` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitCommitArgs {
    /// Path to repository
    pub path: String,

    /// Commit message
    pub message: String,

    /// Author name (optional, uses git config if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,

    /// Author email (optional, uses git config if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_email: Option<String>,

    /// Stage all modified tracked files before committing
    #[serde(default)]
    pub all: bool,
}

/// Prompt arguments for `git_commit` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitCommitPromptArgs {}

// ============================================================================
// GIT LOG
// ============================================================================

/// Arguments for `git_log` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitLogArgs {
    /// Path to repository
    pub path: String,

    /// Maximum number of commits to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_count: Option<usize>,

    /// Number of commits to skip
    #[serde(default)]
    pub skip: usize,

    /// Filter commits by file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_filter: Option<String>,
}

/// Prompt arguments for `git_log` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitLogPromptArgs {}

// ============================================================================
// GIT BRANCH CREATE
// ============================================================================

/// Arguments for `git_branch_create` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitBranchCreateArgs {
    /// Path to repository
    pub path: String,

    /// Name for new branch
    pub branch: String,

    /// Starting point (defaults to HEAD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_branch: Option<String>,

    /// Force creation (overwrite if exists)
    #[serde(default)]
    pub force: bool,

    /// Checkout the branch after creation
    #[serde(default)]
    pub checkout: bool,
}

/// Prompt arguments for `git_branch_create` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitBranchCreatePromptArgs {}

// ============================================================================
// GIT BRANCH DELETE
// ============================================================================

/// Arguments for `git_branch_delete` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitBranchDeleteArgs {
    /// Path to repository
    pub path: String,

    /// Name of branch to delete
    pub branch: String,

    /// Force deletion
    #[serde(default)]
    pub force: bool,
}

/// Prompt arguments for `git_branch_delete` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitBranchDeletePromptArgs {}

// ============================================================================
// GIT BRANCH LIST
// ============================================================================

/// Arguments for `git_branch_list` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitBranchListArgs {
    /// Path to repository
    pub path: String,
}

/// Prompt arguments for `git_branch_list` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitBranchListPromptArgs {}

// ============================================================================
// GIT BRANCH RENAME
// ============================================================================

/// Arguments for `git_branch_rename` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitBranchRenameArgs {
    /// Path to repository
    pub path: String,

    /// Current branch name
    pub old_name: String,

    /// New branch name
    pub new_name: String,

    /// Force rename (overwrite if new name exists)
    #[serde(default)]
    pub force: bool,
}

/// Prompt arguments for `git_branch_rename` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitBranchRenamePromptArgs {}

// ============================================================================
// GIT CHECKOUT
// ============================================================================

/// Arguments for `git_checkout` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitCheckoutArgs {
    /// Path to repository
    pub path: String,

    /// Target reference (branch, tag, or commit)
    pub target: String,

    /// Specific file paths to restore from the target reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,

    /// Create new branch before checking out
    #[serde(default)]
    pub create: bool,

    /// Force checkout (discard local changes)
    #[serde(default)]
    pub force: bool,
}

/// Prompt arguments for `git_checkout` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitCheckoutPromptArgs {}

// ============================================================================
// GIT FETCH
// ============================================================================

fn default_remote() -> String {
    "origin".to_string()
}

/// Arguments for `git_fetch` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitFetchArgs {
    /// Path to repository
    pub path: String,

    /// Remote name (defaults to "origin")
    #[serde(default = "default_remote")]
    pub remote: String,

    /// Refspecs to fetch (e.g., ["refs/heads/main:refs/remotes/origin/main"]).
    /// If empty, uses repository's configured refspecs for the remote.
    #[serde(default)]
    pub refspecs: Vec<String>,

    /// Prune remote-tracking branches that no longer exist on remote (default: false)
    #[serde(default)]
    pub prune: bool,
}

/// Prompt arguments for `git_fetch` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitFetchPromptArgs {}

// ============================================================================
// GIT MERGE
// ============================================================================

fn default_true() -> bool {
    true
}

/// Arguments for `git_merge` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitMergeArgs {
    /// Path to repository
    pub path: String,

    /// Branch or commit to merge into current branch
    pub branch: String,

    /// Allow fast-forward merges when possible (default: true).
    /// When false, always creates a merge commit even if fast-forward is possible.
    #[serde(default = "default_true")]
    pub fast_forward: bool,

    /// Automatically create merge commit (default: true).
    /// When false, performs merge but leaves changes staged for manual commit.
    #[serde(default = "default_true")]
    pub auto_commit: bool,
}

/// Prompt arguments for `git_merge` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitMergePromptArgs {}

// ============================================================================
// GIT WORKTREE ADD
// ============================================================================

/// Arguments for `git_worktree_add` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitWorktreeAddArgs {
    /// Path to repository
    pub path: String,

    /// Path where the new worktree will be created
    pub worktree_path: String,

    /// Branch or commit to checkout in the worktree (optional, defaults to HEAD).
    /// Can be a branch name, tag, or commit SHA.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Force creation even if worktree path already exists (default: false)
    #[serde(default)]
    pub force: bool,
}

/// Prompt arguments for `git_worktree_add` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitWorktreeAddPromptArgs {}

// ============================================================================
// GIT WORKTREE LIST
// ============================================================================

/// Arguments for `git_worktree_list` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitWorktreeListArgs {
    /// Path to repository
    pub path: String,
}

/// Prompt arguments for `git_worktree_list` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitWorktreeListPromptArgs {}

// ============================================================================
// GIT WORKTREE LOCK
// ============================================================================

/// Arguments for `git_worktree_lock` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitWorktreeLockArgs {
    /// Path to repository
    pub path: String,

    /// Path to the worktree to lock (prevents deletion)
    pub worktree_path: String,

    /// Optional reason for locking (e.g., "On removable drive").
    /// Stored in the lock file for documentation purposes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Prompt arguments for `git_worktree_lock` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitWorktreeLockPromptArgs {}

// ============================================================================
// GIT WORKTREE UNLOCK
// ============================================================================

/// Arguments for `git_worktree_unlock` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitWorktreeUnlockArgs {
    /// Path to repository
    pub path: String,

    /// Path to worktree to unlock
    pub worktree_path: String,
}

/// Prompt arguments for `git_worktree_unlock` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitWorktreeUnlockPromptArgs {}

// ============================================================================
// GIT WORKTREE PRUNE
// ============================================================================

/// Arguments for `git_worktree_prune` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitWorktreePruneArgs {
    /// Path to repository
    pub path: String,
}

/// Prompt arguments for `git_worktree_prune` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitWorktreePrunePromptArgs {}

// ============================================================================
// GIT WORKTREE REMOVE
// ============================================================================

/// Arguments for `git_worktree_remove` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitWorktreeRemoveArgs {
    /// Path to repository
    pub path: String,

    /// Path to the worktree to remove (both working directory and admin files)
    pub worktree_path: String,

    /// Force removal even if worktree is locked (default: false)
    #[serde(default)]
    pub force: bool,
}

/// Prompt arguments for `git_worktree_remove` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitWorktreeRemovePromptArgs {}
