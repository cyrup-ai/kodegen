//! Error-specific circuit breaker with statistics
//!
//! # Circuit Breaker Pattern
//!
//! The circuit breaker pattern prevents cascading failures in distributed systems by monitoring
//! operation success and failure rates. When failures exceed a configured threshold, the circuit
//! breaker temporarily halts requests to give the failing service time to recover, preventing
//! resource exhaustion and allowing graceful degradation.
//!
//! # States
//!
//! The circuit breaker operates in three distinct states:
//!
//! - **Closed**: Normal operation mode. All requests are allowed through, and failures are counted.
//!   When the failure count reaches the configured threshold, the circuit transitions to Open.
//!
//! - **Open**: Circuit is "tripped" and requests are blocked. The circuit breaker returns errors
//!   immediately without executing operations. After the configured recovery timeout elapses,
//!   the circuit transitions to Half-Open to test if the service has recovered.
//!
//! - **Half-Open**: Testing mode. Requests are allowed through to probe service health. A successful
//!   request transitions the circuit back to Closed. A failed request immediately returns the circuit
//!   to Open state.
//!
//! # State Transitions
//!
//! State transitions occur based on failure counts and timeouts:
//!
//! - **Closed → Open**: Triggered when `failure_count >= failure_threshold`
//! - **Open → Half-Open**: Triggered when `recovery_timeout` has elapsed since the last failure
//! - **Half-Open → Closed**: Triggered on successful request (via `record_success()`)
//! - **Half-Open → Open**: Triggered on failed request (via `record_failure()`)
//!
//! # Usage Example
//!
//! ```rust
//! use std::time::Duration;
//! use kodegen_candle_agent::domain::error::{ErrorCircuitBreaker, ZeroAllocError};
//!
//! // Create circuit breaker with 5 failure threshold and 30 second recovery timeout
//! let breaker = ErrorCircuitBreaker::new(5, Duration::from_secs(30));
//!
//! // Execute operation with circuit breaker protection
//! // The closure must return Result<T, E> where E: Into<ZeroAllocError>
//! let result = breaker.execute(|| -> Result<i32, ZeroAllocError> {
//!     // Your operation that might fail
//!     Ok(42)
//! });
//!
//! match result {
//!     Ok(value) => println!("Operation succeeded: {:?}", value),
//!     Err(error) => println!("Operation failed or circuit is open: {:?}", error),
//! }
//!
//! // Check circuit state
//! if breaker.is_open() {
//!     println!("Circuit is open, requests are being blocked");
//! }
//! ```
//!
//! # Thread Safety
//!
//! The circuit breaker implementation is fully thread-safe and lock-free:
//!
//! - All state management uses atomic operations with `Ordering::Relaxed`
//! - Lock-free design ensures high performance in concurrent scenarios
//! - Safe to share across threads using `Arc<ErrorCircuitBreaker>`
//! - No blocking operations or mutex contention
//!
//! # Integration
//!
//! The circuit breaker integrates with other error handling components:
//!
//! - **`ErrorCounter`**: Provides comprehensive failure statistics by category, severity, and recoverability
//! - **`ZeroAllocError`**: Zero-allocation error type for high-performance error handling
//! - **`CircuitBreakerState`**: Shared state enumeration for consistency across circuit breaker implementations

use super::core::ZeroAllocError;
use super::types::{ErrorCategory, ErrorRecoverability, ErrorSeverity};
use crate::domain::util::{duration_to_millis_u64, duration_to_nanos_u64};
use atomic_counter::{AtomicCounter, RelaxedCounter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

/// Lock-free error counter for statistics
#[derive(Debug)]
pub struct ErrorCounter {
    /// Total error count
    total: RelaxedCounter,
    /// Count by category
    by_category: [RelaxedCounter; 10],
    /// Count by severity
    by_severity: [RelaxedCounter; 4],
    /// Count by recoverability
    by_recoverability: [RelaxedCounter; 4],
    /// Last error timestamp
    last_error: AtomicU64,
}

impl ErrorCounter {
    /// Create new error counter
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            total: RelaxedCounter::new(0),
            by_category: [
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
            ],
            by_severity: [
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
            ],
            by_recoverability: [
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
                RelaxedCounter::new(0),
            ],
            last_error: AtomicU64::new(0),
        }
    }

    /// Record error occurrence
    #[inline]
    pub fn record(&self, error: &ZeroAllocError) {
        self.total.inc();
        self.by_category[error.category as usize].inc();
        self.by_severity[error.severity as usize].inc();
        self.by_recoverability[error.recoverability as usize].inc();

        let timestamp = duration_to_nanos_u64(error.timestamp.elapsed());
        self.last_error.store(timestamp, Ordering::Relaxed);
    }

    /// Get total error count
    #[inline]
    pub fn total(&self) -> usize {
        self.total.get()
    }

    /// Get error count by category
    #[inline]
    pub fn by_category(&self, category: ErrorCategory) -> usize {
        self.by_category[category as usize].get()
    }

    /// Get error count by severity
    #[inline]
    pub fn by_severity(&self, severity: ErrorSeverity) -> usize {
        self.by_severity[severity as usize].get()
    }

    /// Get error count by recoverability
    #[inline]
    pub fn by_recoverability(&self, recoverability: ErrorRecoverability) -> usize {
        self.by_recoverability[recoverability as usize].get()
    }

    /// Get last error timestamp
    #[inline]
    pub fn last_error_time(&self) -> Option<Duration> {
        let timestamp = self.last_error.load(Ordering::Relaxed);
        if timestamp > 0 {
            Some(Duration::from_nanos(timestamp))
        } else {
            None
        }
    }

    /// Reset all counters
    #[inline]
    pub fn reset(&self) {
        self.total.reset();
        for counter in &self.by_category {
            counter.reset();
        }
        for counter in &self.by_severity {
            counter.reset();
        }
        for counter in &self.by_recoverability {
            counter.reset();
        }
        self.last_error.store(0, Ordering::Relaxed);
    }
}

