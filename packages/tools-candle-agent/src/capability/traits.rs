//! Capability Traits
//!
//! Defines what models CAN DO based on the glossary capabilities:
//! - TextToText
//! - TextToImage  
//! - TextEmbedding
//! - ImageEmbedding
//! - TextToSpeech
//! - SpeechToText
//! - Vision

use std::pin::Pin;

use crate::domain::completion::CandleCompletionChunk;
use crate::domain::completion::types::CandleCompletionParams;
use crate::domain::context::chunks::CandleStringChunk;
use crate::domain::image_generation::{ImageGenerationChunk, ImageGenerationConfig};
use crate::domain::model::traits::{CandleModel, GenerationParams};
use crate::domain::prompt::CandlePrompt;
use candle_core::Device;
use tokio_stream::Stream;

/// Type alias for single embedding future
pub type EmbeddingFuture<'a> = Pin<
    Box<
        dyn std::future::Future<
                Output = std::result::Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>>,
            > + Send
            + 'a,
    >,
>;

/// Type alias for batch embedding future
pub type BatchEmbeddingFuture<'a> = Pin<
    Box<
        dyn std::future::Future<
                Output = std::result::Result<
                    Vec<Vec<f32>>,
                    Box<dyn std::error::Error + Send + Sync>,
                >,
            > + Send
            + 'a,
    >,
>;

/// Trait for models capable of text-to-text generation
pub trait TextToTextCapable: CandleModel {
    /// Generate completion from prompt - the actual work method
    fn prompt(
        &self,
        prompt: CandlePrompt,
        params: &CandleCompletionParams,
    ) -> Pin<Box<dyn Stream<Item = CandleCompletionChunk> + Send>>;

    /// Get the default generation parameters for this model
    fn default_generation_params(&self) -> GenerationParams {
        GenerationParams::default()
    }

    /// Get the model's maximum context window length
    fn max_context_length(&self) -> Option<usize> {
        self.max_input_tokens().map(|n| n as usize)
    }
}

/// Trait for models capable of text embedding
pub trait TextEmbeddingCapable: CandleModel {
    /// Generate embedding for a single text
    fn embed(&self, text: &str, task: Option<String>) -> EmbeddingFuture<'_>;

    /// Generate embeddings for multiple texts in batch
    fn batch_embed(&self, texts: &[String], task: Option<String>) -> BatchEmbeddingFuture<'_>;

    /// Get the dimensionality of embeddings produced by this model
    fn embedding_dimension(&self) -> usize;

    /// Get the supported embedding dimensions for this model
    fn supported_dimensions(&self) -> Vec<usize> {
        vec![self.embedding_dimension()]
    }

    /// Get the recommended batch size for optimal performance
    fn recommended_batch_size(&self) -> usize {
        16
    }

    /// Get the maximum batch size supported
    fn max_batch_size(&self) -> usize {
        128
    }

    /// Check if this model supports a specific dimension size
    fn supports_dimension(&self, dim: usize) -> bool {
        self.supported_dimensions().contains(&dim)
    }

    /// Validate a dimension request
    fn validate_dimension_request(
        &self,
        dim: usize,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.supports_dimension(dim) {
            return Ok(());
        }
        let supported = self.supported_dimensions();
        let supported_str = supported
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Err(format!(
            "Model '{}' does not support {}D embeddings. Supported dimensions: [{}]",
            self.name(),
            dim,
            supported_str
        )
        .into())
    }

    /// Validate input text before processing
    fn validate_input(
        &self,
        text: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if text.is_empty() {
            return Err("Input text cannot be empty".into());
        }
        if text.len() > 1_000_000 {
            return Err("Input text exceeds maximum length (1M characters)".into());
        }
        Ok(())
    }

    /// Validate a batch of texts
    fn validate_batch(
        &self,
        texts: &[String],
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if texts.is_empty() {
            return Err("Batch cannot be empty".into());
        }
        if texts.len() > 10_000 {
            return Err("Batch size exceeds maximum (10,000 texts)".into());
        }
        for (index, text) in texts.iter().enumerate() {
            self.validate_input(text)
                .map_err(|e| format!("Text at index {} failed validation: {}", index, e))?;
        }
        Ok(())
    }

    /// Get model configuration information
    fn config_info(&self) -> std::collections::HashMap<String, String> {
        let mut info = std::collections::HashMap::new();
        info.insert("name".to_string(), self.name().to_string());
        info.insert(
            "dimension".to_string(),
            self.embedding_dimension().to_string(),
        );
        info
    }

    /// Check if this model supports the specified task type
    fn supports_task(&self, _task: &str) -> bool {
        true
    }

    /// Check if the model is ready for processing
    fn health_check(&self) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Estimate memory usage for a batch operation
    fn estimate_memory_usage(&self, batch_size: usize) -> usize {
        let text_memory = batch_size * 1024;
        let embedding_memory = batch_size * self.embedding_dimension() * std::mem::size_of::<f32>();
        text_memory + embedding_memory
    }

    /// Process texts in chunks to avoid memory/performance issues
    fn chunked_batch_embed(
        &self,
        texts: &[String],
        task: Option<String>,
    ) -> BatchEmbeddingFuture<'_> {
        let texts = texts.to_vec();
        Box::pin(async move {
            if texts.is_empty() {
                return Ok(Vec::new());
            }
            let chunk_size = self.recommended_batch_size();
            let mut all_embeddings = Vec::with_capacity(texts.len());
            for chunk in texts.chunks(chunk_size) {
                let chunk_embeddings = self.batch_embed(chunk, task.clone()).await?;
                all_embeddings.extend(chunk_embeddings);
            }
            Ok(all_embeddings)
        })
    }
}

/// Trait for models capable of image embedding
pub trait ImageEmbeddingCapable: CandleModel {
    /// Generate embedding for an image from file path
    fn embed_image(&self, image_path: &str) -> EmbeddingFuture<'_>;

    /// Generate embedding for an image from URL
    fn embed_image_url(&self, url: &str) -> EmbeddingFuture<'_>;

    /// Generate embedding for an image from base64-encoded data
    fn embed_image_base64(&self, base64_data: &str) -> EmbeddingFuture<'_>;

    /// Generate embeddings for multiple images in batch
    fn batch_embed_images(&self, image_paths: Vec<&str>) -> BatchEmbeddingFuture<'_>;

    /// Get the dimensionality of embeddings produced by this model
    fn embedding_dimension(&self) -> usize;

    /// Get the supported embedding dimensions for this model
    fn supported_dimensions(&self) -> Vec<usize> {
        vec![self.embedding_dimension()]
    }
}

/// Trait for models capable of vision/multimodal understanding
pub trait VisionCapable: CandleModel {
    /// Describe an image with a text query, streaming tokens as generated
    fn describe_image(
        &self,
        image_path: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>>;

    /// Describe an image from URL with a text query, streaming tokens as generated
    fn describe_url(
        &self,
        url: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>>;
}

/// Trait for models capable of text-to-image generation
pub trait TextToImageCapable: CandleModel {
    /// Generate an image from a text prompt
    fn generate_image(
        &self,
        prompt: &str,
        config: &ImageGenerationConfig,
        device: &Device,
    ) -> Pin<Box<dyn Stream<Item = ImageGenerationChunk> + Send>>;

    /// Get the model's name
    fn registry_key(&self) -> &str;

    /// Get the default number of generation steps
    fn default_steps(&self) -> usize {
        50
    }
}
