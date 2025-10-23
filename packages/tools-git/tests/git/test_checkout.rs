//! Tests for git checkout operation.

use kodegen_tools_git::git::checkout::CheckoutOpts;

#[test]
fn test_checkout_opts_builder() {
    let opts = CheckoutOpts::new("main").force(true);

    assert_eq!(opts.reference, "main");
    assert!(opts.force);
}

#[test]
fn test_checkout_opts_default() {
    let opts = CheckoutOpts::new("feature/branch");

    assert_eq!(opts.reference, "feature/branch");
    assert!(!opts.force);
}

#[test]
fn test_checkout_opts_with_commit_hash() {
    let opts = CheckoutOpts::new("abc123def456").force(false);

    assert_eq!(opts.reference, "abc123def456");
    assert!(!opts.force);
}

#[test]
fn test_checkout_opts_with_tag() {
    let opts = CheckoutOpts::new("v1.0.0");

    assert_eq!(opts.reference, "v1.0.0");
    assert!(!opts.force);
}
