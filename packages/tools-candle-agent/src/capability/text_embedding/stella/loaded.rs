//! Loaded Stella model wrapper with thread-safe interior mutability

use super::config::{STELLA_1_5B_MODEL_INFO, STELLA_400M_MODEL_INFO, detect_variant, embed_dim};
use super::instruction::{format_single_with_instruction, format_with_instruction};
use super::utils::{
    configure_stella_tokenizer, create_stella_config, detect_device_and_dtype, load_stella_weights,
};
use anyhow::{anyhow, Context, Result as AnyResult};
use crate::capability::traits::TextEmbeddingCapable;
use crate::domain::model::CandleModelInfo;
use crate::domain::model::traits::CandleModel;
use candle_core::{DType, Device, Tensor};
use candle_transformers::models::stella_en_v5::{Config, EmbeddingModel, ModelVariant};
use tokenizers::Tokenizer;

/// Loaded Stella model that keeps model/tokenizer in memory.
///
/// This wrapper is designed for use in model pool workers where the model is loaded once
/// during worker spawn and reused across many inference calls, eliminating repeated disk I/O.
///
/// ## Usage Pattern
/// ```rust,ignore
/// // In worker spawn:
/// let loaded_model = LoadedStellaModel::load(&base_model)?;
///
/// // In worker loop (no I/O):
/// let embedding = loaded_model.embed("some text", None)?;
/// ```
///
/// ## Memory Layout
/// - tokenizer: Arc<Tokenizer> (shared, cheap to clone)
/// - model: Arc<Mutex<EmbeddingModel>> (thread-safe interior mutability)
/// - device: Device (CUDA or CPU)
/// - config: Config (Stella model configuration)
/// - variant: ModelVariant (Large=1.5B or Small=400M)
#[derive(Clone)]
pub struct LoadedStellaModel {
    tokenizer: std::sync::Arc<Tokenizer>,
    model: std::sync::Arc<std::sync::Mutex<EmbeddingModel>>,
    device: Device,
    config: Config,
    variant: ModelVariant,
}

impl std::fmt::Debug for LoadedStellaModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedStellaModel")
            .field("device", &self.device)
            .field("variant", &self.variant)
            .field("model", &"Arc<Mutex<EmbeddingModel>>")
            .finish()
    }
}

impl CandleModel for LoadedStellaModel {
    fn info(&self) -> &'static CandleModelInfo {
        // Return the correct ModelInfo based on the loaded variant
        // This ensures the memory governor gets accurate allocation sizes
        match self.variant {
            ModelVariant::Large => &STELLA_1_5B_MODEL_INFO,
            ModelVariant::Small => &STELLA_400M_MODEL_INFO,
        }
    }
}

impl LoadedStellaModel {
    /// Load model and tokenizer from disk once, returning loaded instance ready for inference.
    pub async fn load(
        base_model: &super::base::StellaEmbeddingModel,
    ) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Get config from ModelInfo
        let max_length = base_model
            .info()
            .max_input_tokens
            .ok_or_else(|| anyhow!("max_input_tokens missing in ModelInfo"))?
            .get() as usize;

        let dimension = base_model
            .info()
            .embedding_dimension
            .ok_or_else(|| anyhow!("embedding_dimension missing in ModelInfo"))? as usize;

        let variant = detect_variant(base_model.info().registry_key);
        let embed_dim = embed_dim(dimension as u32)?;

        // Auto-detect device and dtype
        let (device, dtype) = detect_device_and_dtype();

        // Load files from HuggingFace
        let base_weights = base_model
            .huggingface_file(base_model.info().registry_key, "model.safetensors")
            .await?;
        let projection_head = base_model
            .huggingface_file(
                base_model.info().registry_key,
                &format!("2_Dense_{}/model.safetensors", dimension),
            )
            .await?;
        let tokenizer_path = base_model
            .huggingface_file(base_model.info().registry_key, "tokenizer.json")
            .await?;

        // Load tokenizer and configure padding/truncation
        let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;
        configure_stella_tokenizer(&mut tokenizer, variant, max_length)?;

        // Create config and load weights using shared utils
        let stella_config = create_stella_config(variant, embed_dim);
        let (base_vb, embed_vb) =
            load_stella_weights(base_weights, projection_head, dtype, &device)?;

        // Create Stella model with MRL projection
        let model = EmbeddingModel::new(&stella_config, base_vb, embed_vb)
            .context("Failed to create Stella model")?;

        Ok(Self {
            tokenizer: std::sync::Arc::new(tokenizer),
            model: std::sync::Arc::new(std::sync::Mutex::new(model)),
            device,
            config: stella_config,
            variant,
        })
    }

    /// Get the embedding output dimension
    pub fn embedding_dimension(&self) -> usize {
        self.config.embed_head.out_features
    }

    /// Get supported MRL dimensions (Matryoshka Representation Learning)
    pub fn supported_dimensions(&self) -> Vec<usize> {
        vec![256, 768, 1024, 2048, 4096, 6144, 8192]
    }
}

