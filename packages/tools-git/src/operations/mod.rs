//! Git operations module
//!
//! Provides local Git repository operations using the gix (Gitoxide) library.

pub mod add;
pub mod branch;
pub mod checkout;
pub mod clone;
pub mod commit;
pub mod fetch;
pub mod log;
pub mod merge;
pub mod open;
pub mod push;
pub mod reset;
pub mod status;
pub mod tag;
pub mod worktree;

// Re-export operation functions
pub use add::{add, AddOpts};
pub use branch::{branch, delete_branch, list_branches, rename_branch, BranchOpts};
pub use checkout::{checkout, CheckoutOpts};
pub use clone::{clone_repo, CloneOpts};
pub use commit::{commit, CommitOpts, Signature};
pub use fetch::{fetch, FetchOpts};
pub use log::{log, LogOpts};
pub use merge::{merge, MergeOpts, MergeOutcome};
pub use open::{
    discover_repo, init_bare_repo, init_repo, is_repository, open_repo, probe_repository,
    RepositoryInfo,
};
pub use push::{
    check_remote_branch_exists, check_remote_tag_exists, delete_remote_branch, 
    delete_remote_tag, push, push_current_branch, push_tags, PushOpts, PushResult
};
pub use reset::{reset, reset_hard, reset_mixed, reset_soft, ResetMode, ResetOpts};
pub use status::{current_branch, head_commit, is_clean, is_detached, list_remotes, remote_exists, BranchInfo, RemoteInfo};
pub use tag::{create_tag, delete_tag, list_tags, tag_exists, TagInfo, TagOpts};
pub use worktree::{
    list_worktrees, worktree_add, worktree_lock, worktree_prune, worktree_remove,
    worktree_unlock, WorktreeAddOpts, WorktreeInfo, WorktreeLockOpts, WorktreeRemoveOpts,
};
