//! Unified Model Configuration System
//!
//! Defines model-agnostic configuration that ANY model can provide to the core engine.
//! This separates model configuration from inference logic, allowing hundreds of models
//! to share the same inference engine with only configuration differences.

use std::path::PathBuf;

use candle_core::DType;
use candle_transformers::models::llama::Config as LlamaConfig;
#[cfg(test)]
use candle_transformers::models::llama::LlamaEosToks;
use candle_transformers::models::quantized_mixformer::Config as MixFormerConfig;
use serde::{Deserialize, Serialize};

/// Model-agnostic configuration that ANY model can provide to the core engine
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Path to model weights file (model.safetensors)
    pub model_path: PathBuf,
    /// Path to tokenizer file (tokenizer.json)
    pub tokenizer_path: PathBuf,
    /// Model architecture type and specific configuration
    pub architecture: ModelArchitecture,
    /// Model vocabulary size
    pub vocab_size: usize,
    /// Maximum context length the model supports
    pub context_length: usize,
    /// Special token IDs for this model
    pub special_tokens: SpecialTokenIds,
    /// Data type for model weights
    pub dtype: DType,
    /// Human-readable model name
    pub registry_key: String,
    /// Model provider identifier
    pub provider_name: String,
}

impl ModelConfig {
    /// Create a new model configuration
    pub fn new(
        model_path: impl Into<PathBuf>,
        tokenizer_path: impl Into<PathBuf>,
        architecture: ModelArchitecture,
        registry_key: impl Into<String>,
        provider_name: impl Into<String>,
    ) -> Self {
        let arch_defaults = architecture.get_defaults();

        Self {
            model_path: model_path.into(),
            tokenizer_path: tokenizer_path.into(),
            architecture,
            vocab_size: arch_defaults.vocab_size,
            context_length: arch_defaults.context_length,
            special_tokens: arch_defaults.special_tokens,
            dtype: DType::F16, // Default to F16 for efficiency
            registry_key: registry_key.into(),
            provider_name: provider_name.into(),
        }
    }

    /// Set custom vocabulary size
    pub fn with_vocab_size(mut self, vocab_size: usize) -> Self {
        self.vocab_size = vocab_size;
        self
    }

    /// Set custom context length
    pub fn with_context_length(mut self, context_length: usize) -> Self {
        self.context_length = context_length;
        self
    }

    /// Set custom special tokens
    pub fn with_special_tokens(mut self, special_tokens: SpecialTokenIds) -> Self {
        self.special_tokens = special_tokens;
        self
    }

    /// Set custom data type
    pub fn with_dtype(mut self, dtype: DType) -> Self {
        self.dtype = dtype;
        self
    }

    /// Validate the model configuration
    pub fn validate(&self) -> Result<(), ModelConfigError> {
        if self.registry_key.is_empty() {
            return Err(ModelConfigError::InvalidModelName(
                "Model name cannot be empty".into(),
            ));
        }

        if self.provider_name.is_empty() {
            return Err(ModelConfigError::InvalidProviderName(
                "Provider name cannot be empty".into(),
            ));
        }

        if self.vocab_size == 0 {
            return Err(ModelConfigError::InvalidVocabSize(
                "Vocabulary size must be greater than 0".into(),
            ));
        }

        if self.context_length == 0 {
            return Err(ModelConfigError::InvalidContextLength(
                "Context length must be greater than 0".into(),
            ));
        }

        if !self.model_path.exists() {
            return Err(ModelConfigError::ModelFileNotFound(format!(
                "Model file not found: {}",
                self.model_path.display()
            )));
        }

        if !self.tokenizer_path.exists() {
            return Err(ModelConfigError::TokenizerFileNotFound(format!(
                "Tokenizer file not found: {}",
                self.tokenizer_path.display()
            )));
        }

        Ok(())
    }
}

/// Model architecture types with their specific configurations
#[derive(Debug, Clone)]
pub enum ModelArchitecture {
    /// Llama family models (Llama 2, Code Llama, etc.)
    Llama(LlamaConfig),
    /// Mistral family models
    Mistral(MistralConfig),
    /// Gemma family models
    Gemma(GemmaConfig),
    /// Phi family models (MixFormer architecture)
    Phi(MixFormerConfig),
    /// Future architectures will be added here
    Custom {
        name: String,
        config: serde_json::Value,
    },
}

