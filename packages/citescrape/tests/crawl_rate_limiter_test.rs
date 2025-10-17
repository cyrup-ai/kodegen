use kodegen_citescrape::crawl_rate_limiter::*;
use std::thread;

#[test]
fn test_extract_domain() {
    assert_eq!(extract_domain("https://example.com"), Some("example.com".to_string()));
    assert_eq!(extract_domain("https://www.example.com"), Some("example.com".to_string()));
    assert_eq!(extract_domain("https://example.com/path"), Some("example.com".to_string()));
    assert_eq!(extract_domain("https://example.com:8080"), Some("example.com".to_string()));
    assert_eq!(extract_domain("https://sub.example.com"), Some("sub.example.com".to_string()));
    assert_eq!(extract_domain("example.com"), Some("example.com".to_string()));
    assert_eq!(extract_domain("www.example.com"), Some("example.com".to_string()));
}

#[test]
fn test_rate_limit_basic() {
    clear_domain_limiters();
    
    // First request should be allowed
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Allow
    );
    
    // Immediate second request should be denied
    assert!(matches!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Deny { .. }
    ));
}

#[test]
fn test_per_domain_limiting() {
    clear_domain_limiters();
    
    // Requests to different domains should be independent
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Allow
    );
    assert_eq!(
        check_crawl_rate_limit("https://different.com", 1.0),
        RateLimitDecision::Allow
    );
    
    // Second requests should both be denied
    assert!(matches!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Deny { .. }
    ));
    assert!(matches!(
        check_crawl_rate_limit("https://different.com", 1.0),
        RateLimitDecision::Deny { .. }
    ));
}

#[test]
fn test_invalid_rates() {
    // Zero or negative rates should allow all requests
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 0.0),
        RateLimitDecision::Allow
    );
    assert_eq!(
        check_crawl_rate_limit("https://example.com", -1.0),
        RateLimitDecision::Allow
    );
}

#[test]
fn test_invalid_urls() {
    // Invalid URLs should be allowed
    assert_eq!(
        check_crawl_rate_limit("", 1.0),
        RateLimitDecision::Allow
    );
    assert_eq!(
        check_crawl_rate_limit("not-a-url", 1.0),
        RateLimitDecision::Allow
    );
}

#[test]
fn test_thread_local_isolation() {
    clear_domain_limiters();
    
    // Use domain in this thread
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Allow
    );
    assert!(matches!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Deny { .. }
    ));
    
    // Different thread should have independent state
    let handle = thread::spawn(|| {
        // First request in new thread should be allowed
        assert_eq!(
            check_crawl_rate_limit("https://example.com", 1.0),
            RateLimitDecision::Allow
        );
    });
    
    handle.join().unwrap();
}

#[test]
fn test_high_rate_limits() {
    clear_domain_limiters();
    
    // High rate limits should allow multiple requests
    let high_rate = 100.0; // 100 RPS
    
    let mut allowed_count = 0;
    for _ in 0..10 {
        if check_crawl_rate_limit("https://example.com", high_rate) == RateLimitDecision::Allow {
            allowed_count += 1;
        }
    }
    
    // Should allow multiple requests with high rate
    assert!(allowed_count > 1);
}

#[test]
fn test_domain_normalization() {
    clear_domain_limiters();
    
    // These should all be treated as the same domain
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Allow
    );
    assert!(matches!(
        check_crawl_rate_limit("https://www.example.com", 1.0),
        RateLimitDecision::Deny { .. }
    ));
    assert!(matches!(
        check_crawl_rate_limit("https://EXAMPLE.COM", 1.0),
        RateLimitDecision::Deny { .. }
    ));
}

#[test]
fn test_clear_limiters() {
    clear_domain_limiters();
    
    // Use up token
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Allow
    );
    assert!(matches!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Deny { .. }
    ));
    
    // Clear limiters
    clear_domain_limiters();
    
    // Should be allowed again after clearing
    assert_eq!(
        check_crawl_rate_limit("https://example.com", 1.0),
        RateLimitDecision::Allow
    );
}

#[test]
fn test_tracked_domain_count() {
    clear_domain_limiters();
    assert_eq!(get_tracked_domain_count(), 0);
    
    // Add some domains
    check_crawl_rate_limit("https://example.com", 1.0);
    assert_eq!(get_tracked_domain_count(), 1);
    
    check_crawl_rate_limit("https://different.com", 1.0);
    assert_eq!(get_tracked_domain_count(), 2);
    
    // Same domain shouldn't increase count
    check_crawl_rate_limit("https://example.com/path", 1.0);
    assert_eq!(get_tracked_domain_count(), 2);
}