//! Download functionality for external resources
//!
//! This module provides downloading and encoding with efficient buffering.
//!
//! ## Architecture
//!
//! The module provides simple async download functions:
//! - `download_css_async()` - Download CSS content
//! - `download_and_encode_image_async()` - Download and encode images as base64
//! - `download_svg_async()` - Download SVG content
//!
//! These functions focus solely on downloading and encoding. Rate limiting is handled
//! by the orchestration layer in `core.rs` using the `crawl_engine::rate_limiter` module.
//!
//! ## Usage
//!
//! ```ignore
//! use citescrape::inline_css::downloaders::*;
//! use reqwest::Client;
//!
//! let client = Client::new();
//! let config = InlineConfig::default();
//!
//! // Simple async download
//! let css = download_css_async(
//!     "https://example.com/style.css".to_string(),
//!     client,
//!     &config
//! ).await?;
//! ```

use anyhow::{Context, Result};
use base64::Engine;
use futures::StreamExt;
use reqwest::Client;

/// Configuration for download timeouts and size limits
#[derive(Debug, Clone)]
pub struct InlineConfig {
    /// Timeout for CSS downloads
    pub css_timeout: std::time::Duration,
    /// Timeout for image downloads
    pub image_timeout: std::time::Duration,
    /// Timeout for SVG downloads
    pub svg_timeout: std::time::Duration,

    /// Maximum size for CSS downloads (bytes)
    /// Based on 99th percentile of real-world CSS + margin
    /// Typical: 50-200KB, Large frameworks: 500KB-1MB
    pub max_css_size: usize,

    /// Maximum size for image downloads (bytes)
    /// Images larger than this should not be inlined as data URLs
    /// Typical inlined images: 10-500KB, Large: 1-3MB
    pub max_image_size: usize,

    /// Maximum size for SVG downloads (bytes)
    /// SVGs are text-based and should be small
    /// Typical: 5-50KB, Complex: 100-500KB
    pub max_svg_size: usize,
}

impl Default for InlineConfig {
    fn default() -> Self {
        Self {
            css_timeout: std::time::Duration::from_secs(30),
            image_timeout: std::time::Duration::from_secs(60),
            svg_timeout: std::time::Duration::from_secs(30),

            // Reasonable defaults based on real-world usage
            max_css_size: 2 * 1024 * 1024,   // 2MB (down from 10MB)
            max_image_size: 5 * 1024 * 1024, // 5MB (down from 50MB)
            max_svg_size: 1024 * 1024,       // 1MB (down from 5MB)
        }
    }
}

// ============================================================================
// CORE DOWNLOAD IMPLEMENTATIONS (Shared Logic)
// ============================================================================

/// Core CSS download implementation
///
/// Handles HTTP download with streaming, size limits, and timeout.
/// Called by the public async API.
async fn download_css_core(url: String, client: Client, config: &InlineConfig) -> Result<String> {
    // Download with timeout
    let response = client
        .get(&url)
        .timeout(config.css_timeout)
        .send()
        .await
        .context("Failed to download CSS")?;

    // Check status
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "CSS download failed with status: {}",
            response.status()
        ));
    }

    // Get expected size and enforce limit BEFORE downloading
    let expected_size = response.content_length().unwrap_or(0);
    if expected_size > config.max_css_size as u64 {
        return Err(anyhow::anyhow!(
            "CSS file too large: {} bytes exceeds limit of {} bytes",
            expected_size,
            config.max_css_size
        ));
    }

    // Pre-allocate buffer based on Content-Length
    let mut buffer = if expected_size > 0 {
        Vec::with_capacity(expected_size as usize)
    } else {
        Vec::new()
    };

    // Stream response with size checking (second line of defense)
    let mut stream = response.bytes_stream();
    let mut total_size = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.context("Failed to read CSS chunk")?;

        // Check BEFORE accumulating
        let new_total = total_size + chunk.len();
        if new_total > config.max_css_size {
            return Err(anyhow::anyhow!(
                "CSS download exceeded size limit during download: {} bytes (max: {})",
                new_total,
                config.max_css_size
            ));
        }

        buffer.extend_from_slice(&chunk);
        total_size = new_total;
    }

    // Convert buffer to string
    String::from_utf8(buffer).context("CSS content is not valid UTF-8")
}

