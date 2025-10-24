//! Model types and implementations for the extraction module

use serde::{Deserialize, Serialize};

/// Configuration for the extraction model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Maximum number of tokens to generate
    pub max_tokens: usize,
    /// Sampling temperature (0.0 to 2.0)
    pub temperature: f32,
    /// Top-p sampling parameter
    pub top_p: f32,
    /// Stop sequences for generation
    pub stop_sequences: Vec<String>,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            max_tokens: 1000,
            temperature: 0.2,
            top_p: 1.0,
            stop_sequences: vec!["\n".to_string()],
        }
    }
}

/// Represents an extraction request
#[derive(Debug, Clone)]
pub struct ExtractionRequest<T> {
    /// The text to extract from
    pub text: String,
    /// The target type to extract
    pub target_type: std::marker::PhantomData<T>,
    /// Optional system prompt
    pub system_prompt: Option<String>,
    /// Model configuration
    pub config: ExtractionConfig,
}

impl<T> ExtractionRequest<T> {
    /// Create a new extraction request
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            target_type: std::marker::PhantomData,
            system_prompt: None,
            config: ExtractionConfig::default(),
        }
    }

    /// Set the system prompt
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the extraction configuration
    #[must_use]
    pub fn with_config(mut self, config: ExtractionConfig) -> Self {
        self.config = config;
        self
    }
}

/// Represents the result of an extraction
#[derive(Debug, Clone)]
pub struct ExtractionResult<T> {
    /// The extracted data
    pub data: T,
    /// The raw response from the model
    pub raw_response: String,
    /// The number of tokens used
    pub tokens_used: usize,
}

impl<T> ExtractionResult<T> {
    /// Create a new extraction result
    pub fn new(data: T, raw_response: String, tokens_used: usize) -> Self {
        Self {
            data,
            raw_response,
            tokens_used,
        }
    }
}
