//! Generation statistics and performance monitoring
//!
//! This module provides comprehensive statistics tracking for text generation,
//! including performance metrics, token throughput analysis, and SIMD utilization
//! reporting. Consolidates duplicate GenerationStatistics definitions.

use std::time::{Duration, Instant};

/// Comprehensive generation statistics and performance metrics
///
/// Tracks all aspects of text generation performance including timing,
/// throughput, SIMD utilization, and resource usage. Provides methods
/// for analysis and reporting.
#[derive(Debug, Clone)]
pub struct GenerationStatistics {
    /// Total number of tokens generated
    pub total_tokens: u64,

    /// Number of input tokens processed
    pub input_tokens: u64,

    /// Total generation time
    pub total_duration: Duration,

    /// Time spent on prompt processing
    pub prompt_processing_duration: Duration,

    /// Time spent on token generation (excluding prompt)
    pub token_generation_duration: Duration,

    /// Number of forward passes through the model
    pub forward_passes: u64,

    /// Number of times SIMD acceleration was used
    pub simd_operations: u64,

    /// Number of times scalar fallback was used
    pub scalar_operations: u64,

    /// Peak memory usage during generation (bytes)
    pub peak_memory_bytes: u64,

    /// Number of cache hits during generation
    pub cache_hits: u64,

    /// Number of cache misses during generation
    pub cache_misses: u64,

    /// Start time of generation for ongoing tracking
    generation_start: Option<Instant>,
}
impl GenerationStatistics {
    /// Create new GenerationStatistics
    pub fn new() -> Self {
        Self {
            total_tokens: 0,
            input_tokens: 0,
            total_duration: Duration::ZERO,
            prompt_processing_duration: Duration::ZERO,
            token_generation_duration: Duration::ZERO,
            forward_passes: 0,
            simd_operations: 0,
            scalar_operations: 0,
            peak_memory_bytes: 0,
            cache_hits: 0,
            cache_misses: 0,
            generation_start: None,
        }
    }

    /// Start tracking generation time
    pub fn start_generation(&mut self) {
        self.generation_start = Some(Instant::now());
    }

    /// Stop tracking generation time and update total duration
    pub fn stop_generation(&mut self) {
        if let Some(start) = self.generation_start.take() {
            self.total_duration = start.elapsed();
        }
    }

    /// Record a forward pass
    pub fn record_forward_pass(&mut self) {
        self.forward_passes += 1;
    }

    /// Record SIMD operation usage
    pub fn record_simd_operation(&mut self) {
        self.simd_operations += 1;
    }

    /// Record scalar operation usage
    pub fn record_scalar_operation(&mut self) {
        self.scalar_operations += 1;
    }

    /// Add generated tokens to the count
    pub fn add_tokens(&mut self, count: u64) {
        self.total_tokens += count;
    }
    /// Set input token count
    pub fn set_input_tokens(&mut self, count: u64) {
        self.input_tokens = count;
    }

    /// Set prompt processing duration
    pub fn set_prompt_processing_duration(&mut self, duration: Duration) {
        self.prompt_processing_duration = duration;
    }

    /// Set token generation duration
    pub fn set_token_generation_duration(&mut self, duration: Duration) {
        self.token_generation_duration = duration;
    }

    /// Update peak memory usage
    pub fn update_peak_memory(&mut self, bytes: u64) {
        if bytes > self.peak_memory_bytes {
            self.peak_memory_bytes = bytes;
        }
    }

    /// Record cache hit
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Record cache miss
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    /// Calculate tokens per second for total generation
    pub fn tokens_per_second(&self) -> f64 {
        if self.total_duration.as_secs_f64() > 0.0 {
            self.total_tokens as f64 / self.total_duration.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Calculate input processing tokens per second
    pub fn input_tokens_per_second(&self) -> f64 {
        if self.prompt_processing_duration.as_secs_f64() > 0.0 {
            self.input_tokens as f64 / self.prompt_processing_duration.as_secs_f64()
        } else {
            0.0
        }
    }
    /// Calculate generation-only tokens per second (excluding input processing)
    pub fn generation_tokens_per_second(&self) -> f64 {
        let generation_tokens = self.total_tokens.saturating_sub(self.input_tokens);
        if self.token_generation_duration.as_secs_f64() > 0.0 && generation_tokens > 0 {
            generation_tokens as f64 / self.token_generation_duration.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Calculate SIMD utilization percentage
    pub fn simd_utilization(&self) -> f64 {
        let total_ops = self.simd_operations + self.scalar_operations;
        if total_ops > 0 {
            (self.simd_operations as f64 / total_ops as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate cache hit rate percentage
    pub fn cache_hit_rate(&self) -> f64 {
        let total_accesses = self.cache_hits + self.cache_misses;
        if total_accesses > 0 {
            (self.cache_hits as f64 / total_accesses as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get efficiency summary as formatted string
    pub fn efficiency_summary(&self) -> String {
        format!(
            "Tokens/sec: {:.2} | SIMD: {:.1}% | Cache: {:.1}% | Memory: {:.1}MB",
            self.tokens_per_second(),
            self.simd_utilization(),
            self.cache_hit_rate(),
            self.peak_memory_bytes as f64 / 1_048_576.0 // Convert to MB
        )
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
impl Default for GenerationStatistics {
    fn default() -> Self {
        Self::new()
    }
}
