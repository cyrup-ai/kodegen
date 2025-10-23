use kodegen_tools_citescrape::search::errors::*;
use std::time::Duration;

#[test]
fn test_error_transient_detection() {
    let transient = SearchError::WriterAcquisition("test".to_string());
    assert!(transient.is_transient());
    assert!(transient.retry_delay().is_some());

    let permanent = SearchError::QueryParsing("invalid query".to_string());
    assert!(!permanent.is_transient());
    assert!(permanent.retry_delay().is_none());
}

#[test]
fn test_retry_config_delays() {
    let config = RetryConfig::default();

    // Test exponential backoff
    assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
    assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
    assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));

    // Test max delay cap
    assert_eq!(config.delay_for_attempt(10), config.max_delay);
}
