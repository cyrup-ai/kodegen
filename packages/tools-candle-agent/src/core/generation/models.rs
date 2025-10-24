//! Model integration and wrapper functionality
//!
//! This module provides model wrappers and integration points for Candle models,
//! consolidating duplicate CandleLlamaModel definitions and providing a unified
//! interface for model operations.

use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::llama::{Cache, Llama};
use candle_transformers::models::quantized_llama;
use candle_transformers::models::quantized_mixformer;
use candle_transformers::models::quantized_phi3;

use super::types::CandleResult;
use crate::core::ModelConfig as CandleConfig;
use crate::core::model_config::ModelArchitecture;
use crate::domain::model::error::CandleModelError;

/// Wrapper to make raw pointers Send for spawn_blocking
///
/// # Safety
/// This is safe because:
/// 1. We have exclusive &mut access to the model
/// 2. spawn_blocking ensures the closure completes before the async fn returns
/// 3. The underlying models are Send + Sync
struct SendPtr<T>(*mut T);
unsafe impl<T> Send for SendPtr<T> {}

impl<T> SendPtr<T> {
    unsafe fn new(ptr: *mut T) -> Self {
        SendPtr(ptr)
    }

    unsafe fn into_mut(self) -> &'static mut T {
        unsafe { &mut *self.0 }
    }
}

/// Trait for Candle model implementations
///
/// Defines the interface that all Candle models must implement for
/// integration with the text generation system.
pub trait CandleModel: Send + Sync {
    /// Perform a forward pass through the model (async)
    ///
    /// Implementations should wrap CPU/GPU compute in spawn_blocking
    /// to avoid blocking the async runtime.
    fn forward<'a>(
        &'a mut self,
        input: &'a Tensor,
        position: usize,
    ) -> Pin<Box<dyn Future<Output = CandleResult<Tensor>> + Send + '_>>;

    /// Get the model's device
    fn device(&self) -> &Device;

    /// Get the model's vocabulary size
    fn vocab_size(&self) -> usize;

    /// Get model configuration if available
    fn config(&self) -> Option<&CandleConfig> {
        None
    }
}

/// Llama model wrapper for Candle integration
///
/// Consolidates duplicate CandleLlamaModel definitions into a single,
/// well-defined implementation with proper error handling and configuration.
#[derive(Debug)]
pub struct CandleLlamaModel {
    /// The underlying Llama model
    model: Llama,

    /// KV cache for the model
    cache: Cache,

    /// Device the model is loaded on
    device: Device,

    /// Model configuration
    config: Arc<CandleConfig>,

    /// Model vocabulary size
    vocab_size: usize,
}
impl CandleLlamaModel {
    /// Create new CandleLlamaModel
    pub fn new(
        model: Llama,
        cache: Cache,
        device: Device,
        config: Arc<CandleConfig>,
        vocab_size: usize,
    ) -> Self {
        Self {
            model,
            cache,
            device,
            config,
            vocab_size,
        }
    }

