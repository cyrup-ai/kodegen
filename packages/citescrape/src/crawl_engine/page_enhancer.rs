//! Page enhancement functionality for improved crawling
//!
//! This module provides functions to enhance browser pages with
//! stealth features and performance optimizations.

use anyhow::Result;
use chromiumoxide::{Page, cdp};

/// Enhance a page with stealth features and optimizations
pub fn enhance_page(page: Page) -> crate::runtime::AsyncTask<Result<()>> {
    use crate::runtime::spawn_async;
    
    spawn_async(async move {
        // Apply elite kromekover stealth features
        match crate::kromekover::inject(page.clone()).await {
            Ok(Ok(())) => {
                log::debug!("Kromekover stealth evasions injected successfully");
            }
            Ok(Err(e)) => {
                log::warn!("Failed to inject kromekover stealth: {}", e);
                // Continue anyway - stealth failure shouldn't block enhancement
            }
            Err(_) => {
                log::warn!("Kromekover injection task cancelled");
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
    })
}
