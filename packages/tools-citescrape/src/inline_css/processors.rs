//! Processors for CSS and resource inlining
//!
//! This module provides processing functions for different resource types.

use anyhow::Result;
use lazy_static::lazy_static;
use reqwest::Client;
use scraper::{Html, Selector};

use super::core::{InliningError, InliningResult, ResourceType};
use super::downloaders::{
    InlineConfig, download_and_encode_image_async, download_css_async, download_svg_async,
};
use super::utils::resolve_url;

/// Type alias for extraction results (urls, failures)
type ExtractionResult = Result<(Vec<(String, String)>, Vec<InliningError>)>;

lazy_static! {
    // These selectors are hardcoded and syntactically valid CSS selectors.
    // If they fail to parse, it indicates a compile-time bug in the selector strings.
    static ref CSS_LINK_SELECTOR: Selector =
        Selector::parse("link[rel=\"stylesheet\"]")
            .expect("BUG: hardcoded CSS selector 'link[rel=\"stylesheet\"]' is invalid - this is a compile-time bug");

    static ref IMG_SELECTOR: Selector =
        Selector::parse("img[src]")
            .expect("BUG: hardcoded CSS selector 'img[src]' is invalid - this is a compile-time bug");

    static ref SVG_SELECTOR: Selector =
        Selector::parse("img[src*=\".svg\"]")
            .expect("BUG: hardcoded CSS selector 'img[src*=\".svg\"]' is invalid - this is a compile-time bug");
}

/// Extract CSS link information from parsed HTML (synchronous, no async)
/// Returns tuple of (successful URLs, failures) for error tracking
pub fn extract_css_links(document: &Html, base_url: &str) -> ExtractionResult {
    let mut replacements = Vec::new();
    let mut failures = Vec::new();

    for element in document.select(&CSS_LINK_SELECTOR) {
        if let Some(href) = element.value().attr("href") {
            match resolve_url(base_url, href) {
                Ok(css_url) => {
                    // Return href for DOM-based replacement
                    replacements.push((css_url, href.to_string()));
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    log::warn!("Failed to resolve CSS URL {href}: {error_msg}");
                    failures.push(InliningError {
                        url: href.to_string(),
                        resource_type: ResourceType::Css,
                        error: error_msg,
                    });
                }
            }
        }
    }
    Ok((replacements, failures))
}

/// Extract image information from parsed HTML (synchronous, no async)
/// Returns tuple of (successful URLs, failures) for error tracking
pub fn extract_images(document: &Html, base_url: &str) -> ExtractionResult {
    let mut replacements = Vec::new();
    let mut failures = Vec::new();

    for element in document.select(&IMG_SELECTOR) {
        if let Some(src) = element.value().attr("src") {
            // Skip data URLs that are already inlined
            if src.starts_with("data:") {
                continue;
            }

            match resolve_url(base_url, src) {
                Ok(image_url) => {
                    replacements.push((image_url, src.to_string()));
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    log::warn!("Failed to resolve image URL {src}: {error_msg}");
                    failures.push(InliningError {
                        url: src.to_string(),
                        resource_type: ResourceType::Image,
                        error: error_msg,
                    });
                }
            }
        }
    }
    Ok((replacements, failures))
}

/// Extract SVG information from parsed HTML (synchronous, no async)
/// Returns tuple of (successful URLs, failures) for error tracking
pub fn extract_svgs(document: &Html, base_url: &str) -> ExtractionResult {
    let mut replacements = Vec::new();
    let mut failures = Vec::new();

    for element in document.select(&SVG_SELECTOR) {
        if let Some(src) = element.value().attr("src") {
            // Skip data URLs that are already inlined
            if src.starts_with("data:") {
                continue;
            }

            match resolve_url(base_url, src) {
                Ok(svg_url) => {
                    // Return src for DOM-based replacement
                    replacements.push((svg_url, src.to_string()));
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    log::warn!("Failed to resolve SVG URL {src}: {error_msg}");
                    failures.push(InliningError {
                        url: src.to_string(),
                        resource_type: ResourceType::Svg,
                        error: error_msg,
                    });
                }
            }
        }
    }
    Ok((replacements, failures))
}

