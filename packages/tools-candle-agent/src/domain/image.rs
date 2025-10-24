use candle_core::{Device, Tensor};
use serde::{Deserialize, Serialize};

/// Image structure for storing image data and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// The image data as a string (base64, URL, or raw data)
    pub data: String,
    /// Optional format specification for the image content
    pub format: Option<ContentFormat>,
    /// Optional media type classification for the image
    pub media_type: Option<ImageMediaType>,
    /// Optional detail level specification for image processing
    pub detail: Option<ImageDetail>,
}

/// Tensor-based image representation
///
/// Wraps a Candle tensor with metadata about its format and device location.
/// Used throughout the image processing pipeline to ensure correct dimension
/// ordering and device placement.
#[derive(Debug, Clone)]
pub struct ImageTensor {
    /// The underlying Candle tensor containing image data
    /// Typically F32 dtype with shape matching the format
    pub tensor: Tensor,

    /// Dimension format of the tensor (CHW or HWC)
    /// Must be tracked to ensure correct permutation operations
    pub format: TensorFormat,

    /// Device where tensor is located (CPU, CUDA, Metal)
    /// Must match for tensor operations
    pub device: Device,
}

/// Content format enum specifying how image data is provided
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ContentFormat {
    /// Base64 encoded image data
    Base64,
    /// Image accessible via URL
    Url,
    /// Raw binary image data
    Raw,
}

/// Image media type enum for classifying image formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImageMediaType {
    /// Portable Network Graphics format
    PNG,
    /// JPEG image format
    JPEG,
    /// Graphics Interchange Format
    GIF,
    /// WebP image format
    WEBP,
    /// Scalable Vector Graphics format
    SVG,
}

/// Image detail level enum for specifying processing quality
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    /// Low detail/quality processing
    Low,
    /// High detail/quality processing
    High,
    /// Automatic detail level selection
    Auto,
}

/// Tensor dimension format for images
///
/// Tracks the ordering of dimensions in image tensors:
/// - CHW: Candle's native format (Channel, Height, Width)
/// - HWC: image crate's format (Height, Width, Channel)
///
/// Conversion via permute: HWC→CHW uses (2, 0, 1), CHW→HWC uses (1, 2, 0)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TensorFormat {
    /// Channel-Height-Width (Candle default)
    /// Shape: (C, H, W) - e.g., (3, 224, 224) for RGB 224x224
    CHW,

    /// Height-Width-Channel (image crate format)
    /// Shape: (H, W, C) - e.g., (224, 224, 3) for RGB 224x224
    HWC,
}

// Builder implementations moved to cyrup/src/builders/image.rs