/// Core image download and encoding implementation
///
/// Handles HTTP download with streaming, size limits, and timeout.
/// Encodes to base64 data URL or returns original URL if too large.
/// Called by the public async API.
async fn download_and_encode_image_core(
    url: String,
    client: Client,
    config: &InlineConfig,
    max_inline_size_bytes: Option<usize>,
) -> Result<String> {
    // Download image with timeout
    let response = client
        .get(&url)
        .timeout(config.image_timeout)
        .send()
        .await
        .context("Failed to download image")?;

    // Check status
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Image download failed with status: {}",
            response.status()
        ));
    }

    // Get content type
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    // Get expected size and enforce limit BEFORE downloading
    let expected_size = response.content_length().unwrap_or(0);
    if expected_size > config.max_image_size as u64 {
        return Err(anyhow::anyhow!(
            "Image too large: {} bytes exceeds limit of {} bytes",
            expected_size,
            config.max_image_size
        ));
    }

    // Pre-allocate buffer based on Content-Length
    let mut buffer = if expected_size > 0 {
        Vec::with_capacity(expected_size as usize)
    } else {
        Vec::new()
    };

    // Stream response with size checking (second line of defense)
    let mut stream = response.bytes_stream();
    let mut total_size = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.context("Failed to read image chunk")?;

        // Check BEFORE accumulating
        let new_total = total_size + chunk.len();
        if new_total > config.max_image_size {
            return Err(anyhow::anyhow!(
                "Image download exceeded size limit during download: {} bytes (max: {})",
                new_total,
                config.max_image_size
            ));
        }

        buffer.extend_from_slice(&chunk);
        total_size = new_total;
    }

    // Check if image size exceeds the threshold
    if let Some(max_size) = max_inline_size_bytes
        && buffer.len() > max_size
    {
        // Return the original URL instead of encoding
        log::debug!(
            "Image size ({} bytes) exceeds max_inline_size_bytes ({} bytes), keeping as external URL: {}",
            buffer.len(),
            max_size,
            url
        );
        return Ok(url);
    }

    // Encode to base64 directly from buffer (implements AsRef<[u8]>)
    let encoded_capacity = base64::encoded_len(buffer.len(), false).unwrap_or(0);
    let mut encoded = String::with_capacity(encoded_capacity + 30 + content_type.len());

    encoded.push_str("data:");
    encoded.push_str(&content_type);
    encoded.push_str(";base64,");

    // Use STANDARD encoding for better compatibility
    base64::engine::general_purpose::STANDARD.encode_string(&buffer, &mut encoded);

    Ok(encoded)
}

/// Core SVG download implementation
///
/// Handles HTTP download with streaming, size limits, and timeout.
/// Cleans up SVG content by removing XML declarations and commenting DOCTYPE.
/// Called by the public async API.
async fn download_svg_core(url: String, client: Client, config: &InlineConfig) -> Result<String> {
    // Download with timeout
    let response = client
        .get(&url)
        .timeout(config.svg_timeout)
        .send()
        .await
        .context("Failed to download SVG")?;

    // Check status
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "SVG download failed with status: {}",
            response.status()
        ));
    }

    // Get expected size and enforce limit BEFORE downloading
    let expected_size = response.content_length().unwrap_or(0);
    if expected_size > config.max_svg_size as u64 {
        return Err(anyhow::anyhow!(
            "SVG too large: {} bytes exceeds limit of {} bytes",
            expected_size,
            config.max_svg_size
        ));
    }

    // Pre-allocate buffer based on Content-Length
    let mut buffer = if expected_size > 0 {
        Vec::with_capacity(expected_size as usize)
    } else {
        Vec::new()
    };

    // Stream response with size checking (second line of defense)
    let mut stream = response.bytes_stream();
    let mut total_size = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.context("Failed to read SVG chunk")?;

        // Check BEFORE accumulating
        let new_total = total_size + chunk.len();
        if new_total > config.max_svg_size {
            return Err(anyhow::anyhow!(
                "SVG download exceeded size limit during download: {} bytes (max: {})",
                new_total,
                config.max_svg_size
            ));
        }

        buffer.extend_from_slice(&chunk);
        total_size = new_total;
    }

    // Convert buffer to string
    let text = String::from_utf8(buffer).context("SVG content is not valid UTF-8")?;

    // Clean up SVG for inline usage
    let mut cleaned = text;

    // Remove XML declaration
    cleaned = cleaned.replace("<?xml version=\"1.0\" encoding=\"UTF-8\"?>", "");

    // Comment out DOCTYPE if present
    if let Some(doctype_start) = cleaned.find("<!DOCTYPE svg") {
        // Find the closing '>' of the DOCTYPE specifically
        if let Some(doctype_end_offset) = cleaned[doctype_start..].find('>') {
            let doctype_end = doctype_start + doctype_end_offset + 1;

            // Extract the DOCTYPE
            let doctype = &cleaned[doctype_start..doctype_end];

            // Replace with commented version
            let commented = format!("<!--{doctype}-->");
            cleaned.replace_range(doctype_start..doctype_end, &commented);
        }
    }

    Ok(cleaned)
}

// ============================================================================
// PUBLIC API FUNCTIONS
// ============================================================================

/// Async version: Download CSS content
#[inline]
pub async fn download_css_async(
    url: String,
    client: Client,
    config: &InlineConfig,
) -> Result<String> {
    download_css_core(url, client, config).await
}

/// Async version: Download and encode image as base64 data URL
///
/// If `max_inline_size_bytes` is set and the image exceeds this size,
/// the original URL will be returned instead of a base64-encoded data URL.
#[inline]
pub async fn download_and_encode_image_async(
    url: String,
    client: Client,
    config: &InlineConfig,
    max_inline_size_bytes: Option<usize>,
) -> Result<String> {
    download_and_encode_image_core(url, client, config, max_inline_size_bytes).await
}

/// Async version: Download SVG content
#[inline]
pub async fn download_svg_async(
    url: String,
    client: Client,
    config: &InlineConfig,
) -> Result<String> {
    download_svg_core(url, client, config).await
}
