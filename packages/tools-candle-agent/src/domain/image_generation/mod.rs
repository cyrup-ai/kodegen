//! Image generation domain types for text-to-image diffusion models
//!
//! This module provides the foundational type system for image generation providers
//! (Stable Diffusion 3.5, FLUX, etc.) to implement. It defines configuration, streaming
//! chunks, and the provider trait following cyrup's AsyncStream pattern.

use candle_core::Tensor;
use cyrup_sugars::prelude::MessageChunk;
use image::DynamicImage;
use std::pin::Pin;
use tokio_stream::Stream;

/// Configuration for image generation with diffusion models
///
/// Contains all parameters needed for text-to-image generation including
/// dimensions, denoising steps, guidance scale, and optimization flags.
#[derive(Debug, Clone)]
pub struct ImageGenerationConfig {
    /// Image width in pixels
    pub width: usize,
    /// Image height in pixels
    pub height: usize,
    /// Denoising steps (`num_inference_steps`)
    pub steps: usize,
    /// CFG scale for classifier-free guidance
    pub guidance_scale: f64,
    /// Negative prompt for guidance
    pub negative_prompt: Option<String>,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
    /// Flash attention optimization (opt-in)
    pub use_flash_attn: bool,
}

impl Default for ImageGenerationConfig {
    fn default() -> Self {
        Self {
            width: 1024,         // SD3.5/FLUX native
            height: 1024,        // SD3.5 native (FLUX uses 768)
            steps: 4,            // SD3.5 Turbo / FLUX Schnell default
            guidance_scale: 3.5, // SD3.5 Turbo default
            negative_prompt: None,
            seed: None,
            use_flash_attn: false, // Opt-in optimization
        }
    }
}

/// Streaming chunks emitted during image generation
///
/// Providers emit these chunks during the generation process to enable
/// progress monitoring and live previews.
#[derive(Clone)]
pub enum ImageGenerationChunk {
    /// Progress update during denoising steps
    Step {
        /// Current step number (0-based)
        step: usize,
        /// Total steps configured
        total: usize,
        /// Intermediate latent (optional - can be zero tensor for streaming progress)
        latent: Tensor,
    },

    /// Final generated image tensor
    Complete {
        /// Final image tensor (CHW format, F32, [0-1] range)
        image: Tensor,
    },

    /// Generation error
    Error(String),
}

impl Default for ImageGenerationChunk {
    fn default() -> Self {
        Self::Error(String::new())
    }
}

impl MessageChunk for ImageGenerationChunk {
    fn bad_chunk(msg: String) -> Self {
        Self::Error(msg)
    }

    fn error(&self) -> Option<&str> {
        match self {
            Self::Error(msg) => Some(msg.as_str()),
            _ => None,
        }
    }
}

/// Provider trait for text-to-image generation models
///
/// Implementers provide text-to-image generation following the diffusion model
/// pattern (SD3.5, FLUX, etc.) with streaming support for progress monitoring.
pub trait ImageGenerationModel: Send + Sync + 'static {
    /// Generate image from text prompt
    ///
    /// Implementers should:
    /// 1. Encode prompt to embeddings
    /// 2. Run denoising loop, emitting Step chunks for progress
    /// 3. Decode final latent via VAE
    /// 4. Normalize to [0, 1] F32 range
    /// 5. Emit Complete chunk with final tensor
    ///
    /// # Arguments
    /// * `prompt` - Text prompt describing desired image
    /// * `config` - Generation parameters (size, steps, guidance, etc.)
    /// * `device` - Target device (CPU, CUDA, Metal)
    ///
    /// # Returns
    /// Stream of generation chunks (Step for progress, Complete for final image)
    fn generate(
        &self,
        prompt: &str,
        config: &ImageGenerationConfig,
        device: &candle_core::Device,
    ) -> Pin<Box<dyn Stream<Item = ImageGenerationChunk> + Send>>;

    /// Model identifier (e.g., "stable-diffusion-3.5-medium", "flux-schnell")
    fn registry_key(&self) -> &str;

    /// Recommended default steps for this model
    /// - SD3.5 Turbo: 4 steps
    /// - SD3.5 Standard: 28 steps
    /// - FLUX Schnell: 4 steps
    /// - FLUX Dev: 20-50 steps
    fn default_steps(&self) -> usize;
}

/// Convert image tensor (CHW, F32, [0-1]) to `DynamicImage` for saving
///
/// This is the standard conversion used by all Candle image generation examples.
/// Expects tensor in Candle's native CHW format with values in [0, 1] range.
///
/// # Arguments
/// * `tensor` - Image tensor (C, H, W) with F32 dtype in [0, 1] range
///
/// # Returns
/// `DynamicImage` ready for saving to disk
///
/// # Errors
/// Returns an error if:
/// - Tensor is not 3D (expected CHW format)
/// - Number of channels is not 3 (RGB required)
/// - Tensor operations fail (permute, flatten, extraction)
///
/// # Reference
/// Based on `save_image()` from candle-examples/src/lib.rs
/// See: ../../tmp/candle-examples/candle-examples/src/lib.rs
pub fn tensor_to_image(tensor: &Tensor) -> Result<DynamicImage, String> {
    // 1. Validate 3D tensor (C, H, W) - Candle's native format
    let (channels, height, width) = tensor
        .dims3()
        .map_err(|e| format!("Expected 3D CHW tensor: {e}"))?;

    if channels != 3 {
        return Err(format!("Expected RGB (3 channels), got {channels}"));
    }

    // 2. Permute CHW → HWC for image crate compatibility
    // image crate expects (Height, Width, Channel) format
    let hwc = tensor
        .permute((1, 2, 0))
        .map_err(|e| format!("Permute failed: {e}"))?;

    // 3. Flatten and extract f32 pixels
    let flat = hwc
        .flatten_all()
        .map_err(|e| format!("Flatten failed: {e}"))?;
    let pixels_f32 = flat
        .to_vec1::<f32>()
        .map_err(|e| format!("Tensor extraction failed: {e}"))?;

    // 4. Scale [0,1] → [0,255] and convert to u8
    // This matches the inverse of normalize_unsigned() from builders/image.rs
    let pixels_u8: Vec<u8> = pixels_f32
        .iter()
        .map(|&x| {
            // Clamp to [0.0, 1.0], scale to [0.0, 255.0], round to nearest integer
            let scaled = (x.clamp(0.0, 1.0) * 255.0).round();
            // Explicit bounds checking to satisfy clippy without allowing casts
            // Since we clamped and rounded, value is guaranteed to be an integer in [0.0, 255.0]
            if scaled >= 255.0 {
                255
            } else if scaled <= 0.0 {
                0
            } else {
                // We've checked that scaled is in (0.0, 255.0) range
                // APPROVED BY DAVID MAPLE on 2025-10-03
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    scaled as u8
                }
            }
        })
        .collect();

    // 5. Create RGB image from raw pixels
    let rgb = image::RgbImage::from_raw(
        width
            .try_into()
            .map_err(|_| "Image width exceeds u32::MAX")?,
        height
            .try_into()
            .map_err(|_| "Image height exceeds u32::MAX")?,
        pixels_u8,
    )
    .ok_or("Failed to create image from pixels")?;

    Ok(DynamicImage::ImageRgb8(rgb))
}