impl ModelArchitecture {
    /// Get default configuration values for this architecture
    pub fn get_defaults(&self) -> ArchitectureDefaults {
        match self {
            ModelArchitecture::Llama(_) => ArchitectureDefaults {
                vocab_size: 32000,
                context_length: 2048,
                special_tokens: SpecialTokenIds {
                    bos_token_id: Some(1),
                    eos_token_id: Some(2),
                    pad_token_id: Some(0),
                    unk_token_id: Some(0),
                },
            },
            ModelArchitecture::Mistral(_) => ArchitectureDefaults {
                vocab_size: 32000,
                context_length: 4096,
                special_tokens: SpecialTokenIds {
                    bos_token_id: Some(1),
                    eos_token_id: Some(2),
                    pad_token_id: Some(0),
                    unk_token_id: Some(0),
                },
            },
            ModelArchitecture::Gemma(_) => ArchitectureDefaults {
                vocab_size: 256000,
                context_length: 8192,
                special_tokens: SpecialTokenIds {
                    bos_token_id: Some(2),
                    eos_token_id: Some(1),
                    pad_token_id: Some(0),
                    unk_token_id: Some(3),
                },
            },
            ModelArchitecture::Phi(_config) => ArchitectureDefaults {
                vocab_size: 51200,     // Phi-4 vocab size (config.vocab_size is private)
                context_length: 16384, // Phi-4 context length
                special_tokens: SpecialTokenIds {
                    bos_token_id: None,
                    eos_token_id: Some(2),
                    pad_token_id: Some(0),
                    unk_token_id: Some(0),
                },
            },
            ModelArchitecture::Custom { .. } => ArchitectureDefaults {
                vocab_size: 32000,
                context_length: 2048,
                special_tokens: SpecialTokenIds::default(),
            },
        }
    }

    /// Get architecture name as string
    pub fn name(&self) -> &str {
        match self {
            ModelArchitecture::Llama(_) => "llama",
            ModelArchitecture::Mistral(_) => "mistral",
            ModelArchitecture::Gemma(_) => "gemma",
            ModelArchitecture::Phi(_) => "phi",
            ModelArchitecture::Custom { name, .. } => name,
        }
    }
}

/// Configuration for Mistral models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralConfig {
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub num_key_value_heads: usize,
    pub sliding_window: Option<usize>,
}

/// Configuration for Gemma models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemmaConfig {
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub num_key_value_heads: usize,
    pub head_dim: usize,
}

/// Special token IDs for model-specific tokens
#[derive(Debug, Clone, Copy)]
pub struct SpecialTokenIds {
    /// Beginning of sequence token ID
    pub bos_token_id: Option<u32>,
    /// End of sequence token ID
    pub eos_token_id: Option<u32>,
    /// Padding token ID
    pub pad_token_id: Option<u32>,
    /// Unknown token ID
    pub unk_token_id: Option<u32>,
}

impl Default for SpecialTokenIds {
    fn default() -> Self {
        Self {
            bos_token_id: Some(1),
            eos_token_id: Some(2),
            pad_token_id: Some(0),
            unk_token_id: Some(0),
        }
    }
}

impl SpecialTokenIds {
    /// Check if a token ID is a special token
    pub fn is_special_token(&self, token_id: u32) -> bool {
        token_id == self.bos_token_id.unwrap_or(u32::MAX)
            || token_id == self.eos_token_id.unwrap_or(u32::MAX)
            || token_id == self.pad_token_id.unwrap_or(u32::MAX)
            || token_id == self.unk_token_id.unwrap_or(u32::MAX)
    }

    /// Check if a token is an end-of-sequence token
    pub fn is_eos_token(&self, token_id: u32) -> bool {
        Some(token_id) == self.eos_token_id
    }

