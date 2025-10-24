//! Core CLIP Vision model structures
//!
//! This module defines the two CLIP Vision model patterns:
//! - ClipVisionModel: Lazy loading pattern (model loaded on-demand)
//! - LoadedClipVisionModel: Pre-loaded pattern (model loaded once and reused)

use crate::domain::model::CandleModelInfo;
use crate::domain::model::traits::CandleModel;
use candle_core::Device;
use candle_nn::VarBuilder;
use candle_transformers::models::clip::{ClipConfig, ClipModel};
use std::sync::Arc;

use super::config::{CLIP_VISION_BASE_INFO, CLIP_VISION_LARGE_INFO, get_configs_for_dimension};

/// CLIP vision provider for image embeddings (lazy loading pattern)
///
/// Uses ClipModel.get_image_features() for encoding images to embeddings.
/// Supports ViT-Base-Patch32 (224×224, 512-dim) and ViT-Large-Patch14-336 (336×336, 768-dim).
///
/// Uses lazy loading pattern - model loaded on-demand via huggingface_file().
pub struct ClipVisionModel {
    pub(crate) dimension: usize, // 512 for Base, 768 for Large
}

impl std::fmt::Debug for ClipVisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipVisionModel")
            .field("dimension", &self.dimension)
            .finish()
    }
}

impl ClipVisionModel {
    /// Create new CLIP Vision model instance with specified dimension
    ///
    /// # Arguments
    /// * `dimension` - Embedding dimension: 512 for ViT-Base-Patch32, 768 for ViT-Large-Patch14-336
    ///
    /// Uses lazy loading - model loaded on-demand via huggingface_file()
    pub fn new(dimension: usize) -> Result<Self, String> {
        if dimension != 512 && dimension != 768 {
            return Err(format!(
                "Unsupported dimension {}. CLIP supports 512 (Base) or 768 (Large)",
                dimension
            ));
        }
        Ok(Self { dimension })
    }
}

impl CandleModel for ClipVisionModel {
    fn info(&self) -> &'static CandleModelInfo {
        match self.dimension {
            512 => &CLIP_VISION_BASE_INFO,
            768 => &CLIP_VISION_LARGE_INFO,
            _ => unreachable!("Dimension validated in constructor"),
        }
    }
}

/// Loaded CLIP Vision model for repeated inference with no I/O overhead
///
/// Pattern: Arc<ClipModel> (no Mutex) - ClipModel::get_image_features() takes &self
/// Reference: src/capability/text_embedding/bert.rs (LoadedBertModel)
///
/// This struct holds a pre-loaded CLIP model in memory for efficient repeated inference.
/// Unlike ClipVisionModel which uses lazy loading, this loads the model once during
/// construction and reuses it for all subsequent embedding calls.
#[derive(Clone)]
pub struct LoadedClipVisionModel {
    pub(crate) model: Arc<ClipModel>,
    pub(crate) device: Device,
    pub(crate) config: ClipConfig,
    pub(crate) dimension: usize,
}

impl std::fmt::Debug for LoadedClipVisionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedClipVisionModel")
            .field("device", &self.device)
            .field("dimension", &self.dimension)
            .field("model", &"Arc<ClipModel>")
            .finish()
    }
}

impl LoadedClipVisionModel {
    /// Load CLIP model once for repeated inference
    ///
    /// This method downloads the model weights and loads them into memory.
    /// The loaded model can then be used for multiple inference calls without
    /// reloading, significantly improving performance for repeated use.
    pub async fn load(
        dimension: usize,
    ) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Validate dimension
        if dimension != 512 && dimension != 768 {
            return Err(format!(
                "Unsupported dimension: {}. CLIP supports 512 (Base) or 768 (Large)",
                dimension
            )
            .into());
        }

        // Get ModelInfo from base model
        let base_model = ClipVisionModel::new(dimension)
            .map_err(|e| format!("Failed to create base model: {}", e))?;
        let model_info = base_model.info();

        // Auto-detect device
        let device = crate::core::device_util::detect_best_device().unwrap_or_else(|e| {
            log::warn!("Device detection failed: {}. Using CPU.", e);
            Device::Cpu
        });

        // Download model file via huggingface_file
        let model_path = base_model
            .huggingface_file(model_info.registry_key, "model.safetensors")
            .await?;

        // Build CLIP config using base model's method
        let (text_config, vision_config, _) = get_configs_for_dimension(dimension);
        let config = ClipConfig {
            text_config,
            vision_config,
            logit_scale_init_value: 2.6592,
            image_size: model_info
                .image_size
                .ok_or("image_size missing from ModelInfo")? as usize,
        };

        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_path], candle_core::DType::F32, &device)?
        };

        let model = ClipModel::new(vb, &config)?;

        Ok(Self {
            model: Arc::new(model),
            device,
            config,
            dimension,
        })
    }
}

impl CandleModel for LoadedClipVisionModel {
    fn info(&self) -> &'static CandleModelInfo {
        match self.dimension {
            512 => &CLIP_VISION_BASE_INFO,
            768 => &CLIP_VISION_LARGE_INFO,
            _ => unreachable!("Dimension validated in constructor"),
        }
    }
}
