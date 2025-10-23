//! Tests for git fetch operation.

use kodegen_tools_git::git::fetch::FetchOpts;

#[test]
fn test_fetch_opts_builder() {
    let opts = FetchOpts::from_remote("origin")
        .add_refspec("refs/heads/*:refs/remotes/origin/*")
        .prune(true);

    assert_eq!(opts.remote, "origin");
    assert_eq!(opts.refspecs.len(), 1);
    assert_eq!(opts.refspecs[0], "refs/heads/*:refs/remotes/origin/*");
    assert!(opts.prune);
}

#[test]
fn test_fetch_opts_default() {
    let opts = FetchOpts::default();

    assert_eq!(opts.remote, "origin");
    assert!(opts.refspecs.is_empty());
    assert!(!opts.prune);
}

#[test]
fn test_fetch_opts_multiple_refspecs() {
    let opts = FetchOpts::from_remote("upstream")
        .add_refspec("refs/heads/main:refs/remotes/upstream/main")
        .add_refspec("refs/heads/dev:refs/remotes/upstream/dev");

    assert_eq!(opts.remote, "upstream");
    assert_eq!(opts.refspecs.len(), 2);
}
