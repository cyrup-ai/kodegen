//! CSS and resource inlining core functionality
//!
//! This module provides resource inlining with efficient buffering.

use super::downloaders::InlineConfig;
use crate::page_extractor::schema::ResourceInfo;
use anyhow::Result;
use futures::future::join_all;
use futures::join;
use reqwest::Client;

/// Resource type for error tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Css,
    Image,
    Svg,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::Css => write!(f, "CSS"),
            ResourceType::Image => write!(f, "Image"),
            ResourceType::Svg => write!(f, "SVG"),
        }
    }
}

/// Type alias for resource download future
type ResourceFuture = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<(String, String, ResourceType), InliningError>>
            + Send,
    >,
>;

/// Error information for a failed resource download
#[derive(Debug, Clone)]
pub struct InliningError {
    pub url: String,
    pub resource_type: ResourceType,
    pub error: String,
}

/// Result of resource inlining with success and failure tracking
#[derive(Debug, Clone)]
pub struct InliningResult {
    pub html: String,
    pub successes: usize,
    pub failures: Vec<InliningError>,
}

impl InliningResult {
    /// Total number of resources processed
    #[must_use]
    pub fn total(&self) -> usize {
        self.successes + self.failures.len()
    }

    /// Check if any failures occurred
    #[must_use]
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    /// Get failure rate as a ratio between 0.0 and 1.0
    #[must_use]
    pub fn failure_rate(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.failures.len() as f64 / total as f64
        }
    }
}

/// Inline all external resources with a single HTML parse
///
/// This function parses the HTML document once and extracts all resource information
/// synchronously, then processes the resources asynchronously. This eliminates the
/// performance cost of parsing the same HTML document three separate times.
///
/// Returns `InliningResult` containing the processed HTML along with success/failure metrics.
#[inline]
pub async fn inline_all_resources(
    html_content: String,
    base_url: String,
    config: &InlineConfig,
    max_inline_image_size_bytes: Option<usize>,
    rate_rps: Option<f64>,
) -> Result<InliningResult> {
    let config = config.clone();

    // Parse HTML once and extract all resource information synchronously
    // This eliminates redundant parsing and prepares for concurrent downloads
    let (css_links, images, svgs, extraction_failures) = {
        let document = scraper::Html::parse_document(&html_content);
        let (css, css_failures) = super::processors::extract_css_links(&document, &base_url)?;
        let (imgs, img_failures) = super::processors::extract_images(&document, &base_url)?;
        let (svg_list, svg_failures) = super::processors::extract_svgs(&document, &base_url)?;

        // Collect all extraction failures
        let mut all_extraction_failures = Vec::new();
        all_extraction_failures.extend(css_failures);
        all_extraction_failures.extend(img_failures);
        all_extraction_failures.extend(svg_failures);

        (css, imgs, svg_list, all_extraction_failures)
        // document is dropped here, safe to proceed with async operations
    };

    // Client already uses Arc internally, so just clone it
    let client = Client::new();

    // Process all resource types concurrently using futures::join!
    // Each download task runs in parallel, eliminating sequential bottleneck
    let (css_downloads, img_downloads, svg_downloads) = join!(
        download_all_css(css_links, client.clone(), &config, rate_rps),
        download_all_images(
            images,
            client.clone(),
            &config,
            max_inline_image_size_bytes,
            rate_rps
        ),
        download_all_svgs(svgs, client, &config, rate_rps)
    );

    // Destructure results to get successes and failures
    let (css_replacements, css_failures) = css_downloads?;
    let (image_replacements, image_failures) = img_downloads?;
    let (svg_replacements, svg_failures) = svg_downloads?;

    // Collect all failures (extraction + download)
    let mut all_failures = Vec::new();
    all_failures.extend(extraction_failures);
    all_failures.extend(css_failures);
    all_failures.extend(image_failures);
    all_failures.extend(svg_failures);

    // Count successes
    let successes = css_replacements.len() + image_replacements.len() + svg_replacements.len();

    // Apply all replacements in a single DOM parse/serialize cycle
    // This eliminates the performance bottleneck of parsing/serializing 3 times
    let html = super::utils::apply_all_replacements(
        html_content,
        css_replacements,
        image_replacements,
        svg_replacements,
    )?;

    Ok(InliningResult {
        html,
        successes,
        failures: all_failures,
    })
}

