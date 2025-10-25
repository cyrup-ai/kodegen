//! Loaded Stella model wrapper with thread-safe interior mutability

use super::config::{STELLA_1_5B_MODEL_INFO, STELLA_400M_MODEL_INFO, detect_variant, embed_dim};
use super::instruction::{format_single_with_instruction, format_with_instruction};
use super::utils::{
    configure_stella_tokenizer, create_stella_config, load_stella_weights,
};
use crate::capability::traits::TextEmbeddingCapable;
use crate::domain::model::CandleModelInfo;
use crate::domain::model::traits::CandleModel;
use anyhow::{Context, anyhow};
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
/// - model: Arc<Mutex<EmbeddingModel>> (std::sync::Mutex for spawn_blocking compatibility)
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
            .ok_or_else(|| anyhow!("embedding_dimension missing in ModelInfo"))?
            as usize;

        let variant = detect_variant(base_model.info().registry_key);
        let embed_dim = embed_dim(dimension as u32)?;

        // Use CPU for Stella embeddings (Metal has GPU→CPU transfer issues in async contexts)
        let device = Device::Cpu;
        let dtype = DType::F32;

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
        let tokenizer = self.tokenizer.clone();
        let model = self.model.clone();
        let device = self.device.clone();

        Box::pin(async move {
            // Wrap ENTIRE Metal operation in spawn_blocking (Metal requires specific thread context)
            let embedding_vec = tokio::task::spawn_blocking(move || -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
                log::info!("embed: Started in spawn_blocking");
                
                // Format with instruction prefix
                let formatted_text = format_single_with_instruction(&text, task.as_deref());
                log::info!("embed: Formatted text");

                // Tokenize
                let tokens = tokenizer
                    .encode(formatted_text, true)
                    .map_err(|e| anyhow!("Tokenization failed: {}", e))?;
                log::info!("embed: Tokenized, {} tokens", tokens.len());

                // Create 2D tensors directly with batch dimension (matches Candle example)
                let shape = (1, tokens.len());
                let input_ids = Tensor::from_slice(tokens.get_ids(), shape, &device)
                    .context("Failed to create input tensor")?;
                log::info!("embed: Created input_ids tensor");
                
                let attention_mask = Tensor::from_slice(tokens.get_attention_mask(), shape, &device)
                    .context("Failed to create attention mask")?;
                log::info!("embed: Created attention_mask tensor");

                // Forward pass - lock std::sync::Mutex in blocking context
                log::info!("embed: About to lock model");
                let embeddings = {
                    let mut model_guard = model.lock()
                        .map_err(|e| anyhow!("Model mutex poisoned (thread panic): {}", e))?;
                    log::info!("embed: Model locked, calling forward_norm");
                    model_guard
                        .forward_norm(&input_ids, &attention_mask)
                        .context("Stella forward pass failed")?
                };
                log::info!("embed: forward_norm completed");

                // Extract first embedding - squeeze batch dimension then to_vec1
                log::info!("embed: About to squeeze");
                let squeezed = embeddings
                    .squeeze(0)
                    .context("Failed to squeeze batch dimension")?;
                log::info!("embed: Squeezed, about to to_vec1");
                
                let vec = squeezed
                    .to_vec1::<f32>()
                    .context("Failed to convert embedding to vec")?;
                log::info!("embed: Converted to vec, length: {}", vec.len());

                Ok(vec)
            })
            .await
            .context("spawn_blocking join failed")??;

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
        let tokenizer = self.tokenizer.clone();
        let model = self.model.clone();
        let device = self.device.clone();

        Box::pin(async move {
            // Wrap ENTIRE Metal operation in spawn_blocking
            let embeddings_vec = tokio::task::spawn_blocking(move || -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error + Send + Sync>> {
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

                // Forward pass - lock std::sync::Mutex in blocking context
                let embeddings = {
                    let mut model_guard = model.lock()
                        .map_err(|e| anyhow!("Model mutex poisoned (thread panic): {}", e))?;
                    model_guard
                        .forward_norm(&input_ids, &attention_mask)
                        .context("Stella batch forward pass failed")?
                };

                // Convert to Vec<Vec<f32>>
                let vec = embeddings
                    .to_vec2::<f32>()
                    .context("Failed to convert batch embeddings to vec")?;

                Ok(vec)
            })
            .await
            .context("spawn_blocking join failed")??;

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
