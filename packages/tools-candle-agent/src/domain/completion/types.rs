//! Shared types and constants for completion functionality
//!
//! Contains common types, parameters, and constants used across completion modules.

use std::num::NonZeroU64;
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::model::{
    CandleValidationError as ValidationError, CandleValidationResult as ValidationResult,
};
use cyrup_sugars::ZeroOneOrMany;

/// Temperature range for generation (0.0 to 2.0)
pub const TEMPERATURE_RANGE: RangeInclusive<f64> = 0.0..=2.0;

/// Maximum tokens for a single completion
pub const MAX_TOKENS: u64 = 8192;

/// Maximum chunk size for streaming
pub const MAX_CHUNK_SIZE: usize = 4096;

/// Candle parameters for completion generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CandleCompletionParams {
    /// Sampling temperature (0.0 to 2.0)
    pub temperature: f64,
    /// Maximum number of tokens to generate
    pub max_tokens: Option<NonZeroU64>,
    /// Number of completions to generate
    pub n: std::num::NonZeroU8,
    /// Whether to stream the response
    pub stream: bool,
    /// Tools available to the model for function calling
    pub tools: Option<ZeroOneOrMany<ToolInfo>>,
    /// Additional provider-specific parameters
    pub additional_params: Option<Value>,
}

impl Default for CandleCompletionParams {
    fn default() -> Self {
        Self {
            temperature: 0.0, // Greedy sampling for deterministic output
            max_tokens: None,
            n: match std::num::NonZeroU8::new(1) {
                Some(n) => n,
                None => std::num::NonZeroU8::MIN, // Use minimum valid value as fallback
            },
            stream: false,
            tools: None,
            additional_params: None,
        }
    }
}

impl CandleCompletionParams {
    /// Create new completion parameters
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the temperature
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if temperature is outside valid range
    pub fn with_temperature(mut self, temperature: f64) -> ValidationResult<Self> {
        if !TEMPERATURE_RANGE.contains(&temperature) {
            return Err(ValidationError::InvalidRange {
                field: "temperature".into(),
                value: temperature.to_string(),
                expected: format!(
                    "between {:.1} and {:.1}",
                    TEMPERATURE_RANGE.start(),
                    TEMPERATURE_RANGE.end()
                ),
            });
        }
        self.temperature = temperature;
        Ok(self)
    }

    /// Set the maximum number of tokens
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: Option<NonZeroU64>) -> Self {
        self.max_tokens = max_tokens.and_then(|t| NonZeroU64::new(t.get().min(MAX_TOKENS)));
        self
    }

    /// Set additional provider-specific parameters
    #[must_use]
    pub fn with_additional_params(mut self, additional_params: Option<Value>) -> Self {
        self.additional_params = additional_params;
        self
    }
}

// Re-export existing tool definitions from the tool module
pub use rmcp::model::Tool as ToolInfo;

/// Model-specific parameters for completion requests
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CandleModelParams {
    /// `RoPE` theta parameter for positional encoding
    pub rope_theta: f32,
    /// `RoPE` frequency base for positional encoding  
    pub rope_freq_base: f32,
    /// Context window size
    pub context_length: u32,
    /// Vocabulary size
    pub vocab_size: u32,
}

impl Default for CandleModelParams {
    #[inline]
    fn default() -> Self {
        Self {
            rope_theta: 10000.0,
            rope_freq_base: 1.0,
            context_length: 2048,
            vocab_size: 32000,
        }
    }
}
