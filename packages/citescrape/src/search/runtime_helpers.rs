//! Runtime helper functions for async patterns with retry and fallback support
//!
//! This module provides helper functions for retry and fallback patterns
//! that work with AsyncTask, providing efficient retry logic with exponential backoff.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use std::future::Future;

use crate::runtime::{AsyncTask, spawn_async};
use super::errors::{SearchResult, RetryConfig};

/// Retry an operation that returns AsyncTask with configurable retry logic
///
/// This function provides efficient retry logic for AsyncTask operations with exponential
/// backoff. The retry state is maintained as local variables within the async task.
///
/// # Performance Characteristics
///
/// - **Retry state:** 16 bytes on stack (u32 attempt + u64 last_delay)
/// - **Backoff calculation:** Uses bit-shifting for exponential growth (zero-alloc)
/// - **Backoff delay:** Uses tokio timer (~128 bytes per sleep)
/// - **Logging:** Allocates formatted strings for tracing (50-200 bytes per retry)
/// - **Closure capture:** Allocates for config (~40 bytes) and operation closure
///
/// The implementation prioritizes correctness and debuggability over absolute zero-allocation.
/// For most use cases, these allocations are negligible compared to the I/O operations being retried.
#[inline(always)]
pub fn retry_task<F, T>(
    config: RetryConfig,
    mut operation: F,
) -> AsyncTask<SearchResult<T>>
where
    F: FnMut() -> AsyncTask<SearchResult<T>> + Send + 'static,
    T: Send + 'static,
{
    spawn_async(async move {
        let mut attempt = 0u32;
        let mut _last_delay_ms = 0u64;
        
        loop {
            // Execute the operation
            let task = operation();
            match task.await? {
                Ok(result) => {
                    if attempt > 0 {
                        tracing::info!(
                            attempt = attempt + 1,
                            "Operation succeeded after retry"
                        );
                    }
                    return Ok(result);
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
    })
}

/// Execute an operation with fallback support
///
/// This function tries a primary operation and falls back to a secondary
/// operation if the primary fails. Both operations must return AsyncTask.
#[inline(always)]
pub fn fallback_task<F, G, T>(
    primary: F,
    fallback: G,
) -> AsyncTask<SearchResult<T>>
where
    F: FnOnce() -> AsyncTask<SearchResult<T>> + Send + 'static,
    G: FnOnce() -> AsyncTask<SearchResult<T>> + Send + 'static,
    T: Send + 'static,
{
    spawn_async(async move {
        let primary_task = primary();
        match primary_task.await? {
            Ok(result) => Ok(result),
            Err(primary_error) => {
                tracing::warn!(
                    error = %primary_error,
                    "Primary operation failed, attempting fallback"
                );
                
                let fallback_task = fallback();
                match fallback_task.await? {
                    Ok(result) => {
                        tracing::info!("Fallback operation succeeded");
                        Ok(result)
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
    })
}

/// Cancellable task wrapper for graceful shutdown
pub struct CancellableTask<T> {
    task: AsyncTask<T>,
    cancel_flag: Arc<AtomicBool>,
}

impl<T> CancellableTask<T> {
    /// Create a new cancellable task
    #[inline]
    pub fn new<F, Fut>(cancel_flag: Arc<AtomicBool>, f: F) -> Self
    where
        F: FnOnce(Arc<AtomicBool>) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let flag = cancel_flag.clone();
        let task = spawn_async(async move {
            f(flag).await
        });
        
        Self {
            task,
            cancel_flag,
        }
    }
    
    /// Cancel the task
    #[inline]
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Release);
    }
    
    /// Check if cancelled
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Acquire)
    }
    
    /// Await the task result
    #[inline]
    pub async fn await_result(self) -> T {
        match self.task.await {
            Ok(value) => value,
            Err(e) => panic!("Cancellable task execution failed: {}", e),
        }
    }
}

/// Rate-limited task execution helper
pub struct RateLimitedTask {
    last_execution: AtomicU64,
    min_interval_nanos: u64,
}

impl RateLimitedTask {
    /// Create a new rate-limited task executor
    #[inline]
    pub const fn new(min_interval: Duration) -> Self {
        Self {
            last_execution: AtomicU64::new(0),
            min_interval_nanos: min_interval.as_nanos() as u64,
        }
    }
    
    /// Execute a task if enough time has passed since last execution
    #[inline(always)]
    pub fn try_execute<F, T>(&self, f: F) -> Option<AsyncTask<T>>
    where
        F: FnOnce() -> AsyncTask<T> + Send + 'static,
        T: Send + 'static,
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_nanos() as u64;
        
        let last = self.last_execution.load(Ordering::Acquire);
        
        if now.saturating_sub(last) >= self.min_interval_nanos {
            // Try to update the last execution time
            match self.last_execution.compare_exchange(
                last,
                now,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => Some(f()),
                Err(_) => None, // Another thread beat us to it
            }
        } else {
            None
        }
    }
}

/// Batch multiple operations into a single AsyncTask for efficiency
pub struct BatchedTask<T> {
    batch_size: usize,
    operations: Vec<Box<dyn FnOnce() -> T + Send>>,
}

impl<T: Send + 'static> BatchedTask<T> {
    /// Create a new batched task executor
    #[inline]
    pub fn with_capacity(batch_size: usize) -> Self {
        Self {
            batch_size,
            operations: Vec::with_capacity(batch_size),
        }
    }
    
    /// Add an operation to the batch
    #[inline]
    pub fn add<F>(&mut self, f: F) -> bool
    where
        F: FnOnce() -> T + Send + 'static,
    {
        if self.operations.len() < self.batch_size {
            self.operations.push(Box::new(f));
            true
        } else {
            false
        }
    }
    
    /// Execute all batched operations
    #[inline]
    pub fn execute(self) -> AsyncTask<Vec<T>> {
        spawn_async(async move {
            let mut results = Vec::with_capacity(self.operations.len());
            for op in self.operations {
                results.push(op());
            }
            results
        })
    }
}

use std::sync::Arc;

