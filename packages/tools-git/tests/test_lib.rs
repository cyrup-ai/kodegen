//! Tests for library root module.

use kodegen_tools_git::GitError;
use kodegen_tools_git::git::log::LogOpts;
use kodegen_tools_git::git::open::RepositoryInfo;
use std::path::PathBuf;

#[test]
fn test_error_types() {
    let _error = GitError::Unsupported("test operation");
    let _error = GitError::Parse("test parse error".to_string());
    let _error = GitError::BranchNotFound("main".to_string());
}

#[test]
fn test_repository_info() {
    let info = RepositoryInfo {
        path: PathBuf::from("/test"),
        is_bare: false,
        git_dir: PathBuf::from("/test/.git"),
        work_dir: Some(PathBuf::from("/test")),
    };

    assert_eq!(info.path, PathBuf::from("/test"));
    assert!(!info.is_bare);
    assert_eq!(info.git_dir, PathBuf::from("/test/.git"));
    assert_eq!(info.work_dir, Some(PathBuf::from("/test")));
}

#[test]
fn test_log_opts_builder() {
    let opts = LogOpts::new().max_count(100).path("src/main.rs");

    assert_eq!(opts.max_count, Some(100));
    assert_eq!(opts.path, Some(PathBuf::from("src/main.rs")));
}
