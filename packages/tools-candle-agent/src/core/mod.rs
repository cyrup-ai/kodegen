//! Core domain types and traits

use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use crate::async_stream;
use arc_swap::ArcSwap;
use cyrup_sugars::prelude::MessageChunk;
use once_cell::sync::Lazy;
use tokio_stream::Stream;

use crate::domain::memory::MemoryError;

/// Domain initialization error types with semantic error handling
#[derive(Debug, Clone, thiserror::Error)]
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
}

/// Wrapper for Result types to work with AsyncStream MessageChunk constraint
#[derive(Debug, Clone)]
pub struct DomainResult<T> {
    pub result: std::result::Result<T, DomainInitError>,
}

impl<T> From<std::result::Result<T, DomainInitError>> for DomainResult<T> {
    fn from(result: std::result::Result<T, DomainInitError>) -> Self {
        DomainResult { result }
    }
}

impl<T> Default for DomainResult<T> {
    fn default() -> Self {
        DomainResult {
            result: Err(DomainInitError::System("Default DomainResult".to_string())),
        }
    }
}

impl<T> MessageChunk for DomainResult<T> {
    fn bad_chunk(error: String) -> Self {
        DomainResult {
            result: Err(DomainInitError::System(error)),
        }
    }

    fn error(&self) -> Option<&str> {
        match &self.result {
            Err(err) => match err {
                DomainInitError::Config(msg) => Some(msg),
                DomainInitError::System(msg) => Some(msg),
                DomainInitError::Pool(msg) => Some(msg),
                DomainInitError::Memory(_) => Some("Memory initialization failed"),
                DomainInitError::CircuitBreakerOpen => Some("Circuit breaker is open"),
                DomainInitError::Timeout(_) => Some("Operation timed out"),
                DomainInitError::InvalidState(msg) => Some(msg),
            },
            Ok(_) => None,
        }
    }
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
pub fn channel<T: Send + 'static>() -> (
    ChannelSender<T>,
    tokio::sync::mpsc::Receiver<std::result::Result<T, ChannelError>>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    (ChannelSender { sender: tx }, rx)
}

/// Global state for circuit breaker pattern
static CIRCUIT_BREAKER: Lazy<ArcSwap<CircuitBreaker>> =
    Lazy::new(|| ArcSwap::from_pointee(CircuitBreaker::new()));

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

/// Execute operation with circuit breaker protection using tokio stream
pub fn execute_with_circuit_breaker<F, T, E>(operation: F) -> impl Stream<Item = DomainResult<T>>
where
    F: FnOnce() -> std::result::Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Into<DomainInitError> + Send + 'static,
{
    async_stream::spawn_stream(move |tx| async move {
        let circuit_breaker = CIRCUIT_BREAKER.load();
        if circuit_breaker.is_open().await {
            let _ = tx.send(DomainResult::from(Err(DomainInitError::CircuitBreakerOpen)));
            return;
        }

        match operation() {
            Ok(result) => {
                let _ = tx.send(DomainResult::from(Ok(result)));
            }
            Err(err) => {
                circuit_breaker.record_failure().await;
                let _ = tx.send(DomainResult::from(Err(err.into())));
            }
        }
    })
}

// Re-export commonly used types
// REMOVED: pub use futures::stream::Stream; - ALL FUTURES ELIMINATED!
/// GPU device detection utilities
pub mod device_util;

/// Core engine for completion processing
pub mod engine;

/// Advanced constrained generation with sampling strategies
pub mod generation;

/// Unified model configuration system for hundreds of models
pub mod model_config;

/// SIMD adapter functions for bridging kodegen_simd with generation types
pub mod simd_adapters;

/// Tokenization utilities for text processing
pub mod tokenizer;

// Re-export core types
pub use engine::*;
pub use generation::*;
pub use model_config::*;
pub use simd_adapters::{
    should_use_simd, simd_argmax_with_bounds, simd_error_to_fallback_strategy,
    simd_softmax_with_cache, simd_temperature_scale,
};
pub use tokenizer::{CandleTokenizer, CandleTokenizerConfig};
