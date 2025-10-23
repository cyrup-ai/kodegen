//! Tests for git add operation.

use kodegen_tools_git::git::add::AddOpts;
use std::path::PathBuf;

#[test]
fn test_add_opts_builder() {
    let opts = AddOpts::new(vec!["file1.txt", "file2.txt"])
        .add_path("file3.txt")
        .update_only(true);

    assert_eq!(opts.paths.len(), 3);
    assert_eq!(opts.paths[0], PathBuf::from("file1.txt"));
    assert_eq!(opts.paths[1], PathBuf::from("file2.txt"));
    assert_eq!(opts.paths[2], PathBuf::from("file3.txt"));
    assert!(opts.update_only);
}

#[test]
fn test_add_opts_single_path() {
    let opts = AddOpts::new(vec!["README.md"]);

    assert_eq!(opts.paths.len(), 1);
    assert_eq!(opts.paths[0], PathBuf::from("README.md"));
    assert!(!opts.update_only);
}

#[test]
fn test_add_opts_empty_paths() {
    let opts = AddOpts::new(Vec::<String>::new());

    assert!(opts.paths.is_empty());
    assert!(!opts.update_only);
}

#[test]
fn test_add_opts_pathbuf_input() {
    let paths = vec![PathBuf::from("src/main.rs"), PathBuf::from("Cargo.toml")];
    let opts = AddOpts::new(paths);

    assert_eq!(opts.paths.len(), 2);
    assert_eq!(opts.paths[0], PathBuf::from("src/main.rs"));
    assert_eq!(opts.paths[1], PathBuf::from("Cargo.toml"));
}

#[test]
fn test_add_opts_mixed_types() {
    let opts = AddOpts::new(vec!["file.txt"])
        .add_path(PathBuf::from("dir/file.rs"))
        .add_path("another.md");

    assert_eq!(opts.paths.len(), 3);
    assert_eq!(opts.paths[0], PathBuf::from("file.txt"));
    assert_eq!(opts.paths[1], PathBuf::from("dir/file.rs"));
    assert_eq!(opts.paths[2], PathBuf::from("another.md"));
}
