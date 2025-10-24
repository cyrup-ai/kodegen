//! Embedding Configuration Types and Traits
//!
//! This module provides core configuration types for working with embedding models.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::memory::utils::error::Result;

/// Configuration for embedding models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingConfig {
    /// Model identifier (e.g., "text-embedding-ada-002")
    pub model: Option<String>,

    /// Embedding dimensions (if configurable)
    pub dimensions: Option<usize>,

    /// Whether to normalize embeddings to unit length
    #[serde(default = "default_normalize")]
    pub normalize: bool,

    /// Batch size for processing multiple texts
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Whether to truncate input text if it's too long
    #[serde(default = "default_truncate")]
    pub truncate: bool,

    /// Additional provider-specific parameters
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_params: HashMap<String, Value>,

    /// User identifier for tracking/rate limiting (optional)
    pub user: Option<String>,

    /// Encoding format (e.g., "float", "base64")
    pub encoding_format: Option<String>,
}
fn default_normalize() -> bool {
    true
}

fn default_batch_size() -> usize {
    32
}

fn default_truncate() -> bool {
    true
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: None,
            dimensions: Some(1024),
            normalize: default_normalize(),
            batch_size: default_batch_size(),
            truncate: default_truncate(),
            additional_params: HashMap::new(),
            user: None,
            encoding_format: None,
        }
    }
}

impl EmbeddingConfig {
    /// Create a new embedding configuration with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model identifier
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the embedding dimensions
    #[must_use]
    pub fn with_dimensions(mut self, dimensions: usize) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    /// Set dimensions with validation against model capabilities
    ///
    /// This method validates that the requested dimension is supported by the specified model
    /// before setting it, providing immediate feedback rather than waiting for factory creation.
    ///
    /// # Arguments
    /// * `dimensions` - Requested embedding dimension size
    /// * `registry_key` - Model name to validate against
    ///
    /// # Returns
    /// Result containing the updated config or validation error
    ///
    /// # Errors
    /// Returns `Config` error if the dimension is not supported by the model
    ///
    /// # Example
    /// ```rust,no_run
    /// use crate::domain::embedding::config::EmbeddingConfig;
    ///
    /// let config = EmbeddingConfig::new()
    ///     .with_model("dunzhang/stella_en_1.5B_v5")
    ///     .with_validated_dimension(1024, "dunzhang/stella_en_1.5B_v5")?; // Valid: Stella supports 1024D
    ///
    /// let invalid_config = EmbeddingConfig::new()
    ///     .with_model("bert")
    ///     .with_validated_dimension(512, "bert"); // Error: BERT only supports 384D
    /// ```
    pub fn with_validated_dimension(
        mut self,
        dimensions: usize,
        registry_key: &str,
    ) -> crate::memory::utils::error::Result<Self> {
        Self::validate_dimension_for_model(dimensions, registry_key)?;
        self.dimensions = Some(dimensions);
        Ok(self)
    }

    /// Validate dimension is supported by the specific model
    ///
    /// Internal validation method that checks if the requested dimension is supported
    /// by the specified model using the same logic as the factory validation.
    fn validate_dimension_for_model(
        dimension: usize,
        registry_key: &str,
    ) -> crate::memory::utils::error::Result<()> {
        use crate::memory::utils::error::Error as MemoryError;

        let normalized_name = Self::normalize_registry_key(registry_key);

        match normalized_name {
            "bert" | "sentence-transformers" => {
                if dimension != 384 {
                    return Err(MemoryError::Config(format!(
                        "BERT (sentence-transformers/all-MiniLM-L6-v2) only supports 384 dimensions. Requested: {dimension}. \
                         BERT uses a fixed architecture that produces exactly 384-dimensional embeddings."
                    )));
                }
            }
            "stella" => match dimension {
                256 | 768 | 1024 | 2048 | 4096 | 6144 | 8192 => {}
                _ => {
                    return Err(MemoryError::Config(format!(
                        "Stella natively supports: 256, 768, 1024, 2048, 4096, 6144, 8192 dimensions. Requested: {dimension}. \
                         These are the actual learned projection dimensions available in the MRL framework."
                    )));
                }
            },
            "gte-qwen" | "gte-qwen2" => {
                if dimension != 1536 {
                    return Err(MemoryError::Config(format!(
                        "GTE-Qwen2-1.5B-instruct only supports 1536 dimensions. Requested: {dimension}. \
                         This model has a fixed architecture optimized for 1536-dimensional embeddings."
                    )));
                }
            }
            "jina-bert" | "jina" => {
                if dimension != 768 {
                    return Err(MemoryError::Config(format!(
                        "Jina-BERT only supports 768 dimensions. Requested: {dimension}. \
                         This model uses a fixed BERT-based architecture with 768-dimensional output."
                    )));
                }
            }
            "nvembed" | "nv-embed-v2" | "nvidia/nv-embed-v2" => {
                if dimension != 4096 {
                    return Err(MemoryError::Config(format!(
                        "NVEmbed-v2 only supports 4096 dimensions. Requested: {dimension}. \
                         This model uses a large transformer architecture optimized for 4096-dimensional embeddings."
                    )));
                }
            }
            "clip-vision" | "clip" => {
                if dimension != 512 && dimension != 768 {
                    return Err(MemoryError::Config(format!(
                        "CLIP Vision supports 512 dimensions (ViT-Base-Patch32) or 768 dimensions (ViT-Large-Patch14). Requested: {dimension}. \
                         These correspond to the two available CLIP Vision architectures."
                    )));
                }
            }
            _ => {
                return Err(MemoryError::Config(format!(
                    "Unknown model '{registry_key}' for dimension validation. Supported models: bert, stella, gte-qwen, jina-bert, nvembed, clip-vision"
                )));
            }
        }

        Ok(())
    }

