//! Engine domain module
//!
//! Provides core engine functionality with true zero-allocation patterns and lock-free
//! architecture. The engine routes requests to appropriate AI providers using atomic
//! operations and borrowed data to eliminate allocations in hot paths.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::async_stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_stream::Stream;

use crate::domain::context::chunks::{CandleCompletionChunk, CandleStringChunk};
use crate::domain::model::CandleUsage;

/// Engine-specific error types with minimal allocations
#[derive(Error, Debug, Clone)]
pub enum EngineError {
    #[error("Provider not available")]
    /// The requested AI provider is not available or not configured
    ProviderNotAvailable,

    #[error("Model not found")]
    /// The specified model was not found in the provider
    ModelNotFound,

    #[error("Configuration error: {0}")]
    /// Engine configuration is invalid or incomplete
    ConfigurationError(String),

    #[error("Authentication failed")]
    /// Authentication with the AI provider failed
    AuthenticationFailed,

    #[error("Rate limit exceeded, retry after {retry_after_seconds}s")]
    /// Rate limit was exceeded by the provider
    RateLimitExceeded {
        /// Number of seconds to wait before retrying
        retry_after_seconds: u64,
    },

    #[error("Request timeout after {timeout_seconds}s")]
    /// Request timed out after the specified duration
    RequestTimeout {
        /// Number of seconds the request waited before timing out
        timeout_seconds: u64,
    },

    #[error("Network error: {0}")]
    /// Network communication error occurred
    NetworkError(String),

    #[error("Invalid input")]
    /// The provided input is invalid or malformed
    InvalidInput,

    #[error("Service unavailable")]
    /// The engine service is temporarily unavailable
    ServiceUnavailable,

    #[error("Internal error: {0}")]
    /// An unexpected internal error occurred
    InternalError(String),
}

/// Result type for engine operations
pub type EngineResult<T> = Result<T, EngineError>;

/// Engine configuration with owned strings allocated once at creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Model name for the completion request
    pub registry_key: String,
    /// Provider identifier (e.g., "openai", "anthropic", "gemini")
    pub provider: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum tokens for completion
    pub max_tokens: Option<u32>,
    /// Temperature for response randomness (0.0 - 1.0)
    pub temperature: Option<f32>,
    /// Whether to enable streaming responses
    pub enable_streaming: bool,
    /// Custom endpoint URL override
    pub endpoint_url: Option<String>,
}

impl Default for EngineConfig {
    #[inline]
    fn default() -> Self {
        Self {
            registry_key: String::from("gpt-4o-mini"),
            provider: String::from("openai"),
            api_key: None,
            timeout_seconds: 30,
            max_tokens: Some(4096),
            temperature: Some(0.0), // Global default: greedy sampling for deterministic output
            enable_streaming: false,
            endpoint_url: None,
        }
    }
}