/// Download all CSS files concurrently
/// Returns tuple of (successes, failures) for error tracking
async fn download_all_css(
    css_links: Vec<(String, String)>,
    client: Client,
    config: &InlineConfig,
    rate_rps: Option<f64>,
) -> Result<(Vec<(String, String)>, Vec<InliningError>)> {
    // Create futures for concurrent execution
    let futures = css_links.into_iter().map(|(css_url, href)| {
        let client = client.clone();
        let config = config.clone();
        let css_url_for_error = css_url.clone();

        async move {
            // Apply rate limiting if configured
            if let Some(rate) = rate_rps {
                match crate::crawl_engine::rate_limiter::check_http_rate_limit(&css_url, rate).await
                {
                    crate::crawl_engine::rate_limiter::RateLimitDecision::Deny { .. } => {
                        let error_msg = format!("Rate limited: {css_url}");
                        log::debug!("{error_msg}");
                        return Err(InliningError {
                            url: css_url_for_error,
                            resource_type: ResourceType::Css,
                            error: error_msg,
                        });
                    }
                    crate::crawl_engine::rate_limiter::RateLimitDecision::Allow => {
                        // Proceed with download
                    }
                }
            }

            match super::downloaders::download_css_async(css_url, client, &config).await {
                Ok(css_content) => Ok((href, css_content)),
                Err(e) => {
                    let error_msg = e.to_string();
                    log::warn!("Failed to download CSS from {css_url_for_error}: {error_msg}");
                    Err(InliningError {
                        url: css_url_for_error,
                        resource_type: ResourceType::Css,
                        error: error_msg,
                    })
                }
            }
        }
    });

    // Execute all downloads concurrently
    let download_results = join_all(futures).await;

    // Partition into successes and failures
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for result in download_results {
        match result {
            Ok((href, content)) => results.push((href, content)),
            Err(error) => failures.push(error),
        }
    }

    Ok((results, failures))
}

/// Download all images concurrently
/// Returns tuple of (successes, failures) for error tracking
async fn download_all_images(
    images: Vec<(String, String)>,
    client: Client,
    config: &InlineConfig,
    max_inline_size_bytes: Option<usize>,
    rate_rps: Option<f64>,
) -> Result<(Vec<(String, String)>, Vec<InliningError>)> {
    // Create futures for concurrent execution
    let futures = images.into_iter().map(|(image_url, src)| {
        let client = client.clone();
        let config = config.clone();
        let image_url_for_error = image_url.clone();

        async move {
            // Apply rate limiting if configured
            if let Some(rate) = rate_rps {
                match crate::crawl_engine::rate_limiter::check_http_rate_limit(&image_url, rate)
                    .await
                {
                    crate::crawl_engine::rate_limiter::RateLimitDecision::Deny { .. } => {
                        let error_msg = format!("Rate limited: {image_url}");
                        log::debug!("{error_msg}");
                        return Err(InliningError {
                            url: image_url_for_error,
                            resource_type: ResourceType::Image,
                            error: error_msg,
                        });
                    }
                    crate::crawl_engine::rate_limiter::RateLimitDecision::Allow => {
                        // Proceed with download
                    }
                }
            }

            match super::downloaders::download_and_encode_image_async(
                image_url,
                client,
                &config,
                max_inline_size_bytes,
            )
            .await
            {
                Ok(data_url) => Ok((src, data_url)),
                Err(e) => {
                    let error_msg = e.to_string();
                    log::warn!("Failed to download image from {image_url_for_error}: {error_msg}");
                    Err(InliningError {
                        url: image_url_for_error,
                        resource_type: ResourceType::Image,
                        error: error_msg,
                    })
                }
            }
        }
    });

    // Execute all downloads concurrently
    let download_results = join_all(futures).await;

    // Partition into successes and failures
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for result in download_results {
        match result {
            Ok((src, content)) => results.push((src, content)),
            Err(error) => failures.push(error),
        }
    }

    Ok((results, failures))
}

