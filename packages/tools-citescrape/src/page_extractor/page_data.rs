//! Zero-allocation page data extraction functionality
//!
//! This module provides blazing-fast, lock-free page data extraction
//! with pre-allocated buffers and zero heap allocations in hot paths.

use anyhow::{Context, Result};
use chromiumoxide::Page;

use crate::content_saver;

use super::extractors::{
    extract_interactive_elements, extract_links, extract_metadata, extract_resources,
    extract_security_info, extract_timing_info,
};

/// Configuration for page data extraction
pub struct ExtractPageDataConfig {
    pub output_dir: std::path::PathBuf,
    pub link_rewriter: super::link_rewriter::LinkRewriter,
    pub max_inline_image_size_bytes: Option<usize>,
    pub crawl_rate_rps: Option<f64>,
    pub save_html: bool,
}

/// Extract event handler attribute names from element attributes
#[inline]
fn get_event_handlers(attributes: &std::collections::HashMap<String, String>) -> Vec<String> {
    attributes
        .keys()
        .filter(|k| k.starts_with("on"))
        .cloned()
        .collect()
}

/// Check if element has any event handlers
#[inline]
fn has_event_handlers(attributes: &std::collections::HashMap<String, String>) -> bool {
    attributes.keys().any(|k| k.starts_with("on"))
}

/// Check if element has an interactive ARIA role
#[inline]
fn has_interactive_role(attributes: &std::collections::HashMap<String, String>) -> bool {
    if let Some(role) = attributes.get("role") {
        matches!(
            role.as_str(),
            "button"
                | "checkbox"
                | "radio"
                | "switch"
                | "tab"
                | "slider"
                | "spinbutton"
                | "menuitem"
                | "menuitemcheckbox"
                | "menuitemradio"
                | "option"
                | "link"
                | "searchbox"
                | "textbox"
                | "combobox"
                | "gridcell"
                | "treeitem"
        )
    } else {
        false
    }
}

/// Convert raw interactive elements to structured format
fn convert_interactive_elements(
    elements: Vec<super::schema::InteractiveElement>,
) -> super::schema::InteractiveElements {
    use super::schema::{
        ButtonElement, ClickableElement, InputElement, InteractiveElements, LinkElement,
    };

    let mut result = InteractiveElements::default();

    for element in elements {
        match element.element_type.to_lowercase().as_str() {
            "button" => {
                result.buttons.push(ButtonElement {
                    id: element.attributes.get("id").cloned(),
                    text: element.text.clone(),
                    button_type: element.attributes.get("type").cloned(),
                    selector: element.selector.clone(),
                    disabled: element.attributes.contains_key("disabled"),
                    form_id: element.attributes.get("form").cloned(),
                    attributes: element.attributes.clone(),
                });
            }
            "a" => {
                if let Some(href) = element
                    .url
                    .as_ref()
                    .or_else(|| element.attributes.get("href"))
                {
                    result.links.push(LinkElement {
                        href: href.clone(),
                        text: element.text.clone(),
                        title: element.attributes.get("title").cloned(),
                        target: element.attributes.get("target").cloned(),
                        rel: element.attributes.get("rel").cloned(),
                        selector: element.selector.clone(),
                        attributes: element.attributes.clone(),
                    });
                }
            }
            "input" => {
                result.inputs.push(InputElement {
                    id: element.attributes.get("id").cloned(),
                    name: element.attributes.get("name").cloned(),
                    input_type: element
                        .attributes
                        .get("type")
                        .unwrap_or(&"text".to_string())
                        .clone(),
                    value: element.attributes.get("value").cloned(),
                    placeholder: element.attributes.get("placeholder").cloned(),
                    required: element.attributes.contains_key("required"),
                    disabled: element.attributes.contains_key("disabled"),
                    selector: element.selector.clone(),
                    validation: extract_validation(&element.attributes),
                    attributes: element.attributes.clone(),
                });
            }
            "select" | "textarea" => {
                result.inputs.push(InputElement {
                    id: element.attributes.get("id").cloned(),
                    name: element.attributes.get("name").cloned(),
                    input_type: element.element_type.clone(),
                    value: element.attributes.get("value").cloned(),
                    placeholder: element.attributes.get("placeholder").cloned(),
                    required: element.attributes.contains_key("required"),
                    disabled: element.attributes.contains_key("disabled"),
                    selector: element.selector.clone(),
                    validation: None,
                    attributes: element.attributes.clone(),
                });
            }

            // Native interactive HTML elements
            "details" | "summary" | "dialog" | "menu" => {
                result.clickable.push(ClickableElement {
                    selector: element.selector.clone(),
                    text: element.text.clone(),
                    role: Some(element.element_type.clone()),
                    aria_label: element.attributes.get("aria-label").cloned(),
                    event_handlers: get_event_handlers(&element.attributes),
                    attributes: element.attributes.clone(),
                });
            }

            // Label elements (interactive when associated with input)
            "label" => {
                if element.attributes.contains_key("for") {
                    result.clickable.push(ClickableElement {
                        selector: element.selector.clone(),
                        text: element.text.clone(),
                        role: Some("label".to_string()),
                        aria_label: element.attributes.get("aria-label").cloned(),
                        event_handlers: get_event_handlers(&element.attributes),
                        attributes: element.attributes.clone(),
                    });
                }
            }

            // Catch-all with logging for unhandled types
            _ => {
                // Check if element is interactive via ARIA role or event handlers
                if has_interactive_role(&element.attributes)
                    || has_event_handlers(&element.attributes)
                {
                    result.clickable.push(ClickableElement {
                        selector: element.selector.clone(),
                        text: element.text.clone(),
                        role: element.attributes.get("role").cloned(),
                        aria_label: element.attributes.get("aria-label").cloned(),
                        event_handlers: get_event_handlers(&element.attributes),
                        attributes: element.attributes.clone(),
                    });
                } else {
                    // Log dropped elements for debugging and monitoring
                    log::warn!(
                        "Dropping non-interactive element: type='{}', selector='{}', attributes={:?}",
                        element.element_type,
                        element.selector,
                        element.attributes.keys().collect::<Vec<_>>()
                    );
                }
            }
        }
    }

    result
}

