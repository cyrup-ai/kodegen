//! Tests for library root module.

use kodegen_tools_github::{GitHubError, SearchOrder, UserSearchSort};

#[test]
fn test_error_types() {
    // Test that error types can be constructed
    let _error: GitHubError = GitHubError::RateLimitExceeded;
}

#[test]
fn test_search_order() {
    // Test SearchOrder enum
    assert_eq!(SearchOrder::Asc.as_str(), "asc");
    assert_eq!(SearchOrder::Desc.as_str(), "desc");
}

#[test]
fn test_user_search_sort() {
    // Test UserSearchSort enum
    assert_eq!(UserSearchSort::Followers.as_str(), "followers");
    assert_eq!(UserSearchSort::Repositories.as_str(), "repositories");
    assert_eq!(UserSearchSort::Joined.as_str(), "joined");
}

#[test]
fn test_runtime_types_exported() {
    // Verify runtime types are exported from library root
    use kodegen_tools_github::{AsyncStream, AsyncTask};

    // These types should be available for use
    let _task_type: Option<AsyncTask<i32>> = None;
    let _stream_type: Option<AsyncStream<String>> = None;
}
