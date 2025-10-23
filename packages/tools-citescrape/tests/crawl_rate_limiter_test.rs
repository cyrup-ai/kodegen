use kodegen_tools_citescrape::crawl_rate_limiter::*;

#[test]
fn test_extract_domain() {
    assert_eq!(
        extract_domain("https://example.com"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_domain("https://www.example.com"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_domain("https://example.com/path"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_domain("https://example.com:8080"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_domain("https://sub.example.com"),
        Some("sub.example.com".to_string())
    );
    assert_eq!(
        extract_domain("example.com"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_domain("www.example.com"),
        Some("example.com".to_string())
    );
}

#[tokio::test]
async fn test_rate_limit_basic() {
    clear_domain_limiters().await;

    // First request should be allowed
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Allow
    );

    // Immediate second request should be denied
    assert!(matches!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Deny { .. }
    ));
}

#[tokio::test]
async fn test_per_domain_limiting() {
    clear_domain_limiters().await;

    // Requests to different domains should be independent
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Allow
    );
    assert_eq!(
        check_crawl_rate_limit("https://different.com", 1.0).await,
        RateLimitDecision::Allow
    );

    // Second requests should both be denied
    assert!(matches!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Deny { .. }
    ));
    assert!(matches!(
        check_crawl_rate_limit("https://different.com", 1.0).await,
        RateLimitDecision::Deny { .. }
    ));
}

#[tokio::test]
async fn test_invalid_rates() {
    // Zero or negative rates should allow all requests
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 0.0).await,
        RateLimitDecision::Allow
    );
    assert_eq!(
        check_crawl_rate_limit("https://example.com", -1.0).await,
        RateLimitDecision::Allow
    );
}

#[tokio::test]
async fn test_invalid_urls() {
    // Invalid URLs should be allowed
    assert_eq!(
        check_crawl_rate_limit("", 1.0).await,
        RateLimitDecision::Allow
    );
    assert_eq!(
        check_crawl_rate_limit("not-a-url", 1.0).await,
        RateLimitDecision::Allow
    );
}

#[tokio::test]
async fn test_concurrent_tasks() {
    clear_domain_limiters().await;

    // Use domain in this task
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Allow
    );
    assert!(matches!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Deny { .. }
    ));

    // Spawn concurrent task - they share the same rate limiter state
    let handle = tokio::spawn(async {
        // Should be rate limited since we already consumed the token
        assert!(matches!(
            check_crawl_rate_limit("https://example.com", 1.0).await,
            RateLimitDecision::Deny { .. }
        ));
    });

    handle.await.unwrap();
}

#[tokio::test]
async fn test_high_rate_limits() {
    clear_domain_limiters().await;

    // High rate limits should allow multiple requests
    let high_rate = 100.0; // 100 RPS

    let mut allowed_count = 0;
    for _ in 0..10 {
        if check_crawl_rate_limit("https://example.com", high_rate).await
            == RateLimitDecision::Allow
        {
            allowed_count += 1;
        }
    }

    // Should allow multiple requests with high rate
    assert!(allowed_count > 1);
}

#[tokio::test]
async fn test_domain_normalization() {
    clear_domain_limiters().await;

    // These should all be treated as the same domain
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Allow
    );
    assert!(matches!(
        check_crawl_rate_limit("https://www.example.com", 1.0).await,
        RateLimitDecision::Deny { .. }
    ));
    assert!(matches!(
        check_crawl_rate_limit("https://EXAMPLE.COM", 1.0).await,
        RateLimitDecision::Deny { .. }
    ));
}

#[tokio::test]
async fn test_clear_limiters() {
    clear_domain_limiters().await;

    // Use up token
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Allow
    );
    assert!(matches!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Deny { .. }
    ));

    // Clear limiters
    clear_domain_limiters().await;

    // Should be allowed again after clearing
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0).await,
        RateLimitDecision::Allow
    );
}

#[tokio::test]
async fn test_tracked_domain_count() {
    clear_domain_limiters().await;
    assert_eq!(get_tracked_domain_count().await, 0);

    // Add some domains
    check_crawl_rate_limit("https://example.com", 1.0).await;
    assert_eq!(get_tracked_domain_count().await, 1);

    check_crawl_rate_limit("https://different.com", 1.0).await;
    assert_eq!(get_tracked_domain_count().await, 2);

    // Same domain shouldn't increase count
    check_crawl_rate_limit("https://example.com/path", 1.0).await;
    assert_eq!(get_tracked_domain_count().await, 2);
}