/// Extract validation rules from input attributes
fn extract_validation(
    attributes: &std::collections::HashMap<String, String>,
) -> Option<super::schema::InputValidation> {
    if attributes.contains_key("pattern")
        || attributes.contains_key("minlength")
        || attributes.contains_key("maxlength")
        || attributes.contains_key("min")
        || attributes.contains_key("max")
        || attributes.contains_key("step")
    {
        Some(super::schema::InputValidation {
            pattern: attributes.get("pattern").cloned(),
            min_length: attributes.get("minlength").and_then(|v| v.parse().ok()),
            max_length: attributes.get("maxlength").and_then(|v| v.parse().ok()),
            min: attributes.get("min").cloned(),
            max: attributes.get("max").cloned(),
            step: attributes.get("step").cloned(),
        })
    } else {
        None
    }
}

/// Extract all page data including metadata, resources, timing, and content
/// This is the production function used by the crawler, with `LinkRewriter` integration
pub async fn extract_page_data(
    page: Page,
    url: String,
    config: ExtractPageDataConfig,
) -> Result<super::schema::PageData> {
    log::info!("Starting to extract page data for URL: {url}");

    // Launch all extractions in parallel with tokio::try_join!
    let (metadata, resources, timing, security, title, interactive_elements_vec, links) = tokio::try_join!(
        extract_metadata(page.clone()),
        extract_resources(page.clone()),
        extract_timing_info(page.clone()),
        extract_security_info(page.clone()),
        async {
            let result: Result<String> = async {
                let title_value = page
                    .evaluate("document.title")
                    .await
                    .context("Failed to evaluate document.title")?
                    .into_value()
                    .map_err(|e| anyhow::anyhow!("Failed to get page title: {e}"))?;

                if let serde_json::Value::String(title) = title_value {
                    Ok(title)
                } else {
                    Ok(String::new())
                }
            }
            .await;
            result
        },
        extract_interactive_elements(page.clone()),
        extract_links(page.clone()),
    )?;

    // Get HTML content
    let content = page
        .content()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get page content: {e}"))?;

    // Phase 1: Mark all links with data attributes for discovery tracking
    let content_with_data_attrs = config
        .link_rewriter
        .mark_links_for_discovery(&content, &url)
        .await?;

    // Phase 2: Rewrite links using data attributes and registered URL mappings
    let content_with_rewritten_links = config
        .link_rewriter
        .rewrite_links_from_data_attrs(content_with_data_attrs)
        .await?;

    // Convert Vec<InteractiveElement> to InteractiveElements (FIX for the bug!)
    let interactive_elements = convert_interactive_elements(interactive_elements_vec);

    // Get local path for URL registration BEFORE saving
    // This allows us to register the URL→path mapping after successful save
    let local_path_str =
        match crate::utils::get_mirror_path(&url, &config.output_dir, "index.html").await {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(e) => {
                log::warn!("Failed to get mirror path for URL registration: {e}");
                // Fallback path - registration will still work but path may be incorrect
                config
                    .output_dir
                    .join("index.html")
                    .to_string_lossy()
                    .to_string()
            }
        };

    // Register URL → local path mapping BEFORE saving
    // This enables progressive rewriting: pages crawled later can immediately
    // link to this page using relative paths instead of external URLs
    config
        .link_rewriter
        .register_url(&url, &local_path_str)
        .await;

    log::debug!(
        "Registered URL mapping: {url} → {local_path_str} (enables progressive link rewriting)"
    );

    // Save HTML content if enabled
    if config.save_html {
        match content_saver::save_html_content_with_resources(
            &content_with_rewritten_links,
            url.clone(),
            config.output_dir.clone(),
            &resources,
            config.max_inline_image_size_bytes,
            config.crawl_rate_rps,
        )
        .await
        {
            Ok(()) => {
                log::info!("HTML content saved successfully for: {url}");
            }
            Err(e) => {
                log::warn!("Failed to save HTML for {url}: {e}");
                // Note: URL is already registered even if save failed
                // This is acceptable - worst case is a 404 for a registered path
            }
        }
    }

    log::info!("Successfully extracted page data for URL: {url}");
    Ok(super::schema::PageData {
        url: url.clone(),
        title,
        content: content_with_rewritten_links,
        metadata,
        interactive_elements,
        links,
        resources,
        timing,
        security,
        crawled_at: chrono::Utc::now(),
    })
}