impl EngineConfig {
    /// Create a new engine configuration
    #[inline]
    pub fn new(registry_key: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            registry_key: registry_key.into(),
            provider: provider.into(),
            ..Default::default()
        }
    }

    /// Set API key
    #[must_use]
    #[inline]
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set timeout in seconds
    #[must_use]
    #[inline]
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Set max tokens
    #[must_use]
    #[inline]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature (clamped to valid range)
    #[must_use]
    #[inline]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Enable streaming responses
    #[must_use]
    #[inline]
    pub fn with_streaming(mut self) -> Self {
        self.enable_streaming = true;
        self
    }

    /// Set custom endpoint URL
    #[must_use]
    #[inline]
    pub fn with_endpoint(mut self, endpoint_url: impl Into<String>) -> Self {
        self.endpoint_url = Some(endpoint_url.into());
        self
    }

    /// Validate configuration
    #[inline]
    pub fn validate(&self) -> EngineResult<()> {
        if self.registry_key.is_empty() {
            return Err(EngineError::ConfigurationError(
                "Model name cannot be empty".to_string(),
            ));
        }

        if self.provider.is_empty() {
            return Err(EngineError::ConfigurationError(
                "Provider cannot be empty".to_string(),
            ));
        }

        if self.timeout_seconds == 0 {
            return Err(EngineError::ConfigurationError(
                "Timeout must be greater than 0".to_string(),
            ));
        }

        if let Some(temp) = self.temperature
            && !(0.0..=1.0).contains(&temp)
        {
            return Err(EngineError::ConfigurationError(
                "Temperature must be between 0.0 and 1.0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Core engine implementation with lock-free atomic operations
#[derive(Debug, Clone)]
pub struct Engine {
    config: EngineConfig,
    request_count: Arc<AtomicU64>,
    active_requests: Arc<AtomicU64>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    is_healthy: Arc<AtomicBool>,
}

impl Engine {
    /// Create a new engine with the given configuration
    #[inline]
    pub fn new(config: EngineConfig) -> EngineResult<Self> {
        config.validate()?;

        Ok(Self {
            config,
            request_count: Arc::new(AtomicU64::new(0)),
            active_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            is_healthy: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Get immutable reference to configuration
    #[inline]
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Get current request count (atomic read)
    #[inline]
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Get current active request count (atomic read)
    #[inline]
    pub fn active_requests(&self) -> u64 {
        self.active_requests.load(Ordering::Relaxed)
    }

    /// Get successful request count (atomic read)
    #[inline]
    pub fn successful_requests(&self) -> u64 {
        self.successful_requests.load(Ordering::Relaxed)
    }

    /// Get failed request count (atomic read)
    #[inline]
    pub fn failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::Relaxed)
    }

    /// Check if engine is healthy (atomic read)
    #[inline]
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::Relaxed)
    }

    /// Set health status (atomic write)
    #[inline]
    pub fn set_healthy(&self, healthy: bool) {
        self.is_healthy.store(healthy, Ordering::Relaxed);
    }

    /// Coordinate text generation with metrics and streaming management
    ///
    /// Provides orchestration services for providers:
    /// - Automatic metrics tracking (request_count, active_requests, etc.)
    /// - Stream conversion from CandleStringChunk to CandleCompletionChunk  
    /// - Error handling and health monitoring
    /// - Performance timing and throughput calculation
    pub fn coordinate_generation<F, S>(
        &self,
        generation_fn: F,
    ) -> impl Stream<Item = CandleCompletionChunk> + use<F, S>
    where
        F: FnOnce() -> S + Send + 'static,
        S: Stream<Item = CandleStringChunk> + Send + 'static,
    {
        // Update metrics atomically
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_add(1, Ordering::Relaxed);

        // Execute provider's generation function
        let text_stream = generation_fn();

        // Convert and manage streaming response with metrics
        self.manage_streaming_response(text_stream)
    }

    /// Coordinate generation for providers that emit CandleCompletionChunk directly
    ///
    /// Use this for providers that support:
    /// - Tool/function calling with ToolCallStart, ToolCall, ToolCallComplete variants
    /// - Custom completion logic requiring direct control over chunk types
    ///
    /// This bypasses the text-to-completion conversion and provides metrics tracking only.
    pub fn coordinate_completion<F, S>(
        &self,
        generation_fn: F,
    ) -> impl Stream<Item = CandleCompletionChunk> + use<F, S>
    where
        F: FnOnce() -> S + Send + 'static,
        S: Stream<Item = CandleCompletionChunk> + Send + 'static,
    {
        // Update metrics atomically
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_add(1, Ordering::Relaxed);

        let active_requests = Arc::clone(&self.active_requests);
        let successful_requests = Arc::clone(&self.successful_requests);
        let failed_requests = Arc::clone(&self.failed_requests);

        // Execute provider's generation function
        let completion_stream = generation_fn();

        // Pass through with metrics tracking and timing augmentation
        async_stream::spawn_stream(move |tx| async move {
            use tokio_stream::StreamExt;
            let mut has_error = false;
            let mut stream = Box::pin(completion_stream);

            while let Some(chunk) = stream.next().await {
                // Check for error chunks
                if matches!(chunk, CandleCompletionChunk::Error(_)) {
                    has_error = true;
                }

                if tx.send(chunk).is_err() {
                    // Client disconnected
                    has_error = true;
                    break;
                }
            }

            // Update completion metrics
            active_requests.fetch_sub(1, Ordering::Relaxed);
            if has_error {
                failed_requests.fetch_add(1, Ordering::Relaxed);
            } else {
                successful_requests.fetch_add(1, Ordering::Relaxed);
            }
        })
    }

    /// Convert TextGenerator output to completion chunks with metrics tracking
    fn manage_streaming_response<S>(
        &self,
        text_stream: S,
    ) -> impl Stream<Item = CandleCompletionChunk> + use<S>
    where
        S: Stream<Item = CandleStringChunk> + Send + 'static,
    {
        let active_requests = Arc::clone(&self.active_requests);
        let successful_requests = Arc::clone(&self.successful_requests);
        let failed_requests = Arc::clone(&self.failed_requests);

        async_stream::spawn_stream(move |tx| async move {
            use tokio_stream::StreamExt;
            let mut has_error = false;
            let mut stream = Box::pin(text_stream);

            // Process each chunk from TextGenerator
            while let Some(string_chunk) = stream.next().await {
                // Convert CandleStringChunk to CandleCompletionChunk
                let completion_chunk = match string_chunk {
                    CandleStringChunk {
                        text,
                        is_final: false,
                        stats: _,
                    } if text.starts_with("ERROR:") => {
                        has_error = true;
                        CandleCompletionChunk::Error(
                            text.strip_prefix("ERROR:").unwrap_or(&text).to_string(),
                        )
                    }
                    CandleStringChunk {
                        text,
                        is_final: false,
                        stats: _,
                    } => CandleCompletionChunk::Text(text.clone()),
                    CandleStringChunk {
                        text: _,
                        is_final: true,
                        stats: Some(gen_stats),
                    } => {
                        // Final chunk with stats from TextGenerator - extract real timing
                        CandleCompletionChunk::Complete {
                            text: String::new(),
                            finish_reason: if has_error {
                                Some(crate::domain::context::chunks::FinishReason::Error)
                            } else {
                                Some(crate::domain::context::chunks::FinishReason::Stop)
                            },
                            usage: Some(CandleUsage {
                                input_tokens: 0,
                                output_tokens: gen_stats.tokens_generated,
                                total_tokens: gen_stats.tokens_generated,
                            }),
                            token_count: Some(gen_stats.tokens_generated),
                            elapsed_secs: Some(gen_stats.elapsed_secs),
                            tokens_per_sec: Some(gen_stats.tokens_per_sec),
                        }
                    }
                    CandleStringChunk {
                        text: _,
                        is_final: true,
                        stats: None,
                    } => {
                        // Final chunk without stats (shouldn't happen, but handle gracefully)
                        log::warn!("Received final chunk without stats - this should not happen");
                        CandleCompletionChunk::Complete {
                            text: String::new(),
                            finish_reason: if has_error {
                                Some(crate::domain::context::chunks::FinishReason::Error)
                            } else {
                                Some(crate::domain::context::chunks::FinishReason::Stop)
                            },
                            usage: None,
                            token_count: None,
                            elapsed_secs: None,
                            tokens_per_sec: None,
                        }
                    }
                };

                if tx.send(completion_chunk).is_err() {
                    // Client disconnected
                    has_error = true;
                    break;
                }
            }

            // Update metrics atomically on completion
            active_requests.fetch_sub(1, Ordering::Relaxed);
            if has_error {
                failed_requests.fetch_add(1, Ordering::Relaxed);
            } else {
                successful_requests.fetch_add(1, Ordering::Relaxed);
            }
        })
    }

    /// Get engine statistics
    #[inline]
    pub fn stats(&self) -> EngineStats {
        EngineStats {
            total_requests: self.request_count.load(Ordering::Relaxed),
            active_requests: self.active_requests.load(Ordering::Relaxed),
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            is_healthy: self.is_healthy.load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics (atomic operations)
    #[inline]
    pub fn reset_metrics(&self) {
        self.request_count.store(0, Ordering::Relaxed);
        self.active_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
    }
}

/// Engine statistics snapshot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineStats {
    pub total_requests: u64,
    pub active_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub is_healthy: bool,
}

impl EngineStats {
    /// Calculate success rate as a percentage
    #[inline]
    pub fn success_rate(&self) -> f64 {
        let completed = self.successful_requests + self.failed_requests;
        if completed == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / completed as f64) * 100.0
        }
    }

    /// Calculate failure rate as a percentage
    #[inline]
    pub fn failure_rate(&self) -> f64 {
        100.0 - self.success_rate()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            config: EngineConfig::default(),
            request_count: Arc::new(AtomicU64::new(0)),
            active_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            is_healthy: Arc::new(AtomicBool::new(true)),
        }
    }
}

// Engine is automatically Send + Sync due to atomic operations - no unsafe impl needed