/// Internal helper: Process CSS links using a pre-parsed HTML document
///
/// This function accepts a reference to an already-parsed HTML document to avoid
/// redundant parsing when processing multiple resource types on the same HTML.
///
/// Returns `InliningResult` containing the processed HTML along with success/failure metrics.
pub async fn process_css_links_internal(
    document: &Html,
    html: String,
    base_url: String,
    client: Client,
    config: &InlineConfig,
) -> Result<InliningResult> {
    // Extract CSS URLs from the provided document
    let mut failures = Vec::new();
    let css_urls = {
        let mut urls = Vec::new();

        for element in document.select(&CSS_LINK_SELECTOR) {
            if let Some(href) = element.value().attr("href") {
                match resolve_url(&base_url, href) {
                    Ok(css_url) => {
                        urls.push((css_url, href.to_string()));
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        log::warn!("Failed to resolve CSS URL {href}: {error_msg}");
                        failures.push(InliningError {
                            url: href.to_string(),
                            resource_type: ResourceType::Css,
                            error: error_msg,
                        });
                    }
                }
            }
        }
        Ok::<Vec<(String, String)>, anyhow::Error>(urls)
    }?;

    // Download all CSS content and collect replacements
    let mut css_replacements = Vec::new();
    for (css_url, href) in css_urls {
        let css_url_for_error = css_url.clone();

        match download_css_async(css_url, client.clone(), config).await {
            Ok(css_content) => {
                css_replacements.push((href, css_content));
            }
            Err(e) => {
                let error_msg = e.to_string();
                log::warn!("Failed to download CSS from {css_url_for_error}: {error_msg}");
                failures.push(InliningError {
                    url: css_url_for_error,
                    resource_type: ResourceType::Css,
                    error: error_msg,
                });
            }
        }
    }

    let successes = css_replacements.len();

    // Apply all CSS replacements using DOM manipulation
    let processed_html = if css_replacements.is_empty() {
        html
    } else {
        super::utils::replace_css_links_with_styles(html, css_replacements)?
    };

    Ok(InliningResult {
        html: processed_html,
        successes,
        failures,
    })
}

/// Process CSS links in HTML content
///
/// This is the public API function that accepts a pre-parsed HTML document to avoid
/// redundant parsing when processing multiple resource types on the same HTML.
///
/// **Performance Note**: Callers should parse HTML once using `Html::parse_document(&html)`
/// and pass the same document reference to all processor functions to eliminate redundant
/// parsing overhead.
///
/// # Parameters
/// * `document` - Reference to a pre-parsed HTML document
/// * `html` - The original HTML string (needed for applying replacements)
/// * `base_url` - Base URL for resolving relative URLs
/// * `client` - HTTP client for downloading resources
///
/// Returns `InliningResult` containing the processed HTML along with success/failure metrics.
pub async fn process_css_links(
    document: &Html,
    html: String,
    base_url: String,
    client: Client,
) -> Result<InliningResult> {
    let config = InlineConfig::default();
    process_css_links_internal(document, html, base_url, client, &config).await
}

/// Internal helper: Process images using a pre-parsed HTML document
///
/// This function accepts a reference to an already-parsed HTML document to avoid
/// redundant parsing when processing multiple resource types on the same HTML.
///
/// Returns `InliningResult` containing the processed HTML along with success/failure metrics.
pub async fn process_images_internal(
    document: &Html,
    html: String,
    base_url: String,
    client: Client,
    config: &InlineConfig,
) -> Result<InliningResult> {
    // Extract image URLs from the provided document
    let mut failures = Vec::new();
    let image_urls = {
        let mut urls = Vec::new();

        for element in document.select(&IMG_SELECTOR) {
            if let Some(src) = element.value().attr("src") {
                // Skip data URLs that are already inlined
                if src.starts_with("data:") {
                    continue;
                }

                match resolve_url(&base_url, src) {
                    Ok(image_url) => {
                        urls.push((image_url, src.to_string()));
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        log::warn!("Failed to resolve image URL {src}: {error_msg}");
                        failures.push(InliningError {
                            url: src.to_string(),
                            resource_type: ResourceType::Image,
                            error: error_msg,
                        });
                    }
                }
            }
        }
        Ok::<Vec<(String, String)>, anyhow::Error>(urls)
    }?;

    // Download all images and collect replacements
    let mut image_replacements = Vec::new();
    for (image_url, src) in image_urls {
        let image_url_for_error = image_url.clone();

        match download_and_encode_image_async(image_url, client.clone(), config, None).await {
            Ok(data_url) => {
                image_replacements.push((src, data_url));
            }
            Err(e) => {
                let error_msg = e.to_string();
                log::warn!("Failed to download image from {image_url_for_error}: {error_msg}");
                failures.push(InliningError {
                    url: image_url_for_error,
                    resource_type: ResourceType::Image,
                    error: error_msg,
                });
            }
        }
    }

    let successes = image_replacements.len();

    // Apply all image replacements using DOM manipulation
    let processed_html = if image_replacements.is_empty() {
        html
    } else {
        super::utils::replace_image_sources(html, image_replacements)?
    };

    Ok(InliningResult {
        html: processed_html,
        successes,
        failures,
    })
}