    /// Normalize model name for consistent matching (internal)
    fn normalize_registry_key(registry_key: &str) -> &'static str {
        let lower = registry_key.to_lowercase();
        match lower.as_str() {
            // BERT variants
            "bert" | "sentence-transformers" | "all-minilm-l6-v2" => "bert",

            // Stella variants
            "stella" | "stella_en_1.5b_v5" | "dunzhang/stella_en_1.5b_v5" => "stella",

            // GTE-Qwen variants
            "gte-qwen"
            | "gte-qwen2"
            | "gte-qwen2-1.5b-instruct"
            | "alibaba-nlp/gte-qwen2-1.5b-instruct" => "gte-qwen",

            // Jina-BERT variants
            "jina-bert"
            | "jina"
            | "jina-embeddings-v2-base-en"
            | "jinaai/jina-embeddings-v2-base-en" => "jina-bert",

            // NVEmbed variants
            "nvembed" | "nv-embed-v2" | "nvidia/nv-embed-v2" => "nvembed",

            // CLIP Vision variants
            "clip-vision"
            | "clip"
            | "clip-vit-base-patch32"
            | "clip-vit-large-patch14"
            | "openai/clip-vit-base-patch32"
            | "openai/clip-vit-large-patch14-336" => "clip-vision",

            // Default fallback - return generic "unknown" for unrecognized models
            _ => "unknown",
        }
    }

    /// Enable or disable normalization
    #[must_use]
    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }
    /// Set the batch size
    #[must_use]
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Enable or disable truncation
    #[must_use]
    pub fn with_truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Add an additional parameter
    #[must_use]
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.additional_params.insert(key.into(), value.into());
        self
    }

    /// Set the user identifier
    #[must_use]
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Set the encoding format
    #[must_use]
    pub fn with_encoding_format(mut self, format: impl Into<String>) -> Self {
        self.encoding_format = Some(format.into());
        self
    }

    /// Validate this configuration against embedding model constraints
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid for the embedding model
    pub fn validate(&self) -> Result<()> {
        // Validate dimensions if specified
        self.validate_dimensions()?;

        // Validate model name exists
        if let Some(ref model) = self.model
            && model.trim().is_empty()
        {
            return Err(crate::memory::utils::error::Error::Config(
                "Model name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate that the dimensions are supported by the configured model
    ///
    /// # Errors
    ///
    /// Returns error if dimensions are not supported by the configured model
    pub fn validate_dimensions(&self) -> Result<()> {
        if let Some(dims) = self.dimensions {
            let registry_key = self.model.as_deref().unwrap_or("stella");
            Self::validate_dimension_for_model(dims, registry_key)
        } else {
            Ok(())
        }
    }

    /// Check if a specific dimension is supported by the configured model
    #[must_use]
    pub fn is_dimension_supported(&self, dimension: usize) -> bool {
        let registry_key = self.model.as_deref().unwrap_or("stella");
        Self::validate_dimension_for_model(dimension, registry_key).is_ok()
    }

    /// Get all supported dimensions for the configured model
    #[must_use]
    pub fn get_supported_dimensions(&self) -> Vec<usize> {
        let registry_key = self.model.as_deref().unwrap_or("stella");
        let normalized_name = Self::normalize_registry_key(registry_key);

        match normalized_name {
            "bert" => vec![384],
            "stella" => vec![256, 768, 1024, 2048, 4096, 6144, 8192],
            "gte-qwen" => vec![1536],
            "jina-bert" => vec![768],
            "nvembed" => vec![4096],
            "clip-vision" => vec![512, 768],
            _ => vec![],
        }
    }
}

/// Trait for types that can be converted to an `EmbeddingConfig`
pub trait IntoEmbeddingConfig {
    /// Convert to an `EmbeddingConfig`
    fn into_embedding_config(self) -> EmbeddingConfig;
}

impl IntoEmbeddingConfig for EmbeddingConfig {
    fn into_embedding_config(self) -> EmbeddingConfig {
        self
    }
}

impl<T: AsRef<str> + Into<String>> IntoEmbeddingConfig for T {
    fn into_embedding_config(self) -> EmbeddingConfig {
        EmbeddingConfig::default().with_model(self.into())
    }
}
