//! Error statistics and aggregation

use super::breaker::{ErrorCircuitBreaker, ErrorCounter};
use super::core::ZeroAllocError;
use super::types::ErrorCategory;
use crate::domain::util::duration_to_nanos_u64;
use std::sync::{
    LazyLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

/// Global error aggregator with lock-free statistics
#[derive(Debug)]
pub struct ErrorAggregator {
    /// Error counters by category
    counters: [ErrorCounter; 10],
    /// Circuit breakers by category
    breakers: [ErrorCircuitBreaker; 10],
    /// Global error rate limiter
    rate_limiter: AtomicU64,
    /// Last rate limit reset
    last_reset: AtomicU64,
    /// Rate limit window
    rate_window: Duration,
    /// Maximum errors per window
    max_errors_per_window: usize,
}

impl ErrorAggregator {
    /// Create new error aggregator
    #[inline]
    #[must_use]
    pub fn new(max_errors_per_window: usize, rate_window: Duration) -> Self {
        fn create_counter() -> ErrorCounter {
            ErrorCounter::new()
        }

        Self {
            counters: [
                create_counter(),
                create_counter(),
                create_counter(),
                create_counter(),
                create_counter(),
                create_counter(),
                create_counter(),
                create_counter(),
                create_counter(),
                create_counter(),
            ],
            breakers: [
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
                ErrorCircuitBreaker::new(10, Duration::from_secs(30)),
            ],
            rate_limiter: AtomicU64::new(0),
            last_reset: AtomicU64::new(0),
            rate_window,
            max_errors_per_window,
        }
    }

    /// Record error with rate limiting
    #[inline]
    pub fn record(&self, error: &ZeroAllocError) -> bool {
        // Check rate limit
        if !self.check_rate_limit() {
            return false;
        }

        // Record in category-specific counter
        self.counters[error.category as usize].record(error);

        true
    }

    /// Check rate limit
    #[inline]
    fn check_rate_limit(&self) -> bool {
        let now = duration_to_nanos_u64(Instant::now().elapsed());
        let last_reset = self.last_reset.load(Ordering::Relaxed);
        let rate_window_nanos = duration_to_nanos_u64(self.rate_window);

        // Reset rate limiter if window expired
        if now - last_reset > rate_window_nanos {
            self.rate_limiter.store(0, Ordering::Relaxed);
            self.last_reset.store(now, Ordering::Relaxed);
        }

        // Check if under rate limit
        let current_count = self.rate_limiter.load(Ordering::Relaxed);
        if current_count < self.max_errors_per_window as u64 {
            self.rate_limiter.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Get error statistics by category
    #[inline]
    pub fn stats(&self, category: ErrorCategory) -> &ErrorCounter {
        &self.counters[category as usize]
    }

    /// Get circuit breaker by category
    #[inline]
    pub fn breaker(&self, category: ErrorCategory) -> &ErrorCircuitBreaker {
        &self.breakers[category as usize]
    }

    /// Get total error count across all categories
    #[inline]
    pub fn total_errors(&self) -> usize {
        self.counters.iter().map(ErrorCounter::total).sum()
    }

    /// Reset all statistics
    #[inline]
    pub fn reset(&self) {
        for counter in &self.counters {
            counter.reset();
        }
        for breaker in &self.breakers {
            breaker.reset();
        }
        self.rate_limiter.store(0, Ordering::Relaxed);
        self.last_reset.store(0, Ordering::Relaxed);
    }
}

/// Global error aggregator instance
static ERROR_AGGREGATOR: LazyLock<ErrorAggregator> =
    LazyLock::new(|| ErrorAggregator::new(1000, Duration::from_secs(60)));

/// Record error in global aggregator
#[inline]
pub fn record_error(error: &ZeroAllocError) -> bool {
    ERROR_AGGREGATOR.record(error)
}

/// Get global error statistics
#[inline]
pub fn error_stats(category: ErrorCategory) -> &'static ErrorCounter {
    ERROR_AGGREGATOR.stats(category)
}

/// Get global circuit breaker
#[inline]
pub fn error_breaker(category: ErrorCategory) -> &'static ErrorCircuitBreaker {
    ERROR_AGGREGATOR.breaker(category)
}

/// Get total error count
#[inline]
pub fn total_errors() -> usize {
    ERROR_AGGREGATOR.total_errors()
}

/// Reset global error statistics
#[inline]
pub fn reset_error_stats() {
    ERROR_AGGREGATOR.reset();
}
