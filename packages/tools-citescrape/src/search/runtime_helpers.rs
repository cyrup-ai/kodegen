//! Runtime helper functions for async patterns with retry and fallback support
//!
//! This module provides helper functions for retry and fallback patterns
//! with efficient retry logic and exponential backoff.

use std::future::Future;
use std::time::Duration;

use super::errors::{RetryConfig, SearchResult};

/// Retry an operation with configurable retry logic
///
/// This function provides efficient retry logic with exponential backoff.
/// The retry state is maintained as local variables within the async task.
///
/// # Performance Characteristics
///
/// - **Retry state:** 16 bytes on stack (u32 attempt + u64 `last_delay`)
/// - **Backoff calculation:** Uses bit-shifting for exponential growth (zero-alloc)
/// - **Backoff delay:** Uses tokio timer (~128 bytes per sleep)
/// - **Logging:** Allocates formatted strings for tracing (50-200 bytes per retry)
/// - **Closure capture:** Allocates for config (~40 bytes) and operation closure
///
/// The implementation prioritizes correctness and debuggability over absolute zero-allocation.
/// For most use cases, these allocations are negligible compared to the I/O operations being retried.
#[inline(always)]
pub async fn retry_task<F, Fut, T>(config: RetryConfig, mut operation: F) -> SearchResult<T>
where
    F: FnMut() -> Fut + Send,
    Fut: Future<Output = SearchResult<T>> + Send,
    T: Send + 'static,
{
    let mut attempt = 0u32;
    let mut _last_delay_ms = 0u64;

    loop {
        // Execute the operation
        let result = operation().await;
        match result {
            Ok(value) => {
                if attempt > 0 {
                    tracing::info!(attempt = attempt + 1, "Operation succeeded after retry");
                }
                return Ok(value);
            }
            Err(e) => {
                // Check if error is transient
                if !e.is_transient() {
                    return Err(e);
                }

                // Check if we can retry
                if attempt >= config.max_attempts {
                    tracing::error!(
                        attempts = attempt + 1,
                        error = %e,
                        "Max retry attempts exceeded"
                    );
                    return Err(e);
                }

                attempt += 1;

                // Calculate delay using bit shifting for exponential backoff
                let delay_ms = if attempt == 1 {
                    config.initial_delay.as_millis() as u64
                } else {
                    // Use bit shifting for power of 2 calculation (zero allocation)
                    let multiplier = 1u64 << (attempt - 1).min(10); // Cap at 2^10 = 1024x
                    (config.initial_delay.as_millis() as u64).saturating_mul(multiplier)
                };

                // Cap at max delay
                let delay_ms = delay_ms.min(config.max_delay.as_millis() as u64);
                _last_delay_ms = delay_ms;

                tracing::warn!(
                    attempt = attempt,
                    max_attempts = config.max_attempts,
                    delay_ms = delay_ms,
                    error = %e,
                    "Transient error, retrying after delay"
                );

                // Use tokio sleep for the delay
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

/// Execute an operation with fallback support
///
/// This function tries a primary operation and falls back to a secondary
/// operation if the primary fails.
#[inline(always)]
pub async fn fallback_task<F, G, Fut1, Fut2, T>(primary: F, fallback: G) -> SearchResult<T>
where
    F: FnOnce() -> Fut1 + Send,
    G: FnOnce() -> Fut2 + Send,
    Fut1: Future<Output = SearchResult<T>> + Send,
    Fut2: Future<Output = SearchResult<T>> + Send,
    T: Send + 'static,
{
    let result = primary().await;
    match result {
        Ok(value) => Ok(value),
        Err(primary_error) => {
            tracing::warn!(
                error = %primary_error,
                "Primary operation failed, attempting fallback"
            );

            let result = fallback().await;
            match result {
                Ok(value) => {
                    tracing::info!("Fallback operation succeeded");
                    Ok(value)
                }
                Err(fallback_error) => {
                    tracing::error!(
                        primary_error = %primary_error,
                        fallback_error = %fallback_error,
                        "Both primary and fallback operations failed"
                    );
                    Err(primary_error) // Return primary error as it's more relevant
                }
            }
        }
    }
}
