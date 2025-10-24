//! CLIP Vision Embedding Provider - Async Wrapper for EmbeddingModel Trait
//!
//! This module provides an async wrapper around ClipVisionProvider to enable
//! integration with the EmbeddingModel trait system and EmbeddingModelFactory.

use std::sync::Arc;

use super::{ClipVisionModel, LoadedClipVisionModel};
use crate::domain::model::CandleModelInfo;
use crate::domain::model::traits::CandleModel;
use crate::memory::utils::error::{Error as MemoryError, Result};

/// Async wrapper for ClipVisionModel implementing EmbeddingModel trait
///
/// This adapter wraps the async ClipVisionModel for integration with the
/// async ImageEmbeddingCapable trait system.
///
/// Supports two modes:
/// - Lazy loading: Model loaded on-demand for each inference call (via provider)
/// - Pre-loaded: Model loaded once and reused (via loaded_model) for better performance
#[derive(Clone)]
pub struct ClipVisionEmbeddingModel {
    provider: Arc<ClipVisionModel>, // Lazy loading fallback
    loaded_model: Option<Arc<LoadedClipVisionModel>>, // Pre-loaded for performance
    dimension: usize,
}

impl std::fmt::Debug for ClipVisionEmbeddingModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipVisionEmbeddingModel")
            .field("provider", &"ClipVisionModel { .. }")
            .field("dimension", &self.dimension)
            .finish()
    }
}

impl ClipVisionEmbeddingModel {
    /// Create with pre-loaded model for optimal performance
    ///
    /// This method downloads and loads the model once, making subsequent
    /// inference calls much faster as they reuse the loaded model.
    pub async fn load(dimension: usize) -> Result<Self> {
        // Validate dimension (CLIP supports 512D for Base, 768D for Large)
        if dimension != 512 && dimension != 768 {
            return Err(MemoryError::Config(format!(
                "CLIP Vision supports 512D (Base) or 768D (Large). Requested: {}",
                dimension
            )));
        }

        let loaded = LoadedClipVisionModel::load(dimension)
            .await
            .map_err(|e| MemoryError::ModelError(format!("Failed to load CLIP model: {}", e)))?;

        let provider = ClipVisionModel::new(dimension).map_err(MemoryError::Config)?;

        Ok(Self {
            provider: Arc::new(provider),
            loaded_model: Some(Arc::new(loaded)),
            dimension,
        })
    }

    /// Create new CLIP vision embedding provider with ViT-Base configuration (512D)
    ///
    /// Uses lazy loading - model loaded on-demand for each inference call.
    /// For better performance with repeated inference, use `load()` instead.
    pub async fn new() -> Result<Self> {
        Self::with_dimension(512).await
    }

    /// Create CLIP vision provider with specific dimension
    ///
    /// # Arguments
    /// * `dimension` - 512 for ViT-Base-Patch32 or 768 for ViT-Large-Patch14-336
    ///
    /// Uses lazy loading - model loaded on-demand for each inference call.
    /// For better performance with repeated inference, use `load(dimension)` instead.
    pub async fn with_dimension(dimension: usize) -> Result<Self> {
        // Validate dimension (CLIP supports 512D for Base, 768D for Large)
        if dimension != 512 && dimension != 768 {
            return Err(MemoryError::Config(format!(
                "CLIP Vision supports 512D (Base) or 768D (Large). Requested: {}",
                dimension
            )));
        }

        // Create provider with specified dimension (lazy loading)
        let provider = ClipVisionModel::new(dimension).map_err(MemoryError::Config)?;

        Ok(Self {
            provider: Arc::new(provider),
            loaded_model: None, // Will lazy-load on first use
            dimension,
        })
    }

    /// Create from existing ClipVisionModel (lazy loading)
    pub fn from_model(model: ClipVisionModel, dimension: usize) -> Self {
        Self {
            provider: Arc::new(model),
            loaded_model: None,
            dimension,
        }
    }

    /// Create from pre-loaded LoadedClipVisionModel (optimal performance)
    pub fn from_loaded(loaded: LoadedClipVisionModel, dimension: usize) -> Result<Self> {
        let provider = ClipVisionModel::new(dimension).map_err(MemoryError::Config)?;

        Ok(Self {
            provider: Arc::new(provider),
            loaded_model: Some(Arc::new(loaded)),
            dimension,
        })
    }

    /// Deprecated: Use from_model instead
    #[deprecated(since = "0.1.0", note = "Use from_model instead")]
    pub fn from_provider(provider: ClipVisionModel, dimension: usize) -> Self {
        Self {
            provider: Arc::new(provider),
            loaded_model: None,
            dimension,
        }
    }

    /// Encode image from file path (public API for direct image embedding)
    ///
    /// This is the primary method for encoding images to embeddings.
    pub async fn embed_image(&self, image_path: &str) -> Result<Vec<f32>> {
        let tensor = self
            .provider
            .encode_image(image_path)
            .await
            .map_err(|e| MemoryError::ModelError(format!("Image encoding failed: {}", e)))?;

        // Convert Tensor to Vec<f32>
        tensor
            .flatten_all()
            .and_then(|t| t.to_vec1::<f32>())
            .map_err(|e| {
                MemoryError::ModelError(format!("Failed to convert tensor to vector: {}", e))
            })
    }

