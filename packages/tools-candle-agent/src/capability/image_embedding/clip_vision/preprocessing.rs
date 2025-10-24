//! Shared image preprocessing utilities for CLIP Vision encoding
//!
//! This module provides deduplication helpers that consolidate the repeated
//! preprocessing logic from all encoding methods. The only variation between
//! encoding methods is the image source (path/url/base64), while the resize,
//! normalization, and batching steps are identical.
//!
//! This module eliminates ~400 lines of code duplication across the original file.

use crate::builders::image::ResizeFilter;
use crate::domain::model::CandleModelInfo;
use candle_core::{Device, Tensor};

/// Configuration extracted from ModelInfo for preprocessing
pub struct PreprocessingConfig {
    pub image_size: usize,
    pub image_mean: [f32; 3],
    pub image_std: [f32; 3],
}

/// Extract preprocessing config from ModelInfo
///
/// Gets image_size, image_mean, and image_std from ModelInfo,
/// returning error if any required field is missing.
pub fn extract_config(info: &CandleModelInfo) -> Result<PreprocessingConfig, String> {
    let image_size = info.image_size.ok_or("image_size missing from ModelInfo")? as usize;
    let image_mean = info.image_mean.ok_or("image_mean missing from ModelInfo")?;
    let image_std = info.image_std.ok_or("image_std missing from ModelInfo")?;

    Ok(PreprocessingConfig {
        image_size,
        image_mean,
        image_std,
    })
}

/// Generic image preprocessing pipeline (async version)
///
/// This consolidates the duplicated preprocessing code from all encoding methods.
/// The only variation is the image source (path/url/base64), which is provided
/// as an ImageBuilder implementation.
///
/// Pipeline:
/// 1. Resize to target size with Triangle filter
/// 2. normalize_unsigned(): [0, 255] → [0, 1]
/// 3. normalize_with(mean, std): (x - mean) / std
/// 4. to_tensor(): Convert to Tensor
/// 5. unsqueeze(0): Add batch dimension (C,H,W) → (1,C,H,W)
pub async fn preprocess_image(
    image_builder: impl crate::builders::image::ImageBuilder,
    config: &PreprocessingConfig,
    device: &Device,
) -> Result<Tensor, String> {
    let image_tensor = image_builder
        .resize(config.image_size, config.image_size, ResizeFilter::Triangle)
        .normalize_unsigned() // Step 1: [0, 255] → [0, 1]
        .normalize_with(config.image_mean, config.image_std) // Step 2: (x - mean) / std
        .to_tensor(device)
        .await?;

    // Add batch dimension: (C,H,W) → (1,C,H,W)
    image_tensor
        .unsqueeze(0)
        .map_err(|e| format!("Failed to add batch dimension: {}", e))
}