    /// Load a Llama model from the specified path
    pub async fn from_path<P: AsRef<std::path::Path>>(
        model_path: P,
        device: Device,
        config: Arc<CandleConfig>,
    ) -> CandleResult<Self> {
        let model_path = model_path.as_ref();

        // Extract the LlamaConfig from ModelConfig's architecture
        let llama_config = match &config.architecture {
            ModelArchitecture::Llama(llama_cfg) => llama_cfg,
            _ => {
                return Err(CandleModelError::InvalidConfiguration(
                    "Expected Llama architecture in config".into(),
                ));
            }
        };

        // Determine if single or multi-file model
        let safetensors_files = if model_path.is_file() {
            // Single file provided directly
            vec![model_path.to_path_buf()]
        } else if model_path.is_dir() {
            // Check for index file first (multi-file model)
            let index_path = model_path.join("model.safetensors.index.json");
            if index_path.exists() {
                discover_multi_file_model(&index_path).await?
            } else {
                // Single file in directory
                let single_file = model_path.join("model.safetensors");
                if single_file.exists() {
                    vec![single_file]
                } else {
                    return Err(CandleModelError::InvalidConfiguration(
                        format!("No model files found in {}", model_path.display()).into(),
                    ));
                }
            }
        } else {
            return Err(CandleModelError::InvalidConfiguration(
                format!("Invalid model path: {}", model_path.display()).into(),
            ));
        };

        // Load model weights using memory-mapped safetensors
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&safetensors_files, config.dtype, &device).map_err(
                |e| {
                    CandleModelError::InvalidConfiguration(
                        format!("Failed to load model weights: {}", e).into(),
                    )
                },
            )?
        };

        // Create KV cache for efficient inference
        let cache = Cache::new(true, config.dtype, llama_config, &device).map_err(|e| {
            CandleModelError::InvalidConfiguration(format!("Failed to create cache: {}", e).into())
        })?;

        // Load the Llama model from weights
        let model = Llama::load(vb, llama_config).map_err(|e| {
            CandleModelError::InvalidConfiguration(
                format!("Failed to load Llama model: {}", e).into(),
            )
        })?;

        let vocab_size = config.vocab_size;

        Ok(Self::new(model, cache, device, config, vocab_size))
    }

    /// Get the underlying Llama model
    pub fn inner(&self) -> &Llama {
        &self.model
    }

    /// Get the underlying Llama model mutably
    pub fn inner_mut(&mut self) -> &mut Llama {
        &mut self.model
    }

    /// Get model configuration
    pub fn config(&self) -> &CandleConfig {
        &self.config
    }
}
impl CandleLlamaModel {
    /// Synchronous forward pass (internal implementation)
    pub(crate) fn forward_sync(&mut self, input: &Tensor, position: usize) -> CandleResult<Tensor> {
        self.model
            .forward(input, position, &mut self.cache)
            .map_err(Into::into)
    }
}

impl CandleModel for CandleLlamaModel {
    fn forward<'a>(
        &'a mut self,
        input: &'a Tensor,
        position: usize,
    ) -> Pin<Box<dyn Future<Output = CandleResult<Tensor>> + Send + '_>> {
        Box::pin(async move {
            let input_clone = input.clone();
            let model_ptr = unsafe { SendPtr::new(self as *mut Self) };

            tokio::task::spawn_blocking(move || unsafe {
                model_ptr.into_mut().forward_sync(&input_clone, position)
            })
            .await
            .map_err(|e| {
                CandleModelError::Internal(format!("spawn_blocking failed: {}", e).into())
            })?
        })
    }

    fn device(&self) -> &Device {
        &self.device
    }

    fn vocab_size(&self) -> usize {
        self.vocab_size
    }

    fn config(&self) -> Option<&CandleConfig> {
        Some(&self.config)
    }
}

/// Quantized Llama model wrapper for GGUF models
///
/// This wrapper handles quantized models loaded from GGUF files using
/// the candle_transformers::models::quantized_llama module.
#[derive(Debug)]
pub struct CandleQuantizedLlamaModel {
    /// The underlying quantized model weights
    model_weights: quantized_llama::ModelWeights,

    /// Device the model is loaded on
    device: Device,

    /// Model configuration
    config: Arc<CandleConfig>,

    /// Model vocabulary size
    vocab_size: usize,
}

impl CandleQuantizedLlamaModel {
    /// Create new CandleQuantizedLlamaModel
    pub fn new(
        model_weights: quantized_llama::ModelWeights,
        device: Device,
        config: Arc<CandleConfig>,
        vocab_size: usize,
    ) -> Self {
        Self {
            model_weights,
            device,
            config,
            vocab_size,
        }
    }

