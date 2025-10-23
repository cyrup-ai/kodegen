// Test that rate limiter works in async context without panicking
use kodegen_tools_citescrape::crawl_engine::rate_limiter;

#[tokio::test]
async fn test_rate_limiter_in_async_context() {
    // This test verifies that the rate limiter can be used inside tokio runtime
    // with fully async API

    let decision = rate_limiter::check_crawl_rate_limit("https://example.com", 1.0).await;

    // Should return a decision (Allow or Deny)
    match decision {
        rate_limiter::RateLimitDecision::Allow => {
            // Expected on first call
        }
        rate_limiter::RateLimitDecision::Deny { .. } => {
            // Possible if rate limited
        }
    }

    // Verify we can get domain count without panicking
    let _count = rate_limiter::get_tracked_domain_count().await;
}

#[tokio::test]
async fn test_multiple_concurrent_rate_limit_checks() {
    // Verify multiple concurrent checks work without panicking
    let urls = vec![
        "https://example1.com",
        "https://example2.com",
        "https://example3.com",
    ];

    let mut handles = vec![];
    for url in urls {
        let handle =
            tokio::spawn(async move { rate_limiter::check_crawl_rate_limit(url, 10.0).await });
        handles.push(handle);
    }

    // All should complete without panicking
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok());
    }
}
