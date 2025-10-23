//! Tests for error context validation utilities

use kodegen_tools_citescrape::mcp::validation::ErrorContext;

#[test]
fn test_error_context_formatting() {
    let msg = ErrorContext::new("Get crawl results")
        .detail("crawl_id: Some(\"abc\")")
        .detail("Session not found")
        .suggest("Verify crawl_id is correct")
        .suggest("Check if crawl completed")
        .build();

    assert!(msg.contains("Operation failed: Get crawl results"));
    assert!(msg.contains("Details:"));
    assert!(msg.contains("  - crawl_id: Some(\"abc\")"));
    assert!(msg.contains("Suggestions:"));
    assert!(msg.contains("  - Verify crawl_id is correct"));
}