/// Process images in HTML content
///
/// This is the public API function that accepts a pre-parsed HTML document to avoid
/// redundant parsing when processing multiple resource types on the same HTML.
///
/// **Performance Note**: Callers should parse HTML once using `Html::parse_document(&html)`
/// and pass the same document reference to all processor functions to eliminate redundant
/// parsing overhead.
///
/// # Parameters
/// * `document` - Reference to a pre-parsed HTML document
/// * `html` - The original HTML string (needed for applying replacements)
/// * `base_url` - Base URL for resolving relative URLs
/// * `client` - HTTP client for downloading resources
///
/// Returns `InliningResult` containing the processed HTML along with success/failure metrics.
pub async fn process_images(
    document: &Html,
    html: String,
    base_url: String,
    client: Client,
) -> Result<InliningResult> {
    let config = InlineConfig::default();
    process_images_internal(document, html, base_url, client, &config).await
}

/// Internal helper: Process SVG images using a pre-parsed HTML document
///
/// This function accepts a reference to an already-parsed HTML document to avoid
/// redundant parsing when processing multiple resource types on the same HTML.
///
/// Returns `InliningResult` containing the processed HTML along with success/failure metrics.
pub async fn process_svgs_internal(
    document: &Html,
    html: String,
    base_url: String,
    client: Client,
    config: &InlineConfig,
) -> Result<InliningResult> {
    // Extract SVG URLs from the provided document
    let mut failures = Vec::new();
    let svg_urls = {
        let mut urls = Vec::new();

        for element in document.select(&SVG_SELECTOR) {
            if let Some(src) = element.value().attr("src") {
                // Skip data URLs that are already inlined
                if src.starts_with("data:") {
                    continue;
                }

                match resolve_url(&base_url, src) {
                    Ok(svg_url) => {
                        urls.push((svg_url, src.to_string()));
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        log::warn!("Failed to resolve SVG URL {src}: {error_msg}");
                        failures.push(InliningError {
                            url: src.to_string(),
                            resource_type: ResourceType::Svg,
                            error: error_msg,
                        });
                    }
                }
            }
        }
        Ok::<Vec<(String, String)>, anyhow::Error>(urls)
    }?;

    // Download all SVGs and collect replacements
    let mut svg_replacements = Vec::new();
    for (svg_url, src) in svg_urls {
        let svg_url_for_error = svg_url.clone();

        match download_svg_async(svg_url, client.clone(), config).await {
            Ok(svg_content) => {
                svg_replacements.push((src, svg_content));
            }
            Err(e) => {
                let error_msg = e.to_string();
                log::warn!("Failed to download SVG from {svg_url_for_error}: {error_msg}");
                failures.push(InliningError {
                    url: svg_url_for_error,
                    resource_type: ResourceType::Svg,
                    error: error_msg,
                });
            }
        }
    }

    let successes = svg_replacements.len();

    // Apply all SVG replacements using DOM manipulation
    let processed_html = if svg_replacements.is_empty() {
        html
    } else {
        super::utils::replace_img_tags_with_svg(html, svg_replacements)?
    };

    Ok(InliningResult {
        html: processed_html,
        successes,
        failures,
    })
}

/// Process SVG images in HTML content
///
/// This is the public API function that accepts a pre-parsed HTML document to avoid
/// redundant parsing when processing multiple resource types on the same HTML.
///
/// **Performance Note**: Callers should parse HTML once using `Html::parse_document(&html)`
/// and pass the same document reference to all processor functions to eliminate redundant
/// parsing overhead.
///
/// # Parameters
/// * `document` - Reference to a pre-parsed HTML document
/// * `html` - The original HTML string (needed for applying replacements)
/// * `base_url` - Base URL for resolving relative URLs
/// * `client` - HTTP client for downloading resources
///
/// Returns `InliningResult` containing the processed HTML along with success/failure metrics.
pub async fn process_svgs(
    document: &Html,
    html: String,
    base_url: String,
    client: Client,
) -> Result<InliningResult> {
    let config = InlineConfig::default();
    process_svgs_internal(document, html, base_url, client, &config).await
}
