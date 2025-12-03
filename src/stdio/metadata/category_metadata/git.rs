//! Git version control tools: repository operations, branching, commits, worktrees

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn git_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: GIT_ADD,
            category: "git",
            description: "Stage file changes for commit in a Git repository. Specify paths to stage specific files.' } fn read_only() -> bool { false // Modifies index } fn ...",
            schema: build_schema::<git::GitAddArgs>(),
        },
        ToolMetadata {
            name: GIT_BRANCH_CREATE,
            category: "git",
            description: "Create a new branch in a Git repository. Optionally specify a starting point and checkout the branch after creation.' } fn read_only() -> bool { fa...",
            schema: build_schema::<git::GitBranchCreateArgs>(),
        },
        ToolMetadata {
            name: GIT_BRANCH_DELETE,
            category: "git",
            description: "Delete a branch from a Git repository. Cannot delete the currently checked-out branch.' } fn read_only() -> bool { false // Modifies repository } f...",
            schema: build_schema::<git::GitBranchDeleteArgs>(),
        },
        ToolMetadata {
            name: GIT_BRANCH_LIST,
            category: "git",
            description: "List all local branches in a Git repository.' } fn read_only() -> bool { true // Only reads, doesn't modify } fn destructive() -> bool { false } fn...",
            schema: build_schema::<git::GitBranchListArgs>(),
        },
        ToolMetadata {
            name: GIT_BRANCH_RENAME,
            category: "git",
            description: "Rename a branch in a Git repository. Automatically updates HEAD if renaming the current branch.' } fn read_only() -> bool { false // Modifies repos...",
            schema: build_schema::<git::GitBranchRenameArgs>(),
        },
        ToolMetadata {
            name: GIT_CHECKOUT,
            category: "git",
            description: "Checkout a Git reference (branch, tag, or commit) or restore specific files. Without paths: switches branches/commits. With paths: restores files f...",
            schema: build_schema::<git::GitCheckoutArgs>(),
        },
        ToolMetadata {
            name: GIT_CLONE,
            category: "git",
            description: "Clone a remote Git repository to a local path. Supports shallow cloning (limited history) and branch-specific cloning. The destination path must no...",
            schema: build_schema::<git::GitCloneArgs>(),
        },
        ToolMetadata {
            name: GIT_COMMIT,
            category: "git",
            description: "Create a new commit in a Git repository. Optionally specify author information and stage all modified files.' } fn read_only() -> bool { false // C...",
            schema: build_schema::<git::GitCommitArgs>(),
        },
        ToolMetadata {
            name: GIT_DIFF,
            category: "git",
            description: "Show differences between Git revisions. Compare two commits, branches, or working directory against HEAD. Displays file changes with statistics.' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bool { true }",
            schema: build_schema::<git::GitDiffArgs>(),
        },
        ToolMetadata {
            name: GIT_DISCOVER,
            category: "git",
            description: "Discover a Git repository by searching upward from the given path. This will traverse parent directories until it finds a .git directory or reaches...",
            schema: build_schema::<git::GitDiscoverArgs>(),
        },
        ToolMetadata {
            name: GIT_FETCH,
            category: "git",
            description: "Fetch updates from a remote repository. Downloads objects and refs from another repository.' } fn read_only() -> bool { false // Fetches refs } fn ...",
            schema: build_schema::<git::GitFetchArgs>(),
        },
        ToolMetadata {
            name: GIT_INIT,
            category: "git",
            description: "Initialize a new Git repository at the specified path. Supports both normal repositories (with working directory) and bare repositories (without wo...",
            schema: build_schema::<git::GitInitArgs>(),
        },
        ToolMetadata {
            name: GIT_LOG,
            category: "git",
            description: "List commit history from a Git repository. Optionally filter by file path and limit the number of results.' } fn read_only() -> bool { true // Only...",
            schema: build_schema::<git::GitLogArgs>(),
        },
        ToolMetadata {
            name: GIT_MERGE,
            category: "git",
            description: "Merge a branch or commit into the current branch. Joins two or more development histories together.' } fn read_only() -> bool { false // Modifies H...",
            schema: build_schema::<git::GitMergeArgs>(),
        },
        ToolMetadata {
            name: GIT_OPEN,
            category: "git",
            description: "Open an existing Git repository at the specified path. The repository must already exist at the given location.' } fn read_only() -> bool { true //...",
            schema: build_schema::<git::GitOpenArgs>(),
        },
        ToolMetadata {
            name: GIT_WORKTREE_ADD,
            category: "git",
            description: "Create a new worktree linked to the repository. Allows working on multiple branches simultaneously.' } fn read_only() -> bool { false // Creates wo...",
            schema: build_schema::<git::GitWorktreeAddArgs>(),
        },
        ToolMetadata {
            name: GIT_WORKTREE_LIST,
            category: "git",
            description: "List all worktrees in the repository with detailed status. Returns main worktree and all linked worktrees with their paths, branches, lock status, ...",
            schema: build_schema::<git::GitWorktreeListArgs>(),
        },
        ToolMetadata {
            name: GIT_WORKTREE_LOCK,
            category: "git",
            description: "Lock a worktree to prevent deletion. Useful for worktrees on removable media or network drives.' } fn read_only() -> bool { false // Writes lock fi...",
            schema: build_schema::<git::GitWorktreeLockArgs>(),
        },
        ToolMetadata {
            name: GIT_WORKTREE_PRUNE,
            category: "git",
            description: "Remove stale worktree administrative files. Cleans up .git/worktrees/ entries for worktrees whose directories have been manually deleted. Returns l...",
            schema: build_schema::<git::GitWorktreePruneArgs>(),
        },
        ToolMetadata {
            name: GIT_WORKTREE_REMOVE,
            category: "git",
            description: "Remove a worktree and its associated administrative files. Cannot remove locked worktrees without force flag.' } fn read_only() -> bool { false // ...",
            schema: build_schema::<git::GitWorktreeRemoveArgs>(),
        },
        ToolMetadata {
            name: GIT_WORKTREE_UNLOCK,
            category: "git",
            description: "Unlock a locked worktree. Removes the lock that prevents worktree deletion.' } fn read_only() -> bool { false // Removes lock file } fn destructive...",
            schema: build_schema::<git::GitWorktreeUnlockArgs>(),
        },
        ToolMetadata {
            name: GIT_HISTORY,
            category: "git",
            description: "Get detailed commit history with file changes. Shows commits with their messages, authors, timestamps, and affected files.",
            schema: build_schema::<git::GitHistoryArgs>(),
        },
        ToolMetadata {
            name: GIT_PULL,
            category: "git",
            description: "Pull changes from a remote repository. Fetches and merges changes from the remote branch into the current branch.",
            schema: build_schema::<git::GitPullArgs>(),
        },
        ToolMetadata {
            name: GIT_PUSH,
            category: "git",
            description: "Push commits to a remote repository. Uploads local commits to the remote branch.",
            schema: build_schema::<git::GitPushArgs>(),
        },
        ToolMetadata {
            name: GIT_REMOTE_ADD,
            category: "git",
            description: "Add a new remote repository. Creates a new remote reference for fetching and pushing.",
            schema: build_schema::<git::GitRemoteAddArgs>(),
        },
        ToolMetadata {
            name: GIT_REMOTE_LIST,
            category: "git",
            description: "List all remote repositories. Shows all configured remotes with their URLs.",
            schema: build_schema::<git::GitRemoteListArgs>(),
        },
        ToolMetadata {
            name: GIT_REMOTE_REMOVE,
            category: "git",
            description: "Remove a remote repository. Deletes the remote reference from the repository configuration.",
            schema: build_schema::<git::GitRemoteRemoveArgs>(),
        },
        ToolMetadata {
            name: GIT_RESET,
            category: "git",
            description: "Reset current HEAD to specified state. Supports soft, mixed, and hard reset modes.",
            schema: build_schema::<git::GitResetArgs>(),
        },
        ToolMetadata {
            name: GIT_STATUS,
            category: "git",
            description: "Show the working tree status. Displays staged, unstaged, and untracked files.",
            schema: build_schema::<git::GitStatusArgs>(),
        },
        ToolMetadata {
            name: GIT_STASH,
            category: "git",
            description: "Stash changes in a dirty working directory. Saves local modifications and reverts to a clean working tree.",
            schema: build_schema::<git::GitStashArgs>(),
        },
        ToolMetadata {
            name: GIT_TAG,
            category: "git",
            description: "Create, list, or delete tags. Manages repository tags for marking specific commits.",
            schema: build_schema::<git::GitTagArgs>(),
        },
    ]
}