    /// Load a quantized Llama model from a GGUF file
    pub async fn from_gguf_path<P: AsRef<std::path::Path>>(
        model_path: P,
        device: Device,
        config: Arc<CandleConfig>,
    ) -> CandleResult<Self> {
        let file = tokio::fs::File::open(&model_path).await.map_err(|e| {
            crate::domain::model::error::CandleModelError::InvalidConfiguration(
                format!("Failed to open GGUF file: {}", e).into(),
            )
        })?;
        let mut file = file.into_std().await;
        let device_clone = device.clone();

        // CRITICAL: Run blocking GGUF operations on blocking thread
        let model_weights = tokio::task::spawn_blocking(move || {
            let gguf_content = gguf_file::Content::read(&mut file)?;
            quantized_llama::ModelWeights::from_gguf(gguf_content, &mut file, &device_clone)
        })
        .await
        .map_err(|e| {
            crate::domain::model::error::CandleModelError::InvalidConfiguration(
                format!("Failed to spawn blocking task: {}", e).into(),
            )
        })?
        .map_err(|e| {
            crate::domain::model::error::CandleModelError::InvalidConfiguration(
                format!("Failed to load model weights from GGUF: {}", e).into(),
            )
        })?;

        // Extract vocab size from config or use default
        let vocab_size = config.vocab_size;

        Ok(Self::new(model_weights, device, config, vocab_size))
    }

    /// Get the underlying model weights
    pub fn model_weights(&self) -> &quantized_llama::ModelWeights {
        &self.model_weights
    }

    /// Get the underlying model weights mutably
    pub fn model_weights_mut(&mut self) -> &mut quantized_llama::ModelWeights {
        &mut self.model_weights
    }
}

impl CandleQuantizedLlamaModel {
    /// Synchronous forward pass (internal implementation)
    pub(crate) fn forward_sync(&mut self, input: &Tensor, position: usize) -> CandleResult<Tensor> {
        self.model_weights
            .forward(input, position)
            .map_err(Into::into)
    }
}

impl CandleModel for CandleQuantizedLlamaModel {
    fn forward<'a>(
        &'a mut self,
        input: &'a Tensor,
        position: usize,
    ) -> Pin<Box<dyn Future<Output = CandleResult<Tensor>> + Send + '_>> {
        Box::pin(async move {
            let input_clone = input.clone();
            let model_ptr = unsafe { SendPtr::new(self as *mut Self) };

            tokio::task::spawn_blocking(move || unsafe {
                model_ptr.into_mut().forward_sync(&input_clone, position)
            })
            .await
            .map_err(|e| {
                CandleModelError::Internal(format!("spawn_blocking failed: {}", e).into())
            })?
        })
    }

    fn device(&self) -> &Device {
        &self.device
    }

    fn vocab_size(&self) -> usize {
        self.vocab_size
    }

    fn config(&self) -> Option<&CandleConfig> {
        Some(&self.config)
    }
}

/// Quantized MixFormer (Phi) model wrapper for GGUF models
///
/// This wrapper handles quantized MixFormer models (Phi architecture) loaded from GGUF files.
/// MixFormer models manage KV cache internally, so the position parameter in forward() is ignored.
#[derive(Debug)]
pub struct CandleQuantizedMixFormerModel {
    /// The underlying quantized MixFormer model
    model_weights: quantized_mixformer::MixFormerSequentialForCausalLM,
    /// Device the model is loaded on
    device: Device,
    /// Model vocabulary size from config
    vocab_size: usize,
}

impl CandleQuantizedMixFormerModel {
    /// Create new CandleQuantizedMixFormerModel
    pub fn new(
        model_weights: quantized_mixformer::MixFormerSequentialForCausalLM,
        device: Device,
        vocab_size: usize,
    ) -> Self {
        Self {
            model_weights,
            device,
            vocab_size,
        }
    }