/// Download all SVGs concurrently
/// Returns tuple of (successes, failures) for error tracking
async fn download_all_svgs(
    svgs: Vec<(String, String)>,
    client: Client,
    config: &InlineConfig,
    rate_rps: Option<f64>,
) -> Result<(Vec<(String, String)>, Vec<InliningError>)> {
    // Create futures for concurrent execution
    let futures = svgs.into_iter().map(|(svg_url, src)| {
        let client = client.clone();
        let config = config.clone();
        let svg_url_for_error = svg_url.clone();

        async move {
            // Apply rate limiting if configured
            if let Some(rate) = rate_rps {
                match crate::crawl_engine::rate_limiter::check_http_rate_limit(&svg_url, rate).await
                {
                    crate::crawl_engine::rate_limiter::RateLimitDecision::Deny { .. } => {
                        let error_msg = format!("Rate limited: {svg_url}");
                        log::debug!("{error_msg}");
                        return Err(InliningError {
                            url: svg_url_for_error,
                            resource_type: ResourceType::Svg,
                            error: error_msg,
                        });
                    }
                    crate::crawl_engine::rate_limiter::RateLimitDecision::Allow => {
                        // Proceed with download
                    }
                }
            }

            match super::downloaders::download_svg_async(svg_url, client, &config).await {
                Ok(svg_content) => Ok((src, svg_content)),
                Err(e) => {
                    let error_msg = e.to_string();
                    log::warn!("Failed to download SVG from {svg_url_for_error}: {error_msg}");
                    Err(InliningError {
                        url: svg_url_for_error,
                        resource_type: ResourceType::Svg,
                        error: error_msg,
                    })
                }
            }
        }
    });

    // Execute all downloads concurrently
    let download_results = join_all(futures).await;

    // Partition into successes and failures
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for result in download_results {
        match result {
            Ok((src, content)) => results.push((src, content)),
            Err(error) => failures.push(error),
        }
    }

    Ok((results, failures))
}

