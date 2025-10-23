//! Tests for git commit operation.

use chrono::{DateTime, Utc};
use kodegen_tools_git::git::commit::{CommitOpts, Signature};

#[test]
fn test_signature_creation() {
    let sig = Signature::new("John Doe", "john@example.com");
    assert_eq!(sig.name, "John Doe");
    assert_eq!(sig.email, "john@example.com");
    // Time should be recent (within last minute)
    let duration_secs = (Utc::now() - sig.time).num_seconds();
    assert!(duration_secs.abs() < 60);
}

#[test]
fn test_signature_with_time() {
    let time = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let sig = Signature::with_time("Jane Doe", "jane@example.com", time);

    assert_eq!(sig.name, "Jane Doe");
    assert_eq!(sig.email, "jane@example.com");
    assert_eq!(sig.time, time);
}

#[test]
fn test_commit_opts_builder() {
    let author = Signature::new("John Doe", "john@example.com");
    let committer = Signature::new("Jane Doe", "jane@example.com");

    let opts = CommitOpts::message("Initial commit")
        .amend(true)
        .all(true)
        .author(author.clone())
        .committer(committer.clone());

    assert_eq!(opts.message, "Initial commit");
    assert!(opts.amend);
    assert!(opts.all);
    assert_eq!(opts.author, Some(author));
    assert_eq!(opts.committer, Some(committer));
}

#[test]
fn test_commit_opts_default() {
    let opts = CommitOpts::message("Fix bug");

    assert_eq!(opts.message, "Fix bug");
    assert!(!opts.amend);
    assert!(!opts.all);
    assert!(opts.author.is_none());
    assert!(opts.committer.is_none());
}

#[test]
fn test_commit_opts_multiline_message() {
    let message = "Fix critical bug\n\nThis commit fixes a critical bug that was causing\nthe application to crash on startup.";
    let opts = CommitOpts::message(message);

    assert_eq!(opts.message, message);
}

#[test]
fn test_commit_opts_chaining() {
    let opts = CommitOpts::message("Test commit").amend(false).all(false);

    assert_eq!(opts.message, "Test commit");
    assert!(!opts.amend);
    assert!(!opts.all);
}
