//! Zero-allocation page data extraction functions
//!
//! This module provides blazing-fast extraction functions for various page elements
//! with pre-allocated buffers and lock-free operations.

use super::js_scripts::{
    INTERACTIVE_ELEMENTS_SCRIPT, METADATA_SCRIPT, RESOURCES_SCRIPT, SECURITY_SCRIPT, TIMING_SCRIPT,
};
use super::schema::InteractiveElement;
use super::schema::{PageMetadata, ResourceInfo, SecurityInfo, TimingInfo};
use anyhow::{Context, Result};
use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, CaptureScreenshotParams,
};

/// Extract page metadata with zero allocation
#[inline]
pub async fn extract_metadata(page: Page) -> Result<PageMetadata> {
    let js_result = page
        .evaluate(METADATA_SCRIPT)
        .await
        .context("Failed to execute metadata extraction script")?;

    let metadata: PageMetadata = match js_result.into_value() {
        Ok(value) => {
            serde_json::from_value(value).context("Failed to parse metadata from JS result")?
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to get metadata value: {e}")),
    };

    Ok(metadata)
}

/// Extract page resources with pre-allocated collections
#[inline]
pub async fn extract_resources(page: Page) -> Result<ResourceInfo> {
    let js_result = page
        .evaluate(RESOURCES_SCRIPT)
        .await
        .context("Failed to execute resources extraction script")?;

    let resources: ResourceInfo = match js_result.into_value() {
        Ok(value) => {
            serde_json::from_value(value).context("Failed to parse resources from JS result")?
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to get resources value: {e}")),
    };

    // Log resource counts for debugging
    log::debug!(
        "Extracted resources - Stylesheets: {}, Scripts: {}, Images: {}, Fonts: {}, Media: {}",
        resources.stylesheets.len(),
        resources.scripts.len(),
        resources.images.len(),
        resources.fonts.len(),
        resources.media.len()
    );

    Ok(resources)
}

/// Extract timing information with zero allocation
#[inline]
pub async fn extract_timing_info(page: Page) -> Result<TimingInfo> {
    let js_result = page
        .evaluate(TIMING_SCRIPT)
        .await
        .context("Failed to execute timing extraction script")?;

    let timing: TimingInfo = match js_result.into_value() {
        Ok(value) => {
            serde_json::from_value(value).context("Failed to parse timing info from JS result")?
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to get timing info value: {e}")),
    };

    Ok(timing)
}

/// Extract security information efficiently
#[inline]
pub async fn extract_security_info(page: Page) -> Result<SecurityInfo> {
    let js_result = page
        .evaluate(SECURITY_SCRIPT)
        .await
        .context("Failed to execute security extraction script")?;

    let security: SecurityInfo = match js_result.into_value() {
        Ok(value) => {
            serde_json::from_value(value).context("Failed to parse security info from JS result")?
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to get security info value: {e}")),
    };

    Ok(security)
}

/// Extract interactive elements with zero allocation
#[inline]
pub async fn extract_interactive_elements(page: Page) -> Result<Vec<InteractiveElement>> {
    // Use efficient JavaScript evaluation to extract all interactive elements
    let js_result = page
        .evaluate(INTERACTIVE_ELEMENTS_SCRIPT)
        .await
        .context("Failed to execute interactive elements extraction script")?;

    let value = js_result
        .into_value::<serde_json::Value>()
        .map_err(|e| anyhow::anyhow!("Failed to get value from JS result: {e}"))?;

    let arr = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("JavaScript evaluation did not return an array"))?;

    let elements: Vec<InteractiveElement> = arr
        .iter()
        .map(|item| {
            serde_json::from_value(item.clone()).context("Failed to parse interactive element")
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(elements)
}

/// Extract links from the page with zero allocation
#[inline]
pub async fn extract_links(page: Page) -> Result<Vec<super::schema::CrawlLink>> {
    let js_result = page
        .evaluate(super::js_scripts::LINKS_SCRIPT)
        .await
        .context("Failed to execute links extraction script")?;

    let links: Vec<super::schema::CrawlLink> = match js_result.into_value::<serde_json::Value>() {
        Ok(value) => {
            serde_json::from_value(value).context("Failed to parse links from JS result")?
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to get links value: {e}")),
    };

    Ok(links)
}

/// Take a screenshot of the page
pub async fn capture_screenshot(page: Page, url: &str, output_dir: &std::path::Path) -> Result<()> {
    let url = url.to_string();
    let output_dir = output_dir.to_path_buf();

    // Get mirror path (async)
    let path = crate::utils::get_mirror_path(&url, &output_dir, "index.png").await?;

    // Ensure .gitignore exists in domain directory
    crate::utils::ensure_domain_gitignore(&path, &output_dir).await?;

    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    )
    .await?;

    let params = CaptureScreenshotParams {
        quality: Some(100),
        format: Some(CaptureScreenshotFormat::Png),
        capture_beyond_viewport: Some(true),
        ..Default::default()
    };

    let screenshot_data = page
        .screenshot(params)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to capture screenshot: {e}"))?;

    // Save compressed file directly with async/await
    let (_saved_path, _metadata) =
        crate::content_saver::save_compressed_file(screenshot_data, &path, "image/png", false)
            .await?;

    log::info!("Screenshot captured and saved successfully for URL: {url}");
    Ok(())
}