impl TextEmbeddingCapable for LoadedStellaModel {
    fn embed(
        &self,
        text: &str,
        task: Option<String>,
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
        let text = text.to_string();
        // Clone for move into spawn_blocking
        let tokenizer = self.tokenizer.clone();
        let model = self.model.clone();
        let device = self.device.clone();

        Box::pin(async move {
            // Wrap CPU-intensive operations in spawn_blocking to avoid blocking async runtime
            let embedding_vec = tokio::task::spawn_blocking(move || -> AnyResult<Vec<f32>> {
                // Format with instruction prefix
                let formatted_text = format_single_with_instruction(&text, task.as_deref());

                // Tokenize
                let tokens = tokenizer
                    .encode(formatted_text, true)
                    .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

                let input_ids = Tensor::new(tokens.get_ids(), &device)
                    .context("Failed to create input tensor")?;
                let attention_mask = Tensor::new(tokens.get_attention_mask(), &device)
                    .context("Failed to create attention mask")?
                    .to_dtype(DType::U8)
                    .context("Failed to convert mask dtype")?;

                // Forward pass - lock synchronous mutex in blocking context
                let mut model_guard = model.lock().unwrap_or_else(|poisoned| {
                    log::error!("Model mutex was poisoned, attempting recovery");
                    log::error!("Poison error: {:?}", poisoned);
                    poisoned.into_inner()
                });
                let embeddings = model_guard
                    .forward_norm(
                        &input_ids
                            .unsqueeze(0)
                            .context("Failed to unsqueeze input_ids")?,
                        &attention_mask
                            .unsqueeze(0)
                            .context("Failed to unsqueeze attention_mask")?,
                    )
                    .context("Stella forward pass failed")?;

                // Extract first embedding
                embeddings
                    .to_vec2::<f32>()
                    .context("Failed to convert embeddings to vec")?
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No embeddings generated"))
            })
            .await
            .context("Spawn blocking failed")??;

            Ok(embedding_vec)
        })
    }

    fn batch_embed(
        &self,
        texts: &[String],
        task: Option<String>,
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
        let texts = texts.to_vec();
        // Clone for move into spawn_blocking
        let tokenizer = self.tokenizer.clone();
        let model = self.model.clone();
        let device = self.device.clone();

        Box::pin(async move {
            // Wrap CPU-intensive operations in spawn_blocking to avoid blocking async runtime
            let embeddings_vec = tokio::task::spawn_blocking(move || -> AnyResult<Vec<Vec<f32>>> {
                let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

                // Format with instruction prefix
                let formatted_texts = format_with_instruction(&text_refs, task.as_deref());

                // Tokenize batch
                let encodings = tokenizer
                    .encode_batch(formatted_texts, true)
                    .map_err(|e| anyhow!("Batch tokenization failed: {}", e))?;

                // Create batch tensors
                let ids_vecs: Vec<Vec<u32>> =
                    encodings.iter().map(|e| e.get_ids().to_vec()).collect();
                let mask_vecs: Vec<Vec<u32>> = encodings
                    .iter()
                    .map(|e| e.get_attention_mask().to_vec())
                    .collect();

                let input_ids = Tensor::new(ids_vecs, &device)
                    .context("Failed to create batch input tensor")?;
                let attention_mask = Tensor::new(mask_vecs, &device)
                    .context("Failed to create batch attention mask")?
                    .to_dtype(DType::U8)
                    .context("Failed to convert mask dtype")?;

                // Forward pass - lock synchronous mutex in blocking context
                let mut model_guard = model.lock().unwrap_or_else(|poisoned| {
                    log::error!("Model mutex was poisoned, attempting recovery");
                    log::error!("Poison error: {:?}", poisoned);
                    poisoned.into_inner()
                });
                let embeddings = model_guard
                    .forward_norm(&input_ids, &attention_mask)
                    .context("Stella batch forward pass failed")?;

                // Convert to Vec<Vec<f32>>
                embeddings
                    .to_vec2::<f32>()
                    .context("Failed to convert batch embeddings to vec")
            })
            .await
            .context("Spawn blocking failed")??;

            Ok(embeddings_vec)
        })
    }

    fn embedding_dimension(&self) -> usize {
        self.config.embed_head.out_features
    }

    fn recommended_batch_size(&self) -> usize {
        match self.variant {
            ModelVariant::Large => 8,
            ModelVariant::Small => 16,
        }
    }

    fn max_batch_size(&self) -> usize {
        match self.variant {
            ModelVariant::Large => 32,
            ModelVariant::Small => 64,
        }
    }
}
