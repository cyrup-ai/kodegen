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

// Builder implementations moved to cyrup/src/builders/image.rs
