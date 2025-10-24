//! Helper functions for LoadedClipVisionModel encoding operations
//!
//! This module provides utility functions used by the LoadedClipVisionModel
//! trait implementations. These functions are designed to work within
//! spawn_blocking contexts for CPU-intensive operations.

use super::preprocessing::PreprocessingConfig;
use crate::builders::image::{ImageBuilder, ResizeFilter};
use candle_core::{Device, Tensor};
use candle_transformers::models::clip::ClipModel;

/// Synchronous encoding helper for spawn_blocking contexts
///
/// Performs image preprocessing and model inference synchronously.
/// Used by LoadedClipVisionModel trait implementations.
pub fn encode_image_sync(
    image_builder: impl crate::builders::image::ImageBuilder,
    config: &PreprocessingConfig,
    device: &Device,
    model: &ClipModel,
) -> Result<Vec<f32>, String> {
    // Image preprocessing (CPU-intensive): load, resize, normalize
    let image_tensor = image_builder
        .resize(config.image_size, config.image_size, ResizeFilter::Triangle)
        .normalize_unsigned()
        .normalize_with(config.image_mean, config.image_std)
        .to_tensor_sync(device)?;

    // Add batch dimension (CPU-intensive)
    let batched = image_tensor
        .unsqueeze(0)
        .map_err(|e| format!("Failed to add batch dimension: {}", e))?;

    // Model inference (CPU-intensive)
    let features = model
        .get_image_features(&batched)
        .map_err(|e| format!("CLIP encoding failed: {}", e))?;

    // Tensor conversion (CPU-intensive)
    features
        .to_vec1::<f32>()
        .map_err(|e| format!("Failed to convert to vec: {}", e))
}

/// Synchronous batch encoding helper for spawn_blocking contexts
///
/// Performs batch image preprocessing and model inference synchronously.
/// Used by LoadedClipVisionModel batch_embed_images implementation.
pub fn encode_batch_sync(
    paths: &[String],
    config: &PreprocessingConfig,
    device: &Device,
    model: &ClipModel,
) -> Result<Vec<Vec<f32>>, String> {
    use crate::domain::image::Image;

    // Preprocess all images (CPU-intensive)
    let mut tensors = Vec::with_capacity(paths.len());
    for path in paths {
        let image_builder = Image::from_path(path)
            .resize(config.image_size, config.image_size, ResizeFilter::Triangle)
            .normalize_unsigned()
            .normalize_with(config.image_mean, config.image_std);

        let tensor = image_builder
            .to_tensor_sync(device)
            .map_err(|e| format!("Image preprocessing failed for {}: {}", path, e))?;

        tensors.push(tensor);
    }

    // Stack into batch (CPU-intensive)
    let batched =
        Tensor::stack(&tensors, 0).map_err(|e| format!("Failed to batch tensors: {}", e))?;

    // Model inference (CPU-intensive)
    let features = model
        .get_image_features(&batched)
        .map_err(|e| format!("Batch CLIP encoding failed: {}", e))?;

    // Convert to Vec<Vec<f32>> (CPU-intensive)
    let batch_size = features
        .dim(0)
        .map_err(|e| format!("Failed to get batch size: {}", e))?;

    let mut embeddings = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        let row = features
            .get(i)
            .and_then(|t| t.to_vec1::<f32>())
            .map_err(|e| format!("Failed to extract embedding {}: {}", i, e))?;
        embeddings.push(row);
    }

    Ok(embeddings)
}
