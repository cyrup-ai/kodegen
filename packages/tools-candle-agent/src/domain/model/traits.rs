//! Core traits for AI models and their capabilities

use serde::{Deserialize, Serialize};

use crate::domain::model::{CandleModelInfo, CandleUsage};

/// Core trait for all Candle AI models
///
/// This trait provides the foundation for all Candle AI models in the system.
/// It defines the basic functionality that all models must implement,
/// including model information and basic capabilities.
pub trait CandleModel: Send + Sync + std::fmt::Debug + 'static {
    /// Get the model's information
    fn info(&self) -> &'static CandleModelInfo;

    /// Get the model's name
    #[inline]
    fn name(&self) -> &'static str {
        self.info().name()
    }

    /// Get the model's provider name
    #[inline]
    fn provider(&self) -> &'static str {
        self.info().provider_str()
    }

    /// Get the model's maximum input tokens
    #[inline]
    fn max_input_tokens(&self) -> Option<u32> {
        self.info().max_input_tokens.map(std::num::NonZeroU32::get)
    }

    /// Get the model's maximum output tokens
    #[inline]
    fn max_output_tokens(&self) -> Option<u32> {
        self.info().max_output_tokens.map(std::num::NonZeroU32::get)
    }

    /// Check if the model supports vision
    #[inline]
    fn supports_vision(&self) -> bool {
        self.info().has_vision()
    }

    /// Check if the model supports function calling
    #[inline]
    fn supports_function_calling(&self) -> bool {
        self.info().has_function_calling()
    }

    /// Check if the model supports streaming
    #[inline]
    fn supports_streaming(&self) -> bool {
        self.info().has_streaming()
    }

    /// Check if the model requires `max_tokens` to be specified
    #[inline]
    fn requires_max_tokens(&self) -> bool {
        self.info().requires_max_tokens()
    }

    /// Get the model's short CLI identifier (e.g., "kimi-k2", "qwen-coder")
    #[inline]
    fn model_id(&self) -> &'static str {
        self.info().model_id()
    }

    /// Get the `HuggingFace` repository URL for automatic model downloads
    #[inline]
    fn hf_repo_url(&self) -> String {
        self.info().hf_repo_url()
    }

    /// Get the model's quantization format (e.g., "`Q4_0`", "`Q5_0`", "F16")
    #[inline]
    fn quantization(&self) -> &'static str {
        self.info().quantization()
    }

    /// Get path to a file in a `HuggingFace` repository
    ///
    /// Downloads the file if not cached, returns cached path if available.
    ///
    /// # Arguments
    /// * `repo_key` - Repository identifier (e.g., "org/model-name")
    /// * `filename` - File name within the repository (e.g., "config.json")
    ///
    /// # Errors
    /// Returns error if file download or access fails
    fn huggingface_file(
        &self,
        repo_key: &str,
        filename: &str,
    ) -> impl std::future::Future<
        Output = Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>>,
    > + Send
    where
        Self: Sized,
    {
        async move {
            use crate::domain::model::download_lock::acquire_download_lock;
            use hf_hub::Cache;
            use hf_hub::api::tokio::ApiBuilder;

            // CRITICAL: Acquire application-level lock BEFORE attempting download
            // This prevents race conditions when multiple workers spawn simultaneously
            let lock = acquire_download_lock(repo_key, filename).await;
            let _guard = lock.lock().await;

            // Check cache first (file might be ready if we waited for lock)
            let cache = Cache::from_env();
            let cache_repo = cache.model(repo_key.to_string());

            if let Some(cached_path) = cache_repo.get(filename) {
                // Verify file exists and is not empty or corrupted
                if let Ok(metadata) = std::fs::metadata(&cached_path)
                    && metadata.len() > 0
                {
                    log::info!("✅ Using cached file (available after lock wait): {filename}");
                    return Ok(cached_path);
                }
            }

            // We hold lock and file not cached - proceed with download
            log::info!("⬇️  Starting download: {filename} from {repo_key}");

            let mut builder = ApiBuilder::from_env();

            if let Ok(token) = std::env::var("HF_TOKEN") {
                builder = builder.with_token(Some(token));
            }

            let api = builder.build()?;
            let repo = api.model(repo_key.to_string());
            let path = repo.get(filename).await?;

            log::info!("✅ Download complete: {filename}");

            Ok(path)
            // Lock released here when _guard drops
        }
    }
}

/// A message in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ChatMessage {
    /// A message from the system (sets the behavior of the assistant)
    System {
        /// The contents of the system message
        content: String,
        /// An optional name for the participant
        name: Option<String>,
    },

    /// A message from a human user
    User {
        /// The contents of the user message
        content: String,
        /// An optional name for the participant
        name: Option<String>,
        /// Optional list of image URLs or base64-encoded images
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<String>>,
    },

    /// A message from the assistant
    Assistant {
        /// The contents of the assistant's message
        content: Option<String>,
        /// An optional name for the participant
        name: Option<String>,
        /// Function calls made by the assistant
        #[serde(skip_serializing_if = "Option::is_none")]
        function_calls: Option<Vec<FunctionCall>>,
    },

    /// A function call result
    Function {
        /// The name of the function that was called
        name: String,
        /// The output of the function call
        content: String,
    },
}