    /// Encode image from URL
    pub async fn embed_image_url(&self, url: &str) -> Result<Vec<f32>> {
        let tensor =
            self.provider.encode_url(url).await.map_err(|e| {
                MemoryError::ModelError(format!("Image URL encoding failed: {}", e))
            })?;

        tensor
            .flatten_all()
            .and_then(|t| t.to_vec1::<f32>())
            .map_err(|e| {
                MemoryError::ModelError(format!("Failed to convert tensor to vector: {}", e))
            })
    }

    /// Encode image from base64 data
    pub async fn embed_image_base64(&self, base64_data: &str) -> Result<Vec<f32>> {
        let tensor = self
            .provider
            .encode_base64(base64_data)
            .await
            .map_err(|e| MemoryError::ModelError(format!("Base64 image encoding failed: {}", e)))?;

        tensor
            .flatten_all()
            .and_then(|t| t.to_vec1::<f32>())
            .map_err(|e| {
                MemoryError::ModelError(format!("Failed to convert tensor to vector: {}", e))
            })
    }

    /// Batch encode multiple images
    pub async fn batch_embed_images(&self, image_paths: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let batch_tensor =
            self.provider.encode_batch(image_paths).await.map_err(|e| {
                MemoryError::ModelError(format!("Batch image encoding failed: {}", e))
            })?;

        // Convert batch tensor (N, D) to Vec<Vec<f32>>
        let batch_size = batch_tensor
            .dim(0)
            .map_err(|e| MemoryError::ModelError(format!("Failed to get batch size: {}", e)))?;

        let mut embeddings = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let row = batch_tensor
                .get(i)
                .and_then(|t| t.flatten_all())
                .and_then(|t| t.to_vec1::<f32>())
                .map_err(|e| {
                    MemoryError::ModelError(format!("Failed to extract embedding {}: {}", i, e))
                })?;
            embeddings.push(row);
        }

        Ok(embeddings)
    }
}

// Static model info for CLIP Vision Embedding
#[allow(dead_code)] // Reserved for future vision embedding model registry
static CLIP_VISION_EMBEDDING_MODEL_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::OpenAI,
    name: "clip-vit-base-patch32",
    registry_key: "openai/clip-vit-base-patch32",
    quantization_url: None,
    max_input_tokens: None,
    max_output_tokens: None,
    input_price: None,
    output_price: None,
    supports_vision: true,
    supports_function_calling: false,
    supports_streaming: false,
    supports_embeddings: true,
    requires_max_tokens: false,
    supports_thinking: false,
    optimal_thinking_budget: None,
    system_prompt_prefix: None,
    real_name: None,
    model_type: None,
    model_id: "clip-vision-embedding",
    quantization: "none",
    patch: None,
    embedding_dimension: Some(512),
    vocab_size: None,
    image_size: Some(224),
    image_mean: Some([0.48145466, 0.4578275, 0.40821073]),
    image_std: Some([0.26862954, 0.261_302_6, 0.275_777_1]),
    default_temperature: None,
    default_top_k: None,
    default_top_p: None,
    supports_kv_cache: false,
    supports_flash_attention: false,
    use_bf16: false,
    default_steps: None,
    default_guidance_scale: None,
    time_shift: None,
    est_memory_allocation_mb: 0,
};

impl CandleModel for ClipVisionEmbeddingModel {
    fn info(&self) -> &'static CandleModelInfo {
        // Delegate to provider which has correct ModelInfo based on dimension
        self.provider.info()
    }
}

impl crate::capability::traits::ImageEmbeddingCapable for ClipVisionEmbeddingModel {
    fn embed_image(
        &self,
        image_path: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        Vec<f32>,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send
                + '_,
        >,
    > {
        // Use loaded model if available, otherwise fall back to lazy loading
        if let Some(loaded) = &self.loaded_model {
            loaded.embed_image(image_path)
        } else {
            let image_path = image_path.to_string();
            Box::pin(async move {
                ClipVisionEmbeddingModel::embed_image(self, &image_path)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
        }
    }

    fn embed_image_url(
        &self,
        url: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        Vec<f32>,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send
                + '_,
        >,
    > {
        // Use loaded model if available, otherwise fall back to lazy loading
        if let Some(loaded) = &self.loaded_model {
            loaded.embed_image_url(url)
        } else {
            let url = url.to_string();
            Box::pin(async move {
                ClipVisionEmbeddingModel::embed_image_url(self, &url)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
        }
    }

    fn embed_image_base64(
        &self,
        base64_data: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        Vec<f32>,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send
                + '_,
        >,
    > {
        // Use loaded model if available, otherwise fall back to lazy loading
        if let Some(loaded) = &self.loaded_model {
            loaded.embed_image_base64(base64_data)
        } else {
            let base64_data = base64_data.to_string();
            Box::pin(async move {
                ClipVisionEmbeddingModel::embed_image_base64(self, &base64_data)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
        }
    }

    fn batch_embed_images(
        &self,
        image_paths: Vec<&str>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        Vec<Vec<f32>>,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send
                + '_,
        >,
    > {
        // Use loaded model if available, otherwise fall back to lazy loading
        if let Some(loaded) = &self.loaded_model {
            loaded.batch_embed_images(image_paths)
        } else {
            let paths: Vec<String> = image_paths.iter().map(|s| s.to_string()).collect();
            Box::pin(async move {
                let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                ClipVisionEmbeddingModel::batch_embed_images(self, path_refs)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
        }
    }

    fn embedding_dimension(&self) -> usize {
        self.dimension
    }

    fn supported_dimensions(&self) -> Vec<usize> {
        vec![512, 768]
    }
}
