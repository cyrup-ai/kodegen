//! Image builder public API - trait definition and related types

use std::pin::Pin;

use crate::domain::context::CandleDocumentChunk as ImageChunk;
use crate::domain::image::{ContentFormat, ImageDetail, ImageMediaType};
use tokio_stream::Stream;

/// Image builder trait - elegant zero-allocation builder pattern
pub trait ImageBuilder: Sized {
    /// Set format - EXACT syntax: .format(ContentFormat::Base64)
    fn format(self, format: ContentFormat) -> Self;

    /// Set media type - EXACT syntax: .media_type(ImageMediaType::PNG)
    fn media_type(self, media_type: ImageMediaType) -> Self;

    /// Set detail - EXACT syntax: .detail(ImageDetail::High)
    fn detail(self, detail: ImageDetail) -> Self;

    /// Set PNG format - EXACT syntax: .with_png()
    fn with_png(self) -> Self;

    /// Set JPEG format - EXACT syntax: .with_jpeg()
    fn with_jpeg(self) -> Self;

    /// Set high detail - EXACT syntax: .high_detail()
    fn high_detail(self) -> Self;

    /// Set low detail - EXACT syntax: .low_detail()
    fn low_detail(self) -> Self;

    /// Resize image - EXACT syntax: .resize(width, height, filter)
    ///
    /// All vision models require specific input dimensions:
    /// - CLIP: 224×224 (ViT-B), 336×336 (ViT-L)
    /// - LLaVA: 336×336 (default)
    /// - Stable Diffusion: 512×512, 768×768, 1024×1024
    fn resize(self, width: usize, height: usize, filter: ResizeFilter) -> Self;

    /// Normalize to range [-1, 1] - EXACT syntax: .normalize_signed()
    ///
    /// Formula: (x * 2.0 / 255.0) - 1.0
    /// Reference: tmp/candle-examples/candle-examples/examples/clip/main.rs:48
    ///
    /// Used by:
    /// - CLIP and OpenCLIP models
    /// - MobileCLIP
    /// - Chinese CLIP
    fn normalize_signed(self) -> Self;

    /// Normalize to range [0, 1] - EXACT syntax: .normalize_unsigned()
    ///
    /// Formula: x / 255.0
    /// Reference: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:138
    ///
    /// Used by:
    /// - VAE decoders (Stable Diffusion output)
    /// - Visualization and output processing
    /// - Some detection models
    fn normalize_unsigned(self) -> Self;

    /// Normalize with mean/std per channel - EXACT syntax: .normalize_with(mean, std)
    ///
    /// Formula: (tensor - mean) / std
    /// Reference: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:142-146
    /// Implementation: tensor.broadcast_sub(&mean)?.broadcast_div(&std)
    ///
    /// ImageNet standard values:
    /// - Mean: [0.48145466, 0.4578275, 0.40821073]
    /// - Std:  [0.26862954, 0.2613026, 0.2757771]
    ///
    /// Used by:
    /// - LLaVA and vision-language models
    /// - ResNet-based models
    /// - Vision Transformers (ViT)
    /// - Any ImageNet pre-trained model
    fn normalize_with(self, mean: [f32; 3], std: [f32; 3]) -> Self;

    /// Clamp values to range - EXACT syntax: .clamp(min, max)
    ///
    /// Ensures all tensor values are within [min, max] range.
    /// Common ranges:
    /// - [0.0, 1.0] for output processing
    /// - [-1.0, 1.0] for normalized inputs
    ///
    /// Used in:
    /// - Stable Diffusion VAE output
    /// - Tensor sanitization before model input
    /// - Output post-processing
    fn clamp(self, min: f32, max: f32) -> Self;

    /// Convert to Candle tensor - EXACT syntax: .to_tensor(device)
    ///
    /// This executes all queued operations in sequence and returns the final tensor.
    /// The pipeline is:
    /// 1. Load image from source
    /// 2. Apply resize (if queued)
    /// 3. Apply normalization (if queued)
    /// 4. Apply clamp (if queued)
    /// 5. Convert HWC → CHW format
    /// 6. Move to target device
    ///
    /// Returns Result<Tensor, String> wrapped in Future for async execution.
    fn to_tensor(
        self,
        device: &candle_core::Device,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<candle_core::Tensor, String>> + Send + '_>,
    >;

    /// Synchronous tensor creation for use in spawn_blocking contexts - EXACT syntax: .to_tensor_sync(device)
    ///
    /// This method performs the same operations as `to_tensor()` but without
    /// async wrapping, making it suitable for use inside `spawn_blocking`.
    ///
    /// Executes the complete image processing pipeline:
    /// 1. Load image from source (base64/URL/path)
    /// 2. Apply image-level operations (resize, RGB conversion)
    /// 3. Convert image to tensor (HWC→CHW, u8→f32)
    /// 4. Apply tensor-level operations (normalize, clamp)
    /// 5. Transfer to target device
    ///
    /// Returns Result<Tensor, String> synchronously.
    fn to_tensor_sync(self, device: &candle_core::Device) -> Result<candle_core::Tensor, String>;

    /// Set error handler - EXACT syntax: .on_error(|error| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_error<F>(self, handler: F) -> impl ImageBuilder
    where
        F: Fn(String) + Send + Sync + 'static;

    /// Set chunk handler - EXACT syntax: .on_chunk(|chunk| { ... })
    /// Zero-allocation: uses generic function pointer instead of Box<dyn>
    fn on_chunk<F>(self, handler: F) -> impl ImageBuilder
    where
        F: FnMut(ImageChunk) -> ImageChunk + Send + 'static;

    /// Load image - EXACT syntax: .load()
    fn load(self) -> Pin<Box<dyn Stream<Item = ImageChunk> + Send>>;

    /// Process image - EXACT syntax: .process(|chunk| { ... })
    fn process<F>(self, f: F) -> Pin<Box<dyn Stream<Item = ImageChunk> + Send>>
    where
        F: FnOnce(ImageChunk) -> ImageChunk + Send + 'static;
}

/// Image resize filter types matching image crate filters
///
/// Maps to image::imageops::FilterType for actual resize operations.
/// Different models use different filters for optimal quality:
/// - Triangle: CLIP models (fast, good quality)
/// - CatmullRom: Stable Diffusion, LLaVA (high quality, smooth)
/// - Nearest: Fast preview (low quality)
/// - Lanczos3: Maximum quality (slower)
#[derive(Debug, Clone, Copy)]
pub enum ResizeFilter {
    /// Triangle filter - used by CLIP models
    /// Reference: tmp/candle-examples/candle-examples/examples/clip/main.rs:42
    Triangle,

    /// Catmull-Rom filter - used by Stable Diffusion and LLaVA
    /// Reference: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:105
    CatmullRom,

    /// Nearest neighbor - fast, low quality
    Nearest,

    /// Lanczos3 - high quality, slower
    Lanczos3,
}
