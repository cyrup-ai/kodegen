//! Tests for git clone operation.

use kodegen_tools_git::git::clone::CloneOpts;
use std::path::PathBuf;

#[test]
fn test_clone_opts_builder() {
    let opts = CloneOpts::new("https://github.com/user/repo.git", "/tmp/repo")
        .branch("main")
        .shallow(1)
        .bare(true);

    assert_eq!(opts.url, "https://github.com/user/repo.git");
    assert_eq!(opts.destination, PathBuf::from("/tmp/repo"));
    assert_eq!(opts.branch, Some("main".to_string()));
    assert_eq!(opts.shallow, Some(1));
    assert!(opts.bare);
}

#[test]
fn test_clone_opts_defaults() {
    let opts = CloneOpts::new("https://github.com/user/repo.git", "/tmp/repo");

    assert_eq!(opts.url, "https://github.com/user/repo.git");
    assert_eq!(opts.destination, PathBuf::from("/tmp/repo"));
    assert_eq!(opts.branch, None);
    assert_eq!(opts.shallow, None);
    assert!(!opts.bare);
}
