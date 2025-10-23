use kodegen_tools_citescrape::search::errors::{RetryConfig, SearchError};
use kodegen_tools_citescrape::search::runtime_helpers::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

#[tokio::test]
async fn test_retry_task_success() {
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();

    let config = RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(10),
        backoff_multiplier: 2.0,
        max_delay: Duration::from_millis(100),
    };

    let result = retry_task(config, move || {
        let count = counter_clone.fetch_add(1, Ordering::SeqCst);
        async move {
            if count < 2 {
                Err(SearchError::WriterAcquisition("transient".to_string()))
            } else {
                Ok(42)
            }
        }
    })
    .await;

    assert_eq!(result.unwrap(), 42);
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_fallback_task() {
    let result = fallback_task(
        || async { Err(SearchError::Other("primary failed".to_string())) },
        || async { Ok(100) },
    )
    .await;

    assert_eq!(result.unwrap(), 100);
}