    /// Load a quantized MixFormer model from a GGUF file
    pub async fn from_gguf_path<P: AsRef<std::path::Path>>(
        model_path: P,
        device: Device,
    ) -> CandleResult<Self> {
        use candle_core::quantized::gguf_file;
        use candle_transformers::models::mixformer::Config as MixFormerConfig;
        use candle_transformers::quantized_var_builder::VarBuilder as QuantizedVarBuilder;

        // Convert to PathBuf before moving into closure
        let model_path_buf = model_path.as_ref().to_path_buf();

        // First, read GGUF file to extract metadata
        let file = tokio::fs::File::open(&model_path_buf).await.map_err(|e| {
            crate::domain::model::error::CandleModelError::InvalidConfiguration(
                format!("Failed to open GGUF file: {}", e).into(),
            )
        })?;
        let mut file = file.into_std().await;
        let device_clone = device.clone();

        // CRITICAL: Run blocking GGUF operations on blocking thread
        let (vocab_size, config, vb, gguf_content) =
            tokio::task::spawn_blocking(move || -> Result<_, candle_core::Error> {
                let gguf_content = gguf_file::Content::read(&mut file)?;

                // Extract vocab size from GGUF metadata
                let vocab_size = gguf_content
                    .metadata
                    .get("tokenizer.ggml.tokens")
                    .and_then(|v| match v {
                        gguf_file::Value::Array(arr) => Some(arr.len()),
                        _ => None,
                    })
                    .unwrap_or(32000); // Default MixFormer vocab size

                log::info!(
                    "Loading MixFormer model from GGUF with vocab_size={}",
                    vocab_size
                );

                // Use default v2 config (Phi-3 style)
                let config = MixFormerConfig::v2();

                // Load GGUF model using quantized VarBuilder
                let vb = QuantizedVarBuilder::from_gguf(&model_path_buf, &device_clone)?;

                Ok((vocab_size, config, vb, gguf_content))
            })
            .await
            .map_err(|e| {
                crate::domain::model::error::CandleModelError::InvalidConfiguration(
                    format!("Failed to spawn blocking task: {}", e).into(),
                )
            })?
            .map_err(|e| {
                crate::domain::model::error::CandleModelError::InvalidConfiguration(
                    format!("Failed to load GGUF model: {}", e).into(),
                )
            })?;

        // Log first 20 tensor names to understand the structure
        log::info!("GGUF tensor names (first 20):");
        for (i, tensor_name) in gguf_content.tensor_infos.keys().take(20).enumerate() {
            log::info!("  [{}] {}", i, tensor_name);
        }

        // Detect tensor layout by checking which tensors exist
        // new_v2 layout uses "transformer." prefix, old layout uses "layers."
        let uses_transformer_prefix = gguf_content
            .tensor_infos
            .keys()
            .any(|k| k.starts_with("transformer."));

        log::info!(
            "Detected tensor layout: uses_transformer_prefix={}",
            uses_transformer_prefix
        );

        // Create model using appropriate constructor based on tensor layout
        let model = if uses_transformer_prefix {
            log::info!("Using new_v2() constructor for transformer-prefixed layout");
            quantized_mixformer::MixFormerSequentialForCausalLM::new_v2(&config, vb)
        } else {
            log::info!("Using new() constructor for layers-prefixed layout");
            quantized_mixformer::MixFormerSequentialForCausalLM::new(&config, vb)
        }
        .map_err(|e| {
            crate::domain::model::error::CandleModelError::InvalidConfiguration(
                format!("Failed to create MixFormer model: {}", e).into(),
            )
        })?;

        Ok(Self::new(model, device, vocab_size))
    }

    /// Get the underlying model weights
    pub fn model_weights(&self) -> &quantized_mixformer::MixFormerSequentialForCausalLM {
        &self.model_weights
    }

    /// Get the underlying model weights mutably
    pub fn model_weights_mut(
        &mut self,
    ) -> &mut quantized_mixformer::MixFormerSequentialForCausalLM {
        &mut self.model_weights
    }
}

