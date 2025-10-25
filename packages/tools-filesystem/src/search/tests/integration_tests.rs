//! Integration tests for word boundary mode
//!
//! These tests verify that word boundary mode reduces false positives
//! when searching real files.

use crate::search::rg::build_rust_matcher;
use crate::search::types::CaseMode;
use grep::searcher::{Searcher, Sink, SinkMatch};

/// Simple sink that counts matches
struct CountSink {
    count: usize,
}

impl CountSink {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Sink for CountSink {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, _mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        self.count += 1;
        Ok(true)
    }
}

#[test]
fn test_word_boundary_reduces_false_positives() {
    // Create test content with various matches
    let test_content = b"\
fn test() { }
fn test_user() { }
fn testing() { }
let contest = \"value\";
// test comment
fn attest() { }
";

    // Search WITHOUT word boundary (substring mode)
    let matcher_no_boundary = build_rust_matcher("test", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    let mut searcher = Searcher::new();
    let mut sink_no_boundary = CountSink::new();
    searcher
        .search_slice(&matcher_no_boundary, test_content, &mut sink_no_boundary)
        .unwrap_or_else(|e| panic!("Search failed: {e}"));

    // Search WITH word boundary
    let matcher_with_boundary = build_rust_matcher("test", CaseMode::Sensitive, false, true)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    let mut sink_with_boundary = CountSink::new();
    searcher
        .search_slice(
            &matcher_with_boundary,
            test_content,
            &mut sink_with_boundary,
        )
        .unwrap_or_else(|e| panic!("Search failed: {e}"));

    // Verify that word boundary mode reduces matches
    assert!(
        sink_no_boundary.count > sink_with_boundary.count,
        "Word boundary should reduce matches (no boundary: {}, with boundary: {})",
        sink_no_boundary.count,
        sink_with_boundary.count
    );

    // Specifically: without boundary should match all 6 lines
    // with boundary should match only 2 lines (fn test() and // test comment)
    assert_eq!(
        sink_no_boundary.count, 6,
        "Without boundary should match 6 lines"
    );
    assert_eq!(
        sink_with_boundary.count, 2,
        "With boundary should match 2 lines"
    );
}

#[test]
fn test_word_boundary_literal_vs_regex() {
    // Verify that literal mode correctly escapes special characters
    let test_content = b"\
test.log file
testXlog file
test-log file
";

    // Test literal search with word boundary for "test.log"
    let matcher = build_rust_matcher("test.log", CaseMode::Sensitive, true, true)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    let mut searcher = Searcher::new();
    let mut sink = CountSink::new();
    searcher
        .search_slice(&matcher, test_content, &mut sink)
        .unwrap_or_else(|e| panic!("Search failed: {e}"));

    // Should only match "test.log" (dot is escaped), not "testXlog"
    assert_eq!(
        sink.count, 1,
        "Literal search should match exactly one line (test.log)"
    );
}
