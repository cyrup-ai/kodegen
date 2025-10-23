//! Tests for git branch operation.

use kodegen_tools_git::git::branch::BranchOpts;

#[test]
fn test_branch_opts_builder() {
    let opts = BranchOpts::new("feature/new-feature")
        .start_point("main")
        .force(true)
        .checkout(true)
        .track(true);

    assert_eq!(opts.name, "feature/new-feature");
    assert_eq!(opts.start_point, Some("main".to_string()));
    assert!(opts.force);
    assert!(opts.checkout);
    assert!(opts.track);
}

#[test]
fn test_branch_opts_default() {
    let opts = BranchOpts::new("develop");

    assert_eq!(opts.name, "develop");
    assert_eq!(opts.start_point, None);
    assert!(!opts.force);
    assert!(!opts.checkout);
    assert!(!opts.track);
}

#[test]
fn test_branch_opts_with_commit_hash() {
    let opts = BranchOpts::new("hotfix/bug-123")
        .start_point("abc123def456")
        .force(false);

    assert_eq!(opts.name, "hotfix/bug-123");
    assert_eq!(opts.start_point, Some("abc123def456".to_string()));
    assert!(!opts.force);
}

#[test]
fn test_branch_opts_with_tag() {
    let opts = BranchOpts::new("release/v1.0.0")
        .start_point("v0.9.0")
        .checkout(true);

    assert_eq!(opts.name, "release/v1.0.0");
    assert_eq!(opts.start_point, Some("v0.9.0".to_string()));
    assert!(opts.checkout);
}

#[test]
fn test_branch_opts_chaining() {
    let opts = BranchOpts::new("test-branch")
        .force(false)
        .checkout(false)
        .track(false);

    assert_eq!(opts.name, "test-branch");
    assert!(!opts.force);
    assert!(!opts.checkout);
    assert!(!opts.track);
}

#[test]
fn test_branch_opts_complex_name() {
    let opts = BranchOpts::new("feature/user-auth/oauth2-integration");

    assert_eq!(opts.name, "feature/user-auth/oauth2-integration");
}