/// Download and inline external resources using extracted resource information
/// with concurrent downloads for maximum performance
///
/// Returns `InliningResult` containing the processed HTML along with success/failure metrics.
#[inline]
pub async fn inline_resources_from_info(
    html_content: String,
    base_url: String,
    config: &InlineConfig,
    resources: ResourceInfo,
    max_inline_image_size_bytes: Option<usize>,
    rate_rps: Option<f64>,
) -> Result<InliningResult> {
    let config = config.clone();
    let client = Client::new();
    let html = html_content;

    log::debug!("Starting to inline resources for base_url: {base_url}");
    log::debug!(
        "Found {} stylesheets to process",
        resources.stylesheets.len()
    );
    log::debug!("Found {} images to process", resources.images.len());

    // Collect all futures (don't await yet!)
    // Each future returns either Ok((url, content)) or the InliningError on failure
    let mut futures: Vec<ResourceFuture> = Vec::new();

    // Collect stylesheet futures
    for stylesheet in &resources.stylesheets {
        if !stylesheet.inline
            && let Some(ref href) = stylesheet.url
        {
            let base = base_url.clone();
            let href_clone = href.clone();
            let client_clone = client.clone();
            let config_clone = config.clone();
            let rate_rps_clone = rate_rps;

            let future = Box::pin(async move {
                let css_url = match super::utils::resolve_url(&base, &href_clone) {
                    Ok(url) => url,
                    Err(e) => {
                        return Err(InliningError {
                            url: href_clone,
                            resource_type: ResourceType::Css,
                            error: e.to_string(),
                        });
                    }
                };

                // Apply rate limiting if configured
                if let Some(rate) = rate_rps_clone {
                    match crate::crawl_engine::rate_limiter::check_http_rate_limit(&css_url, rate)
                        .await
                    {
                        crate::crawl_engine::rate_limiter::RateLimitDecision::Deny { .. } => {
                            let error_msg = format!("Rate limited: {css_url}");
                            log::debug!("{error_msg}");
                            return Err(InliningError {
                                url: css_url,
                                resource_type: ResourceType::Css,
                                error: error_msg,
                            });
                        }
                        crate::crawl_engine::rate_limiter::RateLimitDecision::Allow => {}
                    }
                }

                log::debug!("Processing CSS: {href_clone} -> {css_url}");
                match super::downloaders::download_css_async(
                    css_url.clone(),
                    client_clone,
                    &config_clone,
                )
                .await
                {
                    Ok(content) => {
                        log::debug!("Downloaded CSS content length: {} chars", content.len());
                        log::info!("Successfully downloaded CSS from: {css_url}");
                        Ok((href_clone, content, ResourceType::Css))
                    }
                    Err(e) => Err(InliningError {
                        url: css_url,
                        resource_type: ResourceType::Css,
                        error: e.to_string(),
                    }),
                }
            });

            futures.push(future);
        }
    }

    // Collect image and SVG futures
    for image in &resources.images {
        let base = base_url.clone();
        let src = image.url.clone();
        let client_clone = client.clone();
        let config_clone = config.clone();
        let rate_rps_clone = rate_rps;

        // Check if this is an SVG based on the URL
        let is_svg = src.to_lowercase().contains(".svg");

        if is_svg {
            // Process as SVG
            let future = Box::pin(async move {
                let svg_url = match super::utils::resolve_url(&base, &src) {
                    Ok(url) => url,
                    Err(e) => {
                        return Err(InliningError {
                            url: src,
                            resource_type: ResourceType::Svg,
                            error: e.to_string(),
                        });
                    }
                };

                // Apply rate limiting if configured
                if let Some(rate) = rate_rps_clone {
                    match crate::crawl_engine::rate_limiter::check_http_rate_limit(&svg_url, rate)
                        .await
                    {
                        crate::crawl_engine::rate_limiter::RateLimitDecision::Deny { .. } => {
                            let error_msg = format!("Rate limited: {svg_url}");
                            log::debug!("{error_msg}");
                            return Err(InliningError {
                                url: svg_url,
                                resource_type: ResourceType::Svg,
                                error: error_msg,
                            });
                        }
                        crate::crawl_engine::rate_limiter::RateLimitDecision::Allow => {}
                    }
                }

                log::debug!("Processing SVG: {src} -> {svg_url}");
                match super::downloaders::download_svg_async(
                    svg_url.clone(),
                    client_clone,
                    &config_clone,
                )
                .await
                {
                    Ok(svg_content) => {
                        log::debug!("Successfully downloaded SVG: {svg_url}");
                        Ok((src, svg_content, ResourceType::Svg))
                    }
                    Err(e) => Err(InliningError {
                        url: svg_url,
                        resource_type: ResourceType::Svg,
                        error: e.to_string(),
                    }),
                }
            });

            futures.push(future);
        } else {
            // Process as regular image
            let future = Box::pin(async move {
                let image_url = match super::utils::resolve_url(&base, &src) {
                    Ok(url) => url,
                    Err(e) => {
                        return Err(InliningError {
                            url: src,
                            resource_type: ResourceType::Image,
                            error: e.to_string(),
                        });
                    }
                };

                // Apply rate limiting if configured
                if let Some(rate) = rate_rps_clone {
                    match crate::crawl_engine::rate_limiter::check_http_rate_limit(&image_url, rate)
                        .await
                    {
                        crate::crawl_engine::rate_limiter::RateLimitDecision::Deny { .. } => {
                            let error_msg = format!("Rate limited: {image_url}");
                            log::debug!("{error_msg}");
                            return Err(InliningError {
                                url: image_url,
                                resource_type: ResourceType::Image,
                                error: error_msg,
                            });
                        }
                        crate::crawl_engine::rate_limiter::RateLimitDecision::Allow => {}
                    }
                }

                log::debug!("Processing image: {src} -> {image_url}");
                match super::downloaders::download_and_encode_image_async(
                    image_url.clone(),
                    client_clone,
                    &config_clone,
                    max_inline_image_size_bytes,
                )
                .await
                {
                    Ok(data_url) => {
                        log::debug!("Successfully downloaded image: {image_url}");
                        Ok((src, data_url, ResourceType::Image))
                    }
                    Err(e) => Err(InliningError {
                        url: image_url,
                        resource_type: ResourceType::Image,
                        error: e.to_string(),
                    }),
                }
            });

            futures.push(future);
        }
    }

    // Execute ALL downloads concurrently
    let results = join_all(futures).await;

    // Separate CSS, image, and SVG replacements and collect failures
    let mut css_replacements = Vec::new();
    let mut image_replacements = Vec::new();
    let mut svg_replacements = Vec::new();
    let mut failures = Vec::new();

    // Process results and apply replacements
    for result in results {
        match result {
            Ok((original_url, content, resource_type)) => match resource_type {
                ResourceType::Css => {
                    css_replacements.push((original_url, content));
                }
                ResourceType::Image => {
                    image_replacements.push((original_url, content));
                }
                ResourceType::Svg => {
                    svg_replacements.push((original_url, content));
                }
            },
            Err(inlining_error) => {
                log::warn!(
                    "Failed to download {} from {}: {}",
                    inlining_error.resource_type,
                    inlining_error.url,
                    inlining_error.error
                );
                failures.push(inlining_error);
            }
        }
    }

    // Count successes
    let successes = css_replacements.len() + image_replacements.len() + svg_replacements.len();

    // Apply all replacements in a single DOM parse/serialize cycle
    // This eliminates the performance bottleneck of parsing/serializing 3 times
    let html = super::utils::apply_all_replacements(
        html,
        css_replacements,
        image_replacements,
        svg_replacements,
    )?;

    Ok(InliningResult {
        html,
        successes,
        failures,
    })
}
