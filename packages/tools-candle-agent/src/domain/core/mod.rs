//! Core domain types and traits

use std::sync::{LazyLock, atomic::AtomicUsize};
use std::time::Duration;

use arc_swap::ArcSwap;

use crate::domain::memory::primitives::types::MemoryError;

/// Domain initialization error types with semantic error handling
#[derive(Debug, thiserror::Error)]
pub enum DomainInitError {
    /// Memory system initialization error
    #[error("Memory initialization failed: {0}")]
    Memory(#[from] MemoryError),
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    /// System error
    #[error("System error: {0}")]
    System(String),
    /// Connection pool error
    #[error("Connection pool error: {0}")]
    Pool(String),
    /// Timeout error
    #[error("Operation timed out after {:?}", .0)]
    Timeout(Duration),
    /// Circuit breaker is open
    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
    /// Invalid state error
    #[error("Invalid state: {0}")]
    InvalidState(String),
    /// Database connection failed
    #[error("Database connection failed: {0}")]
    DatabaseConnectionFailed(String),
    /// Database initialization failed
    #[error("Database initialization failed: {0}")]
    DatabaseInitializationFailed(String),
    /// Memory initialization failed
    #[error("Memory initialization failed: {0}")]
    MemoryInitializationFailed(String),
}

/// Channel error type for proper error handling
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Channel send error")]
    SendError,
    #[error("Channel receive error")]
    ReceiveError,
    #[error("Channel closed")]
    Closed,
}

/// Channel sender wrapper using tokio for async communication
pub struct ChannelSender<T> {
    sender: tokio::sync::mpsc::Sender<std::result::Result<T, ChannelError>>,
}

impl<T: Send + 'static> ChannelSender<T> {
    /// Finish the task by sending the result
    #[inline]
    pub async fn finish(self, value: T) {
        let _ = self.sender.send(Ok(value)).await;
    }

    /// Finish the task with an error
    #[inline]
    pub async fn finish_with_error(self, error: ChannelError) {
        let _ = self.sender.send(Err(error)).await;
    }
}

/// Create a new channel for async communication using tokio
#[inline]
#[must_use]
pub fn channel<T: Send + 'static>() -> (
    ChannelSender<T>,
    tokio::sync::mpsc::Receiver<std::result::Result<T, ChannelError>>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    (ChannelSender { sender: tx }, rx)
}

/// Global state for circuit breaker pattern
static CIRCUIT_BREAKER: LazyLock<ArcSwap<CircuitBreaker>> =
    LazyLock::new(|| ArcSwap::from_pointee(CircuitBreaker::new()));

/// Circuit breaker state
struct CircuitBreaker {
    _failure_count: AtomicUsize,
    reset_after: Duration,
    last_failure: tokio::sync::Mutex<Option<std::time::Instant>>,
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            _failure_count: AtomicUsize::new(0),
            reset_after: Duration::from_secs(60),
            last_failure: tokio::sync::Mutex::new(None),
        }
    }

    async fn is_open(&self) -> bool {
        let last_failure = self.last_failure.lock().await;
        if let Some(time) = *last_failure {
            time.elapsed() < self.reset_after
        } else {
            false
        }
    }

    async fn record_failure(&self) {
        let mut last_failure = self.last_failure.lock().await;
        *last_failure = Some(std::time::Instant::now());
    }
}

/// Execute operation with circuit breaker protection
///
/// # Errors
///
/// Returns `DomainInitError` if:
/// - Circuit breaker is open
/// - Operation execution fails
pub async fn execute_with_circuit_breaker<F, T, E, Fut>(
    operation: F,
) -> std::result::Result<T, DomainInitError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, E>>,
    E: Into<DomainInitError>,
{
    let circuit_breaker = CIRCUIT_BREAKER.load();
    if circuit_breaker.is_open().await {
        return Err(DomainInitError::CircuitBreakerOpen);
    }

    match operation().await {
        Ok(result) => Ok(result),
        Err(err) => {
            circuit_breaker.record_failure().await;
            Err(err.into())
        }
    }
}

// Re-export commonly used types
// REMOVED: pub use futures::stream::Stream; - ALL FUTURES ELIMINATED!
