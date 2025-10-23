//! Page enhancement functionality for improved crawling
//!
//! This module provides functions to enhance browser pages with
//! stealth features and performance optimizations.

use anyhow::Result;
use chromiumoxide::{Page, cdp};

/// Enhance a page with stealth features and optimizations
pub async fn enhance_page(page: Page) -> Result<()> {
    // Apply elite kromekover stealth features
    match crate::kromekover::inject(page.clone()).await {
        Ok(()) => {
            log::debug!("Kromekover stealth evasions injected successfully");
        }
        Err(e) => {
            log::warn!("Failed to inject kromekover stealth: {e}");
            // Continue anyway - stealth failure shouldn't block enhancement
        }
    }

    // Disable images for faster loading (optional)
    // page.set_extra_http_headers(headers).await?;

    // Set viewport to 1920x1080 for consistent desktop rendering
    page.execute(
        cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
            .width(1920)
            .height(1080)
            .device_scale_factor(1.0)
            .mobile(false)
            .build()
            .map_err(anyhow::Error::msg)?,
    )
    .await?;

    Ok(())
}