    /// Get the name of a special token
    pub fn token_name(&self, token_id: u32) -> Option<&'static str> {
        if Some(token_id) == self.bos_token_id {
            Some("<BOS>")
        } else if Some(token_id) == self.eos_token_id {
            Some("<EOS>")
        } else if Some(token_id) == self.pad_token_id {
            Some("<PAD>")
        } else if Some(token_id) == self.unk_token_id {
            Some("<UNK>")
        } else {
            None
        }
    }
}

/// Default configuration values for an architecture
#[derive(Debug, Clone)]
pub struct ArchitectureDefaults {
    pub vocab_size: usize,
    pub context_length: usize,
    pub special_tokens: SpecialTokenIds,
}

/// Errors that can occur during model configuration
#[derive(Debug, Clone, thiserror::Error)]
pub enum ModelConfigError {
    #[error("Invalid model name: {0}")]
    InvalidModelName(String),

    #[error("Invalid provider name: {0}")]
    InvalidProviderName(String),

    #[error("Invalid vocabulary size: {0}")]
    InvalidVocabSize(String),

    #[error("Invalid context length: {0}")]
    InvalidContextLength(String),

    #[error("Model file not found: {0}")]
    ModelFileNotFound(String),

    #[error("Tokenizer file not found: {0}")]
    TokenizerFileNotFound(String),

    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
}

/// Result type for model configuration operations
pub type ModelConfigResult<T> = Result<T, ModelConfigError>;

#[cfg(test)]
mod tests {
    use std::fs::File;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_model_config_creation() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let model_path = temp_dir.path().join("model.safetensors");
        let tokenizer_path = temp_dir.path().join("tokenizer.json");

        // Create dummy files
        File::create(&model_path)?;
        File::create(&tokenizer_path)?;

        let llama_config = LlamaConfig {
            vocab_size: 32000,
            hidden_size: 4096,
            intermediate_size: 11008,
            num_hidden_layers: 32,
            num_attention_heads: 32,
            num_key_value_heads: 32,
            max_position_embeddings: 2048,
            rms_norm_eps: 1e-6,
            rope_theta: 10000.0,
            use_flash_attn: false,
            bos_token_id: Some(1),
            eos_token_id: Some(LlamaEosToks::Single(2)),
            rope_scaling: None,
            tie_word_embeddings: false,
        };

        let config = ModelConfig::new(
            model_path,
            tokenizer_path,
            ModelArchitecture::Llama(llama_config),
            "test-llama",
            "test-provider",
        );

        assert_eq!(config.registry_key, "test-llama");
        assert_eq!(config.provider_name, "test-provider");
        assert_eq!(config.vocab_size, 32000);

        // Validation should pass with existing files
        config.validate()?;
        Ok(())
    }

    #[test]
    fn test_special_token_identification() {
        let tokens = SpecialTokenIds::default();

        assert!(tokens.is_special_token(1)); // BOS
        assert!(tokens.is_special_token(2)); // EOS
        assert!(tokens.is_special_token(0)); // PAD
        assert!(!tokens.is_special_token(100)); // Regular token

        assert!(tokens.is_eos_token(2));
        assert!(!tokens.is_eos_token(1));

        assert_eq!(tokens.token_name(1), Some("<BOS>"));
        assert_eq!(tokens.token_name(2), Some("<EOS>"));
        assert_eq!(tokens.token_name(100), None);
    }

    #[test]
    fn test_architecture_defaults() {
        let llama_arch = ModelArchitecture::Llama(LlamaConfig {
            vocab_size: 32000,
            hidden_size: 4096,
            intermediate_size: 11008,
            num_hidden_layers: 32,
            num_attention_heads: 32,
            num_key_value_heads: 32,
            max_position_embeddings: 2048,
            rms_norm_eps: 1e-6,
            rope_theta: 10000.0,
            use_flash_attn: false,
            bos_token_id: Some(1),
            eos_token_id: Some(LlamaEosToks::Single(2)),
            rope_scaling: None,
            tie_word_embeddings: false,
        });

        let defaults = llama_arch.get_defaults();
        assert_eq!(defaults.vocab_size, 32000);
        assert_eq!(defaults.context_length, 2048);
        assert_eq!(defaults.special_tokens.eos_token_id, Some(2));

        assert_eq!(llama_arch.name(), "llama");
    }
}
