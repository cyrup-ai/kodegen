//! Page enhancement functionality for improved crawling
//!
//! This module provides functions to enhance browser pages with
//! stealth features and performance optimizations.

use anyhow::Result;
use chromiumoxide::{cdp, Page};
use crate::runtime::{spawn_async, AsyncTask};

/// Enhance a page with stealth features and optimizations
#[inline]
pub fn enhance_page(
    page: Page,
    on_result: impl FnOnce(Result<()>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            // Apply elite kromekover stealth features
            let (stealth_tx, stealth_rx) = tokio::sync::oneshot::channel();
            let _stealth_task = crate::kromekover::inject(page.clone(), move |stealth_result| {
                let _ = stealth_tx.send(stealth_result);
            });
            
            match stealth_rx.await {
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
        }.await;
        
        on_result(result);
    })
}