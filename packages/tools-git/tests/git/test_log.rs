//! Tests for git log operation.

use chrono::{DateTime, Utc};
use kodegen_tools_git::git::log::LogOpts;
use std::path::PathBuf;

#[test]
fn test_log_opts_builder() {
    let since_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let until_time = DateTime::parse_from_rfc3339("2024-12-31T23:59:59Z")
        .unwrap()
        .with_timezone(&Utc);

    let opts = LogOpts::new()
        .max_count(100)
        .since(since_time)
        .until(until_time)
        .path("src/main.rs");

    assert_eq!(opts.max_count, Some(100));
    assert_eq!(opts.since, Some(since_time));
    assert_eq!(opts.until, Some(until_time));
    assert_eq!(opts.path, Some(PathBuf::from("src/main.rs")));
}

#[test]
fn test_log_opts_default() {
    let opts = LogOpts::default();

    assert_eq!(opts.max_count, None);
    assert_eq!(opts.since, None);
    assert_eq!(opts.until, None);
    assert_eq!(opts.path, None);
}

#[test]
fn test_log_opts_partial() {
    let opts = LogOpts::new().max_count(50);

    assert_eq!(opts.max_count, Some(50));
    assert_eq!(opts.since, None);
    assert_eq!(opts.until, None);
    assert_eq!(opts.path, None);
}

#[test]
fn test_log_opts_chaining() {
    let opts = LogOpts::new().max_count(10).path("README.md").max_count(20); // Should override previous max_count

    assert_eq!(opts.max_count, Some(20));
    assert_eq!(opts.path, Some(PathBuf::from("README.md")));
}

#[test]
fn test_log_opts_with_pathbuf() {
    let path = PathBuf::from("src/lib.rs");
    let opts = LogOpts::new().path(path.clone());

    assert_eq!(opts.path, Some(path));
}