impl CandleQuantizedMixFormerModel {
    /// Synchronous forward pass (internal implementation)
    pub(crate) fn forward_sync(
        &mut self,
        input: &Tensor,
        _position: usize,
    ) -> CandleResult<Tensor> {
        // MixFormer manages KV cache internally, position parameter is ignored
        self.model_weights.forward(input).map_err(Into::into)
    }
}

impl CandleModel for CandleQuantizedMixFormerModel {
    fn forward<'a>(
        &'a mut self,
        input: &'a Tensor,
        position: usize,
    ) -> Pin<Box<dyn Future<Output = CandleResult<Tensor>> + Send + '_>> {
        Box::pin(async move {
            let input_clone = input.clone();
            let model_ptr = unsafe { SendPtr::new(self as *mut Self) };

            tokio::task::spawn_blocking(move || unsafe {
                model_ptr.into_mut().forward_sync(&input_clone, position)
            })
            .await
            .map_err(|e| {
                CandleModelError::Internal(format!("spawn_blocking failed: {}", e).into())
            })?
        })
    }

    fn device(&self) -> &Device {
        &self.device
    }

    fn vocab_size(&self) -> usize {
        self.vocab_size
    }
}

/// Wrapper for quantized Phi-3/Phi-4 models loaded from GGUF files
///
/// Phi-3 and Phi-4 use a specific tensor layout with `token_embd`, `blk.*`, and `output` tensors
/// and phi3.* metadata keys. This is the correct model type for Phi-4.
#[derive(Debug)]
pub struct CandleQuantizedPhiModel {
    /// The underlying quantized Phi-3/Phi-4 model
    model_weights: quantized_phi3::ModelWeights,
    /// Device the model is loaded on
    device: Device,
    /// Vocabulary size
    vocab_size: usize,
    /// End-of-sequence token ID from GGUF metadata
    eos_token_id: Option<u32>,
}

impl CandleQuantizedPhiModel {
    /// Create new CandleQuantizedPhiModel
    pub fn new(
        model_weights: quantized_phi3::ModelWeights,
        device: Device,
        vocab_size: usize,
        eos_token_id: Option<u32>,
    ) -> Self {
        Self {
            model_weights,
            device,
            vocab_size,
            eos_token_id,
        }
    }

    /// Get the EOS token ID extracted from GGUF metadata
    pub fn eos_token_id(&self) -> Option<u32> {
        self.eos_token_id
    }