impl Default for ErrorCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Circuit breaker for error recovery
#[derive(Debug)]
pub struct ErrorCircuitBreaker {
    /// Error counter for statistics
    counter: ErrorCounter,
    /// Failure threshold
    failure_threshold: usize,
    /// Recovery timeout
    recovery_timeout: Duration,
    /// Half-open requests
    half_open_requests: AtomicU64,
    /// Current state (0=Closed, 1=Open, 2=HalfOpen)
    state: AtomicU64,
    /// Failure count
    failure_count: AtomicU64,
    /// Last failure time in milliseconds
    last_failure_time: AtomicU64,
}

impl ErrorCircuitBreaker {
    /// Create new circuit breaker
    #[inline]
    #[must_use]
    pub fn new(failure_threshold: usize, recovery_timeout: Duration) -> Self {
        Self {
            counter: ErrorCounter::new(),
            failure_threshold,
            recovery_timeout,
            half_open_requests: AtomicU64::new(0),
            state: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            last_failure_time: AtomicU64::new(0),
        }
    }

    /// Get recovery timeout in milliseconds
    #[inline]
    fn recovery_timeout_ms(&self) -> u64 {
        duration_to_millis_u64(self.recovery_timeout)
    }

    /// Record a failure
    #[inline]
    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if failures >= self.failure_threshold as u64 {
            self.state.store(1, Ordering::Relaxed); // Open
            let now = duration_to_millis_u64(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default(),
            );
            self.last_failure_time.store(now, Ordering::Relaxed);
        }
    }

    /// Record a success
    #[inline]
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.state.store(0, Ordering::Relaxed); // Closed
    }

    /// Check if request should be allowed
    #[inline]
    pub fn should_allow_request(&self) -> bool {
        match self.state.load(Ordering::Relaxed) {
            1 => {
                // Open
                let now = duration_to_millis_u64(
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default(),
                );
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                if now - last_failure > self.recovery_timeout_ms() {
                    self.state.store(2, Ordering::Relaxed); // HalfOpen
                    true
                } else {
                    false
                }
            }
            _ => true, // Closed, HalfOpen, or invalid state
        }
    }

    /// Execute operation with circuit breaker protection
    ///
    /// # Errors
    ///
    /// Returns `ZeroAllocError` if operation fails or circuit is open
    #[inline]
    pub fn execute<T, E, F>(&self, operation: F) -> Result<T, Box<ZeroAllocError>>
    where
        F: FnOnce() -> Result<T, E>,
        E: Into<ZeroAllocError>,
    {
        if self.should_allow_request() {
            match operation() {
                Ok(result) => {
                    self.record_success();
                    Ok(result)
                }
                Err(e) => {
                    let error = e.into();
                    self.counter.record(&error);
                    self.record_failure();
                    Err(Box::new(error))
                }
            }
        } else {
            let error = ZeroAllocError::new(
                ErrorCategory::System,
                ErrorSeverity::Error,
                ErrorRecoverability::RetriableWithBackoff,
                "Circuit breaker is open",
                500,
            );
            self.counter.record(&error);
            Err(Box::new(error))
        }
    }

    /// Check if circuit breaker is open
    #[inline]
    pub fn is_open(&self) -> bool {
        self.state.load(Ordering::Relaxed) == 1
    }

    /// Check if circuit breaker is half-open
    #[inline]
    pub fn is_half_open(&self) -> bool {
        self.state.load(Ordering::Relaxed) == 2
    }

    /// Get error statistics
    #[inline]
    pub fn stats(&self) -> &ErrorCounter {
        &self.counter
    }

    /// Reset circuit breaker
    #[inline]
    pub fn reset(&self) {
        self.counter.reset();
        self.state.store(0, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
        self.last_failure_time.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);
    }

    /// Get configured failure threshold
    #[inline]
    #[must_use]
    pub fn failure_threshold(&self) -> usize {
        self.failure_threshold
    }

    /// Get configured recovery timeout
    #[inline]
    #[must_use]
    pub fn recovery_timeout(&self) -> Duration {
        self.recovery_timeout
    }
}