/// A function call made by the model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionCall {
    /// The name of the function to call
    pub name: String,
    /// The arguments to pass to the function (JSON-encoded string)
    pub arguments: String,
}

/// A function definition that can be called by the model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionDefinition {
    /// The name of the function to be called
    pub name: String,
    /// A description of what the function does
    pub description: Option<String>,
    /// The parameters the function accepts, described as a JSON Schema object
    pub parameters: serde_json::Value,
}

/// Parameters for text generation
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GenerationParams {
    /// The maximum number of tokens to generate
    pub max_tokens: Option<u32>,

    /// Controls randomness: lower means more deterministic
    pub temperature: Option<f32>,

    /// Nucleus sampling: limits the next token selection to a subset of the vocabulary
    pub top_p: Option<f32>,

    /// Limits the number of highest probability vocabulary tokens to consider
    pub top_k: Option<u32>,

    /// Penalty for repeating tokens in the generation
    pub frequency_penalty: Option<f32>,

    /// Penalty for repeating tokens that appear in the prompt
    pub presence_penalty: Option<f32>,

    /// Stop sequences where the API will stop generating further tokens
    pub stop_sequences: Option<Vec<String>>,

    /// Whether to stream the response
    pub stream: bool,
}

/// A chunk of generated text
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextChunk {
    /// The generated text
    pub text: String,
    /// The token IDs of the generated text
    pub token_ids: Vec<u32>,
    /// Whether this is the last chunk
    pub is_complete: bool,
    /// The reason generation stopped (if complete)
    pub finish_reason: Option<String>,
    /// Token usage for this chunk (if available)
    pub usage: Option<CandleUsage>,
}

/// Request for text generation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextGenerationRequest {
    /// The input prompt
    pub prompt: String,
    /// Generation parameters
    pub params: GenerationParams,
}

/// Request for chat completion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionRequest {
    /// The conversation messages
    pub messages: Vec<ChatMessage>,
    /// Generation parameters
    pub params: GenerationParams,
    /// Optional function definitions
    pub functions: Option<Vec<FunctionDefinition>>,
}

/// Request for embedding generation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingRequest {
    /// The text(s) to embed
    pub texts: Vec<String>,
}

/// An embedding result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Embedding {
    /// The embedding vector
    pub vector: Vec<f32>,
    /// The original text that was embedded
    pub text: String,
    /// Token usage for this embedding (if available)
    pub usage: Option<CandleUsage>,
}

/// Fine-tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FineTuningConfig {
    /// Learning rate for training
    pub learning_rate: Option<f32>,
    /// Number of training epochs
    pub epochs: Option<u32>,
    /// Batch size for training
    pub batch_size: Option<u32>,
    /// Validation split ratio
    pub validation_split: Option<f32>,
}

/// Trait for models that can generate embeddings
pub trait EmbeddingCapable: CandleModel {
    /// Get the dimensionality of the embeddings
    fn embedding_dimensions(&self) -> usize;

    /// Check if the model supports embeddings
    fn supports_embeddings(&self) -> bool {
        true
    }

    /// Get the maximum text length for embedding
    fn max_embedding_text_length(&self) -> Option<usize> {
        self.max_input_tokens().map(|n| n as usize)
    }

    /// Get the maximum batch size for embedding
    fn max_embedding_batch_size(&self) -> Option<usize> {
        Some(100) // Default reasonable batch size
    }

    /// Get the expected embedding range (min, max)
    fn embedding_range(&self) -> Option<(f32, f32)> {
        Some((-1.0, 1.0)) // Default normalized range
    }
}

/// Trait for models that can be fine-tuned
pub trait FineTunable: CandleModel {
    /// Check if the model supports fine-tuning
    fn supports_fine_tuning(&self) -> bool {
        true
    }

    /// Get the supported fine-tuning data formats
    fn supported_data_formats(&self) -> Vec<&'static str> {
        vec!["json", "jsonl", "csv"]
    }

    /// Get the minimum dataset size required for fine-tuning
    fn min_dataset_size(&self) -> Option<usize> {
        Some(100) // Default minimum
    }

    /// Get the maximum dataset size supported for fine-tuning
    fn max_dataset_size(&self) -> Option<usize> {
        Some(100_000) // Default maximum
    }

    /// Get the default fine-tuning configuration
    fn default_fine_tuning_config(&self) -> FineTuningConfig {
        FineTuningConfig {
            learning_rate: Some(0.0001),
            epochs: Some(3),
            batch_size: Some(32),
            validation_split: Some(0.1),
        }
    }

    /// Check if the model supports saving/loading fine-tuned versions
    fn supports_model_persistence(&self) -> bool {
        true
    }
}
