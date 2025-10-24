//! Image processing operations and conversion helpers

use super::api::ResizeFilter;

/// Image processing operations that can be queued
///
/// These operations are stored and executed in sequence during tensor conversion.
/// Private to builders module - not exposed in public API.
#[derive(Debug, Clone)]
pub(super) enum ImageOperation {
    /// Resize image to target dimensions with specified filter
    Resize {
        width: usize,
        height: usize,
        filter: ResizeFilter,
    },

    /// Normalize to [-1, 1] range (CLIP-style)
    /// Formula: (x * 2.0 / 255.0) - 1.0
    /// Reference: tmp/candle-examples/candle-examples/examples/clip/main.rs:48
    NormalizeSigned,

    /// Normalize to [0, 1] range (LLaVA-style step 1)
    /// Formula: x / 255.0
    /// Reference: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:138
    NormalizeUnsigned,

    /// Normalize with per-channel mean and standard deviation (LLaVA-style step 2)
    /// Formula: (x - mean) / std
    /// Reference: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:142-146
    NormalizeWithParams { mean: [f32; 3], std: [f32; 3] },

    /// Clamp values to range
    Clamp { min: f32, max: f32 },
}

/// Convert ResizeFilter to image crate FilterType
///
/// Maps our ResizeFilter enum to image::imageops::FilterType:
/// - Triangle: CLIP models (fast, good quality)
/// - CatmullRom: Stable Diffusion, LLaVA (high quality, smooth)
/// - Nearest: Fast preview (low quality)
/// - Lanczos3: Maximum quality (slower)
///
/// References:
/// - CLIP: tmp/candle-examples/candle-examples/examples/clip/main.rs:39
/// - LLaVA: tmp/candle-examples/candle-examples/examples/llava/image_processor.rs:114
/// - SD: tmp/candle-examples/candle-examples/examples/stable-diffusion/main.rs
pub(super) fn convert_filter(filter: ResizeFilter) -> image::imageops::FilterType {
    match filter {
        ResizeFilter::Triangle => image::imageops::FilterType::Triangle,
        ResizeFilter::CatmullRom => image::imageops::FilterType::CatmullRom,
        ResizeFilter::Nearest => image::imageops::FilterType::Nearest,
        ResizeFilter::Lanczos3 => image::imageops::FilterType::Lanczos3,
    }
}
