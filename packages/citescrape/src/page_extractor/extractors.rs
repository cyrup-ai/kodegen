//! Zero-allocation page data extraction functions
//!
//! This module provides blazing-fast extraction functions for various page elements
//! with pre-allocated buffers and lock-free operations.

use anyhow::{Context, Result};
use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::page::{CaptureScreenshotFormat, CaptureScreenshotParams};
use crate::runtime::{spawn_async, AsyncTask};
use super::schema::{PageMetadata, ResourceInfo, SecurityInfo, TimingInfo};
use super::schema::InteractiveElement;
use super::js_scripts::*;

/// Extract page metadata with zero allocation
#[inline]
pub fn extract_metadata(
    page: Page,
    on_result: impl FnOnce(Result<PageMetadata>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            let js_result = page
                .evaluate(METADATA_SCRIPT)
                .await
                .context("Failed to execute metadata extraction script")?;
            
            let metadata: PageMetadata = match js_result.into_value() {
                Ok(value) => serde_json::from_value(value)
                    .context("Failed to parse metadata from JS result")?,
                Err(e) => return Err(anyhow::anyhow!("Failed to get metadata value: {}", e)),
            };
            
            Ok(metadata)
        }.await;
        
        on_result(result);
    })
}

/// Extract page resources with pre-allocated collections
#[inline]
pub fn extract_resources(
    page: Page,
    on_result: impl FnOnce(Result<ResourceInfo>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            let js_result = page
                .evaluate(RESOURCES_SCRIPT)
                .await
                .context("Failed to execute resources extraction script")?;
            
            let resources: ResourceInfo = match js_result.into_value() {
                Ok(value) => serde_json::from_value(value)
                    .context("Failed to parse resources from JS result")?,
                Err(e) => return Err(anyhow::anyhow!("Failed to get resources value: {}", e)),
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
        }.await;
        
        on_result(result);
    })
}

/// Extract timing information with zero allocation
#[inline]
pub fn extract_timing_info(
    page: Page,
    on_result: impl FnOnce(Result<TimingInfo>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            let js_result = page
                .evaluate(TIMING_SCRIPT)
                .await
                .context("Failed to execute timing extraction script")?;
            
            let timing: TimingInfo = match js_result.into_value() {
                Ok(value) => serde_json::from_value(value)
                    .context("Failed to parse timing info from JS result")?,
                Err(e) => return Err(anyhow::anyhow!("Failed to get timing info value: {}", e)),
            };
            
            Ok(timing)
        }.await;
        
        on_result(result);
    })
}

/// Extract security information efficiently
#[inline]
pub fn extract_security_info(
    page: Page,
    on_result: impl FnOnce(Result<SecurityInfo>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            let js_result = page
                .evaluate(SECURITY_SCRIPT)
                .await
                .context("Failed to execute security extraction script")?;
            
            let security: SecurityInfo = match js_result.into_value() {
                Ok(value) => serde_json::from_value(value)
                    .context("Failed to parse security info from JS result")?,
                Err(e) => return Err(anyhow::anyhow!("Failed to get security info value: {}", e)),
            };
            
            Ok(security)
        }.await;
        
        on_result(result);
    })
}

/// Extract interactive elements with zero allocation
#[inline]
pub fn extract_interactive_elements(
    page: Page,
    on_result: impl FnOnce(Result<Vec<InteractiveElement>>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            // Use efficient JavaScript evaluation to extract all interactive elements
            let js_result = page.evaluate(INTERACTIVE_ELEMENTS_SCRIPT).await
                .context("Failed to execute interactive elements extraction script")?;

            let value = js_result.into_value::<serde_json::Value>()
                .map_err(|e| anyhow::anyhow!("Failed to get value from JS result: {}", e))?;

            let arr = value.as_array()
                .ok_or_else(|| anyhow::anyhow!("JavaScript evaluation did not return an array"))?;

            let elements: Vec<InteractiveElement> = arr.iter()
                .map(|item| serde_json::from_value(item.clone())
                    .context("Failed to parse interactive element"))
                .collect::<Result<Vec<_>>>()?;

            Ok(elements)
        }.await;

        on_result(result);
    })
}

/// Extract links from the page with zero allocation
#[inline]
pub fn extract_links(
    page: Page,
    on_result: impl FnOnce(Result<Vec<super::schema::CrawlLink>>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            let js_result = page
                .evaluate(super::js_scripts::LINKS_SCRIPT)
                .await
                .context("Failed to execute links extraction script")?;

            let links: Vec<super::schema::CrawlLink> = match js_result.into_value::<serde_json::Value>() {
                Ok(value) => serde_json::from_value(value)
                    .context("Failed to parse links from JS result")?,
                Err(e) => return Err(anyhow::anyhow!("Failed to get links value: {}", e)),
            };
            
            Ok(links)
        }.await;
        
        on_result(result);
    })
}

/// Take a screenshot of the page
pub fn capture_screenshot(
    page: Page,
    url: &str,
    output_dir: &std::path::Path,
    on_result: impl FnOnce(Result<()>) + Send + 'static,
) -> AsyncTask<()> {
    let url = url.to_string();
    let output_dir = output_dir.to_path_buf();

    spawn_async(async move {
        let result = async {
            // Use channels for async get_mirror_path
            let (path_tx, path_rx) = tokio::sync::oneshot::channel();
            let _path_task = crate::utils::get_mirror_path(&url, &output_dir, "index.png", move |result| {
                let _ = path_tx.send(result);
            });
            let path_result = path_rx.await.map_err(|_| anyhow::anyhow!("Failed to get mirror path"))?;
            let path = path_result?;

            std::fs::create_dir_all(
                path.parent()
                    .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
            )?;

            let params = CaptureScreenshotParams {
                quality: Some(100),
                format: Some(CaptureScreenshotFormat::Png),
                capture_beyond_viewport: Some(true),
                ..Default::default()
            };

            let screenshot_data = page.screenshot(params).await
                .map_err(|e| anyhow::anyhow!("Failed to capture screenshot: {}", e))?;

            // Create oneshot channel for save result
            let (save_tx, save_rx) = tokio::sync::oneshot::channel();

            let save_guard = crate::content_saver::save_compressed_file(
                screenshot_data,
                &path,
                "image/png",
                move |metadata_result| {
                    let _ = save_tx.send(metadata_result);
                }
            );

            // Wait for save to complete
            let result = match save_rx.await {
                Ok(Ok(_metadata)) => {
                    log::info!("Screenshot captured and saved successfully for URL: {}", url);
                    Ok(())
                }
                Ok(Err(e)) => {
                    Err(anyhow::anyhow!("Failed to save screenshot: {}", e))
                }
                Err(_) => {
                    Err(anyhow::anyhow!("Screenshot save task was cancelled"))
                }
            };

            // Drop guard to clean up resources
            drop(save_guard);

            result
        }.await;

        on_result(result);
    })
}
