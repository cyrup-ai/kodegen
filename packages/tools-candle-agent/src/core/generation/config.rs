//! Sampling configuration and parameter management
//!
//! This module provides comprehensive configuration structures for text generation,
//! including temperature, top-k, top-p, and penalty settings. Includes validation
//! methods and preset configurations for common use cases.

use super::types::{DEFAULT_CONTEXT_LENGTH, SIMD_THRESHOLD};

/// Comprehensive sampling configuration for text generation
///
/// Encapsulates all parameters that control the generation process,
/// including temperature, nucleus sampling, penalties, and SIMD optimization.
#[derive(Debug, Clone)]
pub struct SamplingConfig {
    /// Temperature for scaling logits (0.0 = deterministic, higher = more random)
    pub temperature: f32,

    /// Top-k sampling: only consider top k tokens (None = disabled)
    pub top_k: Option<usize>,

    /// Top-p (nucleus) sampling: cumulative probability threshold (None = disabled)
    pub top_p: Option<f64>,

    /// Repetition penalty: penalize repeated tokens (1.0 = no penalty)
    pub repetition_penalty: f32,

    /// Frequency penalty: penalize frequent tokens (0.0 = no penalty)
    pub frequency_penalty: f32,

    /// Presence penalty: penalize tokens present in context (0.0 = no penalty)
    pub presence_penalty: f32,

    /// Number of recent tokens to consider for repetition penalty
    pub repetition_context_length: usize,

    /// Random seed for reproducible generation (None = non-deterministic)
    pub seed: Option<u64>,

    /// Whether to use SIMD acceleration when available
    pub use_simd: bool,

    /// Minimum sequence length before applying SIMD optimizations
    pub simd_threshold: usize,
}
impl SamplingConfig {
    /// Create a new SamplingConfig with specified temperature
    pub fn new(temperature: f32) -> Self {
        Self {
            temperature,
            top_k: None,
            top_p: None,
            repetition_penalty: 1.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            repetition_context_length: DEFAULT_CONTEXT_LENGTH,
            seed: None,
            use_simd: true,
            simd_threshold: SIMD_THRESHOLD,
        }
    }

    /// Builder method to set top-k sampling
    #[must_use]
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Builder method to set top-p sampling
    #[must_use]
    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Builder method to set repetition penalty
    #[must_use]
    pub fn with_repetition_penalty(mut self, penalty: f32) -> Self {
        self.repetition_penalty = penalty;
        self
    }

    /// Builder method to set frequency penalty
    #[must_use]
    pub fn with_frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = penalty;
        self
    }

    /// Builder method to set presence penalty
    #[must_use]
    pub fn with_presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = penalty;
        self
    }

    /// Builder method to set random seed
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Builder method to disable SIMD acceleration
    #[must_use]
    pub fn without_simd(mut self) -> Self {
        self.use_simd = false;
        self
    }
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.temperature < 0.0 {
            return Err("Temperature must be non-negative".to_string());
        }

        if let Some(top_k) = self.top_k
            && top_k == 0
        {
            return Err("Top-k must be greater than 0".to_string());
        }

        if let Some(top_p) = self.top_p
            && (top_p <= 0.0 || top_p > 1.0)
        {
            return Err("Top-p must be in (0, 1]".to_string());
        }

        if self.repetition_penalty < 0.0 {
            return Err("Repetition penalty must be non-negative".to_string());
        }

        Ok(())
    }

    /// Check if deterministic sampling is enabled (temperature == 0.0)
    pub fn is_deterministic(&self) -> bool {
        self.temperature == 0.0
    }

    /// Check if any penalty is enabled
    pub fn has_penalties(&self) -> bool {
        self.repetition_penalty != 1.0
            || self.frequency_penalty != 0.0
            || self.presence_penalty != 0.0
    }

    /// Check if SIMD should be used for given sequence length
    pub fn should_use_simd(&self, sequence_length: usize) -> bool {
        self.use_simd && sequence_length >= self.simd_threshold
    }
}

impl Default for SamplingConfig {
    /// Default configuration with deterministic greedy sampling
    fn default() -> Self {
        Self::new(0.0) // Greedy sampling for deterministic output
    }
}
/// Preset configuration for deterministic generation
pub fn deterministic_config() -> SamplingConfig {
    SamplingConfig::new(0.0).with_seed(42)
}

/// Preset configuration for balanced generation
pub fn balanced_config() -> SamplingConfig {
    SamplingConfig::new(0.8)
        .with_top_k(40)
        .with_top_p(0.9)
        .with_repetition_penalty(1.1)
}

/// Preset configuration for creative generation
pub fn creative_config() -> SamplingConfig {
    SamplingConfig::new(1.2)
        .with_top_p(0.95)
        .with_repetition_penalty(1.05)
        .with_presence_penalty(0.1)
}

/// Preset configuration for focused generation
pub fn focused_config() -> SamplingConfig {
    SamplingConfig::new(0.3)
        .with_top_k(10)
        .with_repetition_penalty(1.2)
        .with_frequency_penalty(0.2)
}
