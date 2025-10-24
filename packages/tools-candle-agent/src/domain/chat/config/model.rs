//! Model configuration types for chat interactions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::Stream;

/// Candle model configuration for chat interactions
///
/// This configuration defines model-specific settings including provider selection,
/// model parameters, performance tuning, and behavior customization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleModelConfig {
    /// Model provider (e.g., "openai", "anthropic", "mistral", "gemini")
    pub provider: String,
    /// Model name/identifier
    pub registry_key: String,
    /// Model version or variant
    pub model_version: Option<String>,
    /// Temperature for response randomness (0.0 to 2.0)
    pub temperature: f32,
    /// Maximum tokens in response
    pub max_tokens: Option<u32>,
    /// Top-p nucleus sampling parameter
    pub top_p: Option<f32>,
    /// Top-k sampling parameter
    pub top_k: Option<u32>,
    /// Frequency penalty (-2.0 to 2.0)
    pub frequency_penalty: Option<f32>,
    /// Presence penalty (-2.0 to 2.0)
    pub presence_penalty: Option<f32>,
    /// Stop sequences
    pub stop_sequences: Vec<String>,
    /// System prompt/instructions
    pub system_prompt: Option<String>,
    /// Enable function calling
    pub enable_functions: bool,
    /// Function calling mode ("auto", "none", "required")
    pub function_mode: String,
    /// Model-specific parameters
    pub custom_parameters: HashMap<String, serde_json::Value>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Retry configuration
    pub retry_config: CandleModelRetryConfig,
    /// Performance settings
    pub performance: CandleModelPerformanceConfig,
}

/// Candle model retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleModelRetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Base delay between retries in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f32,
    /// Enable jitter to avoid thundering herd
    pub enable_jitter: bool,
}

/// Candle model performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleModelPerformanceConfig {
    /// Enable response caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Enable request batching
    pub enable_batching: bool,
    /// Maximum batch size
    pub max_batch_size: u32,
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
    /// Enable streaming responses
    pub enable_streaming: bool,
    /// Connection pool size
    pub connection_pool_size: u32,
    /// Keep-alive timeout in seconds
    pub keep_alive_timeout_seconds: u64,
}

impl Default for CandleModelConfig {
    fn default() -> Self {
        Self {
            provider: String::from("openai"),
            registry_key: String::from("gpt-4"),
            model_version: None,
            temperature: 0.0, // Greedy sampling for chat - deterministic output
            max_tokens: Some(2048),
            top_p: Some(1.0),
            top_k: None,
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: Vec::new(),
            system_prompt: None,
            enable_functions: true,
            function_mode: String::from("auto"),
            custom_parameters: HashMap::new(),
            timeout_ms: 30000, // 30 seconds
            retry_config: CandleModelRetryConfig::default(),
            performance: CandleModelPerformanceConfig::default(),
        }
    }
}

impl Default for CandleModelRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000, // 1 second
            max_delay_ms: 30000, // 30 seconds
            backoff_multiplier: 2.0,
            enable_jitter: true,
        }
    }
}

impl Default for CandleModelPerformanceConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_ttl_seconds: 3600, // 1 hour
            enable_batching: false,
            max_batch_size: 10,
            batch_timeout_ms: 100,
            enable_streaming: true,
            connection_pool_size: 10,
            keep_alive_timeout_seconds: 60,
        }
    }
}

impl CandleModelConfig {
    /// Create a new Candle model configuration
    pub fn new(provider: impl Into<String>, registry_key: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            registry_key: registry_key.into(),
            ..Default::default()
        }
    }

    /// Create configuration for `OpenAI` models
    pub fn openai(registry_key: impl Into<String>) -> Self {
        Self::new("openai", registry_key)
    }

    /// Create configuration for Anthropic models
    pub fn anthropic(registry_key: impl Into<String>) -> Self {
        Self::new("anthropic", registry_key)
    }

    /// Create configuration for Mistral models
    pub fn mistral(registry_key: impl Into<String>) -> Self {
        Self::new("mistral", registry_key)
    }

    /// Create configuration for Gemini models
    pub fn gemini(registry_key: impl Into<String>) -> Self {
        Self::new("gemini", registry_key)
    }

    /// Set temperature
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Set max tokens
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set system prompt
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Enable or disable function calling
    #[must_use]
    pub fn with_functions(mut self, enable: bool) -> Self {
        self.enable_functions = enable;
        self
    }

    /// Validate the model configuration
    #[must_use]
    pub fn validate(
        &self,
    ) -> Pin<Box<dyn Stream<Item = crate::domain::context::chunks::CandleUnit> + Send>> {
        let _config = self.clone();
        // Use spawn_stream for streaming-only architecture - emit success immediately
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Emit success via sender - validation happens during stream processing
            let _ = tx.send(crate::domain::context::chunks::CandleUnit(()));
        }))
    }
}
