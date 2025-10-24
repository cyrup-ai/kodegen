//! Generic circuit breaker implementation for fault tolerance

use crate::domain::util::duration_to_millis_u64;
use std::sync::atomic::{AtomicU64, Ordering};

/// Circuit breaker state for error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Circuit is closed, allowing operations
    Closed,
    /// Circuit is open, rejecting operations
    Open,
    /// Circuit is half-open, testing if service has recovered
    HalfOpen,
}

/// Production circuit breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Current state
    state: AtomicU64, // 0=Closed, 1=Open, 2=HalfOpen
    /// Failure count
    failure_count: AtomicU64,
    /// Last failure time
    last_failure_time: AtomicU64,
    /// Failure threshold
    failure_threshold: u64,
    /// Recovery timeout in milliseconds
    recovery_timeout_ms: u64,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    #[must_use]
    pub fn new(failure_threshold: u64, recovery_timeout_ms: u64) -> Self {
        Self {
            state: AtomicU64::new(0), // Closed
            failure_count: AtomicU64::new(0),
            last_failure_time: AtomicU64::new(0),
            failure_threshold,
            recovery_timeout_ms,
        }
    }

    /// Execute operation with circuit breaker protection
    ///
    /// # Errors
    ///
    /// Returns `CircuitBreakerError::CircuitOpen` if circuit is open, or wraps inner error
    pub fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<T, E>,
    {
        match self.get_state() {
            CircuitBreakerState::Open => {
                // Check if we should transition to half-open
                let now = duration_to_millis_u64(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default(),
                );
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);

                if now - last_failure > self.recovery_timeout_ms {
                    self.state.store(2, Ordering::Relaxed); // HalfOpen
                    self.execute_operation(operation)
                } else {
                    Err(CircuitBreakerError::CircuitOpen)
                }
            }
            CircuitBreakerState::HalfOpen | CircuitBreakerState::Closed => {
                self.execute_operation(operation)
            }
        }
    }

    fn execute_operation<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<T, E>,
    {
        match operation() {
            Ok(result) => {
                // Success - reset failure count and close circuit
                self.failure_count.store(0, Ordering::Relaxed);
                self.state.store(0, Ordering::Relaxed); // Closed
                Ok(result)
            }
            Err(error) => {
                // Failure - increment count and potentially open circuit
                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

                if failures >= self.failure_threshold {
                    self.state.store(1, Ordering::Relaxed); // Open
                    let now = duration_to_millis_u64(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default(),
                    );
                    self.last_failure_time.store(now, Ordering::Relaxed);
                }

                Err(CircuitBreakerError::Inner(error))
            }
        }
    }

    pub fn get_state(&self) -> CircuitBreakerState {
        match self.state.load(Ordering::Relaxed) {
            1 => CircuitBreakerState::Open,
            2 => CircuitBreakerState::HalfOpen,
            _ => CircuitBreakerState::Closed, // Default fallback for 0 and unknown states
        }
    }
}

/// Circuit breaker error wrapper
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Inner operation error
    Inner(E),
    /// Circuit breaker is open
    CircuitOpen,
}
