//! Tests for git merge operation.

use kodegen_tools_git::CommitId;
use kodegen_tools_git::git::merge::{MergeOpts, MergeOutcome};

#[test]
fn test_merge_outcome_equality() {
    let commit_id = CommitId::null(gix::hash::Kind::Sha1);

    assert_eq!(
        MergeOutcome::FastForward(commit_id),
        MergeOutcome::FastForward(commit_id)
    );

    assert_eq!(
        MergeOutcome::MergeCommit(commit_id),
        MergeOutcome::MergeCommit(commit_id)
    );

    assert_eq!(MergeOutcome::AlreadyUpToDate, MergeOutcome::AlreadyUpToDate);
}

#[test]
fn test_merge_opts_builder() {
    let opts = MergeOpts::new("feature/branch")
        .no_ff(true)
        .squash(true)
        .commit(false);

    assert_eq!(opts.theirs, "feature/branch");
    assert!(opts.no_ff);
    assert!(opts.squash);
    assert!(!opts.commit);
}

#[test]
fn test_merge_opts_default() {
    let opts = MergeOpts::new("main");

    assert_eq!(opts.theirs, "main");
    assert!(!opts.no_ff);
    assert!(!opts.squash);
    assert!(opts.commit);
}

#[test]
fn test_merge_opts_with_commit_hash() {
    let opts = MergeOpts::new("abc123def456").no_ff(false);

    assert_eq!(opts.theirs, "abc123def456");
    assert!(!opts.no_ff);
}

#[test]
fn test_merge_opts_chaining() {
    let opts = MergeOpts::new("develop")
        .no_ff(true)
        .squash(false)
        .commit(true);

    assert_eq!(opts.theirs, "develop");
    assert!(opts.no_ff);
    assert!(!opts.squash);
    assert!(opts.commit);
}

#[test]
fn test_merge_opts_with_remote_branch() {
    let opts = MergeOpts::new("origin/feature").no_ff(true);

    assert_eq!(opts.theirs, "origin/feature");
    assert!(opts.no_ff);
}

#[test]
fn test_merge_opts_with_tag() {
    let opts = MergeOpts::new("v1.0.0").squash(true).commit(false);

    assert_eq!(opts.theirs, "v1.0.0");
    assert!(opts.squash);
    assert!(!opts.commit);
}