    /// Load a quantized Phi model from a GGUF file
    pub async fn from_gguf_path<P: AsRef<std::path::Path>>(
        model_path: P,
        device: Device,
    ) -> CandleResult<Self> {
        use candle_core::quantized::gguf_file;

        // Open GGUF file
        let file = tokio::fs::File::open(model_path.as_ref())
            .await
            .map_err(|e| {
                CandleModelError::InvalidConfiguration(
                    format!("Failed to open GGUF file: {}", e).into(),
                )
            })?;
        let mut file = file.into_std().await;
        let device_clone = device.clone();

        // CRITICAL: Run blocking GGUF operations on a blocking thread to avoid blocking async runtime
        let (model, vocab_size, eos_token_id) =
            tokio::task::spawn_blocking(move || -> Result<_, candle_core::Error> {
                // Read GGUF content
                let gguf_content = gguf_file::Content::read(&mut file)?;

                // Extract vocab size from GGUF metadata
                let vocab_size = gguf_content
                    .metadata
                    .get("tokenizer.ggml.tokens")
                    .and_then(|v| match v {
                        gguf_file::Value::Array(arr) => Some(arr.len()),
                        _ => None,
                    })
                    .unwrap_or(100352); // Phi-4 default vocab size

                // Extract EOS token ID from GGUF metadata
                let eos_token_id = gguf_content
                    .metadata
                    .get("tokenizer.ggml.eos_token_id")
                    .and_then(|v| match v {
                        gguf_file::Value::U32(id) => Some(*id),
                        gguf_file::Value::I32(id) if *id >= 0 => Some(*id as u32),
                        _ => None,
                    });

                log::info!(
                    "Loading Phi model from GGUF with vocab_size={}, eos_token_id={:?}",
                    vocab_size,
                    eos_token_id
                );

                // Log available metadata keys to understand what Phi-4 uses
                log::info!("GGUF metadata keys (first 30):");
                for (i, key) in gguf_content.metadata.keys().take(30).enumerate() {
                    log::info!("  [{}] {}", i, key);
                }

                // Create model using quantized_phi3 (Phi-4 uses phi3.* metadata)
                // Phi-4 does not support flash attention according to model docs
                let use_flash_attn = false;
                log::info!(
                    "Loading quantized_phi3 model with flash_attn={}",
                    use_flash_attn
                );

                let model = quantized_phi3::ModelWeights::from_gguf(
                    use_flash_attn,
                    gguf_content,
                    &mut file,
                    &device_clone,
                )?;

                Ok((model, vocab_size, eos_token_id))
            })
            .await
            .map_err(|e| {
                CandleModelError::InvalidConfiguration(
                    format!("Failed to spawn blocking task: {}", e).into(),
                )
            })?
            .map_err(|e| {
                CandleModelError::InvalidConfiguration(
                    format!("Failed to load Phi-3/Phi-4 model: {}", e).into(),
                )
            })?;

        log::info!("✅ Phi model loaded successfully!");
        Ok(Self::new(model, device, vocab_size, eos_token_id))
    }
}

impl CandleQuantizedPhiModel {
    /// Synchronous forward pass (internal implementation)
    pub(crate) fn forward_sync(
        &mut self,
        input: &Tensor,
        index_pos: usize,
    ) -> CandleResult<Tensor> {
        self.model_weights
            .forward(input, index_pos)
            .map_err(Into::into)
    }
}

impl CandleModel for CandleQuantizedPhiModel {
    fn forward<'a>(
        &'a mut self,
        input: &'a Tensor,
        index_pos: usize,
    ) -> Pin<Box<dyn Future<Output = CandleResult<Tensor>> + Send + '_>> {
        Box::pin(async move {
            let input_clone = input.clone();
            let model_ptr = unsafe { SendPtr::new(self as *mut Self) };

            tokio::task::spawn_blocking(move || unsafe {
                model_ptr.into_mut().forward_sync(&input_clone, index_pos)
            })
            .await
            .map_err(|e| {
                CandleModelError::Internal(format!("spawn_blocking failed: {}", e).into())
            })?
        })
    }

    fn device(&self) -> &Device {
        &self.device
    }

    fn vocab_size(&self) -> usize {
        self.vocab_size
    }
}

/// Helper function to discover model files from a multi-file model index
///
/// Parses a `model.safetensors.index.json` file to find all weight files
/// referenced in the weight_map, enabling loading of large models split
/// across multiple safetensors files.
async fn discover_multi_file_model(index_path: &Path) -> CandleResult<Vec<PathBuf>> {
    let content = tokio::fs::read_to_string(index_path).await.map_err(|e| {
        CandleModelError::InvalidConfiguration(format!("Failed to read index file: {}", e).into())
    })?;

    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        CandleModelError::InvalidConfiguration(format!("Failed to parse index JSON: {}", e).into())
    })?;

    let weight_map = json
        .get("weight_map")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            CandleModelError::InvalidConfiguration("No weight_map in index file".into())
        })?;

    let base_dir = index_path.parent().ok_or_else(|| {
        CandleModelError::InvalidConfiguration(
            format!("Invalid index path: {}", index_path.display()).into(),
        )
    })?;

    let mut files = HashSet::new();

    for value in weight_map.values() {
        if let Some(filename) = value.as_str() {
            files.insert(base_dir.join(filename));
        }
    }

    Ok(files.into_iter().collect())
}
