//! Encoding methods for ClipVisionModel (lazy loading pattern)
//!
//! This module implements the encoding methods for ClipVisionModel that load
//! the model on-demand for each encoding operation. The methods use shared
//! preprocessing helpers from the preprocessing module to eliminate duplication.

use super::config::get_configs_for_dimension;
use super::models::ClipVisionModel;
use super::preprocessing::{extract_config, preprocess_image};
use crate::domain::image::Image;
use crate::domain::model::traits::CandleModel;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::clip::{ClipConfig, ClipModel};

impl ClipVisionModel {
    /// Encode image from file path to embedding vector
    ///
    /// Uses lazy loading pattern:
    /// - Gets config from ModelInfo (single source of truth)
    /// - Auto-detects device at runtime
    /// - Loads model on-demand via huggingface_file()
    /// - Uses correct CLIP normalization: normalize_unsigned() + normalize_with(mean, std)
    ///
    /// Returns projected embeddings via model.get_image_features()
    pub async fn encode_image(&self, image_path: &str) -> Result<Tensor, String> {
        // 1. GET CONFIG FROM ModelInfo - Single source of truth
        let config = extract_config(self.info())?;

        // 2. AUTO-DETECT DEVICE - Runtime decision
        let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);

        // 3. LAZY MODEL LOADING - Load model file on-demand
        let model_path = self
            .huggingface_file(self.info().registry_key, "model.safetensors")
            .await
            .map_err(|e| format!("Failed to get model file: {}", e))?;

        // 4. BUILD CLIP CONFIG - Select configs based on dimension
        let (text_config, vision_config, _) = get_configs_for_dimension(self.dimension);
        let clip_config = ClipConfig {
            text_config,
            vision_config,
            logit_scale_init_value: 2.6592,
            image_size: config.image_size,
        };

        // 5. LOAD MODEL - From huggingface_file path
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_path], candle_core::DType::F32, &device)
                .map_err(|e| format!("Failed to load model: {}", e))?
        };
        let model = ClipModel::new(vb, &clip_config)
            .map_err(|e| format!("Failed to create model: {}", e))?;

        // 6-7. PREPROCESS IMAGE & ADD BATCH DIMENSION - Use shared helper
        let batched = preprocess_image(Image::from_path(image_path), &config, &device).await?;

        // 8. ENCODE - Use get_image_features() for vision embedding
        model
            .get_image_features(&batched)
            .map_err(|e| format!("CLIP encoding failed: {}", e))
    }

    /// Encode image from URL
    pub async fn encode_url(&self, url: &str) -> Result<Tensor, String> {
        // 1. GET CONFIG FROM ModelInfo
        let config = extract_config(self.info())?;

        // 2. AUTO-DETECT DEVICE
        let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);

        // 3. LAZY MODEL LOADING
        let model_path = self
            .huggingface_file(self.info().registry_key, "model.safetensors")
            .await
            .map_err(|e| format!("Failed to get model file: {}", e))?;

        // 4. BUILD CLIP CONFIG - Select configs based on dimension
        let (text_config, vision_config, _) = get_configs_for_dimension(self.dimension);
        let clip_config = ClipConfig {
            text_config,
            vision_config,
            logit_scale_init_value: 2.6592,
            image_size: config.image_size,
        };

        // 5. LOAD MODEL
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_path], candle_core::DType::F32, &device)
                .map_err(|e| format!("Failed to load model: {}", e))?
        };
        let model = ClipModel::new(vb, &clip_config)
            .map_err(|e| format!("Failed to create model: {}", e))?;

        // 6-7. PREPROCESS IMAGE & ADD BATCH DIMENSION - Use shared helper
        let batched = preprocess_image(Image::from_url(url), &config, &device).await?;

        // 8. ENCODE
        model
            .get_image_features(&batched)
            .map_err(|e| format!("CLIP encoding failed: {}", e))
    }

    /// Encode image from base64 data (for API usage)
    pub async fn encode_base64(&self, base64_data: &str) -> Result<Tensor, String> {
        // 1. GET CONFIG FROM ModelInfo
        let config = extract_config(self.info())?;

        // 2. AUTO-DETECT DEVICE
        let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);

        // 3. LAZY MODEL LOADING
        let model_path = self
            .huggingface_file(self.info().registry_key, "model.safetensors")
            .await
            .map_err(|e| format!("Failed to get model file: {}", e))?;

        // 4. BUILD CLIP CONFIG - Select configs based on dimension
        let (text_config, vision_config, _) = get_configs_for_dimension(self.dimension);
        let clip_config = ClipConfig {
            text_config,
            vision_config,
            logit_scale_init_value: 2.6592,
            image_size: config.image_size,
        };

        // 5. LOAD MODEL
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_path], candle_core::DType::F32, &device)
                .map_err(|e| format!("Failed to load model: {}", e))?
        };
        let model = ClipModel::new(vb, &clip_config)
            .map_err(|e| format!("Failed to create model: {}", e))?;

        // 6-7. PREPROCESS IMAGE & ADD BATCH DIMENSION - Use shared helper
        let batched = preprocess_image(Image::from_base64(base64_data), &config, &device).await?;

        // 8. ENCODE
        model
            .get_image_features(&batched)
            .map_err(|e| format!("CLIP encoding failed: {}", e))
    }

    /// Encode multiple images in batch
    pub async fn encode_batch(&self, image_paths: Vec<&str>) -> Result<Tensor, String> {
        // 1. GET CONFIG FROM ModelInfo
        let config = extract_config(self.info())?;

        // 2. AUTO-DETECT DEVICE
        let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);

        // 3. LAZY MODEL LOADING
        let model_path = self
            .huggingface_file(self.info().registry_key, "model.safetensors")
            .await
            .map_err(|e| format!("Failed to get model file: {}", e))?;

        // 4. BUILD CLIP CONFIG - Select configs based on dimension
        let (text_config, vision_config, _) = get_configs_for_dimension(self.dimension);
        let clip_config = ClipConfig {
            text_config,
            vision_config,
            logit_scale_init_value: 2.6592,
            image_size: config.image_size,
        };

        // 5. LOAD MODEL
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_path], candle_core::DType::F32, &device)
                .map_err(|e| format!("Failed to load model: {}", e))?
        };
        let model = ClipModel::new(vb, &clip_config)
            .map_err(|e| format!("Failed to create model: {}", e))?;

        // 6. PREPROCESS ALL IMAGES - CORRECT normalization
        use crate::builders::image::{ImageBuilder as _, ResizeFilter};
        let mut tensors = Vec::new();
        for path in image_paths {
            let tensor = Image::from_path(path)
                .resize(config.image_size, config.image_size, ResizeFilter::Triangle)
                .normalize_unsigned()
                .normalize_with(config.image_mean, config.image_std)
                .to_tensor(&device)
                .await?;
            tensors.push(tensor);
        }

        // 7. STACK INTO BATCH: [(C,H,W), (C,H,W), ...] → (N,C,H,W)
        let batched =
            Tensor::stack(&tensors, 0).map_err(|e| format!("Failed to batch tensors: {}", e))?;

        // 8. ENCODE ENTIRE BATCH
        model
            .get_image_features(&batched)
            .map_err(|e| format!("Batch encoding failed: {}", e))
    }
}
