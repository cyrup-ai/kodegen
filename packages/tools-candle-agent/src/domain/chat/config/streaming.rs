//! Streaming pattern implementations for configuration types

use super::model::{CandleModelConfig, CandleModelPerformanceConfig, CandleModelRetryConfig};
use super::persistence::{CandlePersistenceEvent, CandlePersistenceType};
use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Candle configuration update event for streaming operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleConfigUpdate {
    /// Update timestamp in nanoseconds since UNIX epoch
    pub timestamp_nanos: u64,
    /// Type of configuration update
    pub update_type: CandleConfigUpdateType,
    /// Section being updated (if applicable)
    pub section: Option<String>,
    /// Success status of the update
    pub success: bool,
    /// Optional description of the update
    pub description: Option<String>,
}

/// Candle type of configuration update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CandleConfigUpdateType {
    /// Configuration validation started
    ValidationStarted,
    /// Configuration validation completed
    ValidationCompleted,
    /// Configuration validator registered
    ValidatorRegistered,
    /// Auto-save check performed
    AutoSaveChecked,
    /// Auto-save executed
    AutoSaveExecuted,
    /// Configuration saved to file
    SavedToFile,
    /// Configuration loaded from file
    LoadedFromFile,
    /// Configuration section updated
    SectionUpdated,
    /// Persistence event triggered
    PersistenceTriggered,
}

/// Streaming wrapper for `CandleModelConfig` that implements `MessageChunk`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CandleModelConfigChunk {
    /// The actual config data
    pub config: CandleModelConfigData,
    /// Error message if this is an error chunk
    pub error_message: Option<String>,
}

/// Serializable version of `CandleModelConfig` using `String` instead of `String`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleModelConfigData {
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

impl MessageChunk for CandleModelConfigChunk {
    fn bad_chunk(error: String) -> Self {
        Self {
            config: CandleModelConfigData::default(),
            error_message: Some(error),
        }
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

impl From<CandleModelConfig> for CandleModelConfigData {
    fn from(config: CandleModelConfig) -> Self {
        Self {
            provider: config.provider.clone(),
            registry_key: config.registry_key.clone(),
            model_version: config.model_version.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            top_p: config.top_p,
            top_k: config.top_k,
            frequency_penalty: config.frequency_penalty,
            presence_penalty: config.presence_penalty,
            stop_sequences: config.stop_sequences,
            system_prompt: config.system_prompt,
            enable_functions: config.enable_functions,
            function_mode: config.function_mode,
            custom_parameters: config.custom_parameters,
            timeout_ms: config.timeout_ms,
            retry_config: config.retry_config,
            performance: config.performance,
        }
    }
}

impl Default for CandleModelConfigData {
    fn default() -> Self {
        Self {
            provider: String::new(),
            registry_key: String::new(),
            model_version: None,
            temperature: 0.0, // Greedy sampling for chat - deterministic output
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: Vec::new(),
            system_prompt: None,
            enable_functions: false,
            function_mode: "auto".to_string(),
            custom_parameters: HashMap::new(),
            timeout_ms: 30000,
            retry_config: CandleModelRetryConfig::default(),
            performance: CandleModelPerformanceConfig::default(),
        }
    }
}

impl MessageChunk for CandleConfigUpdate {
    fn bad_chunk(error: String) -> Self {
        Self {
            timestamp_nanos: 0,
            update_type: CandleConfigUpdateType::ValidationStarted,
            section: Some(format!("error: {error}")),
            success: false,
            description: Some(error),
        }
    }

    fn error(&self) -> Option<&str> {
        if self.success {
            None
        } else {
            self.description.as_deref()
        }
    }
}

impl Default for CandleConfigUpdate {
    fn default() -> Self {
        Self {
            timestamp_nanos: 0,
            update_type: CandleConfigUpdateType::ValidationStarted,
            section: None,
            success: true,
            description: None,
        }
    }
}

impl MessageChunk for CandlePersistenceEvent {
    fn bad_chunk(_error: String) -> Self {
        // Error parameter reserved for future use
        Self {
            timestamp_nanos: 0,
            previous_timestamp_nanos: 0,
            persistence_type: CandlePersistenceType::Manual,
            success: false,
        }
    }

    fn error(&self) -> Option<&str> {
        if self.success {
            None
        } else {
            Some("Persistence operation failed")
        }
    }
}

impl Default for CandlePersistenceEvent {
    fn default() -> Self {
        Self {
            timestamp_nanos: 0,
            previous_timestamp_nanos: 0,
            persistence_type: CandlePersistenceType::Manual,
            success: true,
        }
    }
}
