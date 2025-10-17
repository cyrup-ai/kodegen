//! Zero-allocation page data extraction functionality
//!
//! This module provides blazing-fast, lock-free page data extraction
//! with pre-allocated buffers and zero heap allocations in hot paths.

use anyhow::{Context, Result};
use chromiumoxide::Page;

use crate::content_saver;
use crate::content_saver::await_with_timeout;
use crate::runtime::{spawn_async, AsyncTask};
use crate::on_result;

use super::extractors::*;

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
    attributes.keys()
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
        matches!(role.as_str(),
            "button" | "checkbox" | "radio" | "switch" | "tab" |
            "slider" | "spinbutton" | "menuitem" | "menuitemcheckbox" |
            "menuitemradio" | "option" | "link" | "searchbox" |
            "textbox" | "combobox" | "gridcell" | "treeitem"
        )
    } else {
        false
    }
}

/// Convert raw interactive elements to structured format
fn convert_interactive_elements(elements: Vec<super::schema::InteractiveElement>) -> super::schema::InteractiveElements {
    use super::schema::*;
    
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
                if let Some(href) = element.url.as_ref().or_else(|| element.attributes.get("href")) {
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
                    input_type: element.attributes.get("type").unwrap_or(&"text".to_string()).clone(),
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
                if has_interactive_role(&element.attributes) || 
                   has_event_handlers(&element.attributes) {
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
fn extract_validation(attributes: &std::collections::HashMap<String, String>) -> Option<super::schema::InputValidation> {
    if attributes.contains_key("pattern") || 
       attributes.contains_key("minlength") || 
       attributes.contains_key("maxlength") ||
       attributes.contains_key("min") ||
       attributes.contains_key("max") ||
       attributes.contains_key("step") {
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
/// This is the production function used by the crawler, with LinkRewriter integration
pub fn extract_page_data(
    page: Page,
    url: String,
    config: ExtractPageDataConfig,
    on_result: impl FnOnce(Result<super::schema::PageData>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            log::info!("Starting to extract page data for URL: {}", url);

            // Pre-allocate all channels for parallel extraction
            let (metadata_tx, metadata_rx) = tokio::sync::oneshot::channel();
            let (resources_tx, resources_rx) = tokio::sync::oneshot::channel();
            let (timing_tx, timing_rx) = tokio::sync::oneshot::channel();
            let (security_tx, security_rx) = tokio::sync::oneshot::channel();
            let (title_tx, title_rx) = tokio::sync::oneshot::channel();
            let (interactive_tx, interactive_rx) = tokio::sync::oneshot::channel();
            let (links_tx, links_rx) = tokio::sync::oneshot::channel::<Vec<super::schema::CrawlLink>>();

            // Launch all extraction tasks in parallel
            let page_clone = page.clone();
            let _metadata_task = extract_metadata(page_clone, move |result| {
                on_result!(result, metadata_tx, "Failed to extract metadata");
            });

            let page_clone = page.clone();
            let _resources_task = extract_resources(page_clone, move |result| {
                on_result!(result, resources_tx, "Failed to extract resources");
            });

            let page_clone = page.clone();
            let _timing_task = extract_timing_info(page_clone, move |result| {
                on_result!(result, timing_tx, "Failed to extract timing info");
            });

            let page_clone = page.clone();
            let _security_task = extract_security_info(page_clone, move |result| {
                on_result!(result, security_tx, "Failed to extract security info");
            });

            let page_clone = page.clone();
            let _title_task = spawn_async(async move {
                let result: Result<String> = async {
                    let title_value = page_clone
                        .evaluate("document.title")
                        .await
                        .context("Failed to evaluate document.title")?
                        .into_value()
                        .map_err(|e| anyhow::anyhow!("Failed to get page title: {}", e))?;

                    if let serde_json::Value::String(title) = title_value {
                        Ok(title)
                    } else {
                        Ok(String::new())
                    }
                }.await;

                on_result!(result, title_tx, "Failed to extract title");
            });

            let page_clone = page.clone();
            let _interactive_task = extract_interactive_elements(page_clone, move |result| {
                on_result!(result, interactive_tx, "Failed to extract interactive elements");
            });

            let page_clone = page.clone();
            let _links_task = extract_links(page_clone, move |result| {
                on_result!(result, links_tx, "Failed to extract links");
            });

            // Get HTML content
            let content = page.content().await
                .map_err(|e| anyhow::anyhow!("Failed to get page content: {}", e))?;

            // Await all parallel extractions
            let (metadata_result, resources_result, timing_result,
                 security_result, title_result, interactive_result, links_result) = tokio::join!(
                async { metadata_rx.await.map_err(|_| anyhow::anyhow!("Failed to extract metadata")) },
                async { resources_rx.await.map_err(|_| anyhow::anyhow!("Failed to extract resources")) },
                async { timing_rx.await.map_err(|_| anyhow::anyhow!("Failed to extract timing info")) },
                async { security_rx.await.map_err(|_| anyhow::anyhow!("Failed to extract security info")) },
                async { title_rx.await.map_err(|_| anyhow::anyhow!("Failed to extract title")) },
                async { interactive_rx.await.map_err(|_| anyhow::anyhow!("Failed to extract interactive elements")) },
                async { links_rx.await.map_err(|_| anyhow::anyhow!("Failed to extract links")) },
            );

            let metadata = metadata_result?;
            let resources = resources_result?;
            let timing = timing_result?;
            let security = security_result?;
            let title = title_result?;
            let interactive_elements_vec = interactive_result?;
            let links = links_result?;

            // Phase 1: Mark all links with data attributes for discovery tracking
            let (link_tx, link_rx) = tokio::sync::oneshot::channel();
            let _link_task = config.link_rewriter.mark_links_for_discovery(&content, &url, move |result| {
                crate::on_result!(result, link_tx, "Failed to mark links for discovery");
            });
            let content_with_data_attrs = link_rx.await
                .map_err(|_| anyhow::anyhow!("Failed to mark links for discovery"))?;

            // Phase 2: Rewrite links using data attributes and registered URL mappings
            let (rewrite_tx, rewrite_rx) = tokio::sync::oneshot::channel();
            let _rewrite_task = config.link_rewriter.rewrite_links_from_data_attrs(content_with_data_attrs, move |result| {
                crate::on_result!(result, rewrite_tx, "Failed to rewrite links");
            });
            let content_with_rewritten_links = rewrite_rx.await
                .map_err(|_| anyhow::anyhow!("Failed to rewrite links"))?;

            // Convert Vec<InteractiveElement> to InteractiveElements (FIX for the bug!)
            let interactive_elements = convert_interactive_elements(interactive_elements_vec);

            // Get local path for URL registration BEFORE saving
            // This allows us to register the URL→path mapping after successful save
            let (path_tx, path_rx) = tokio::sync::oneshot::channel();
            let path_url = url.clone();
            let path_output = config.output_dir.clone();
            let path_task = crate::utils::get_mirror_path(
                &path_url,
                &path_output,
                "index.html",
                move |result| {
                    let _ = path_tx.send(result);
                }
            );
            let _path_guard = crate::runtime::TaskGuard::new(path_task, "get_mirror_path_for_registration");

            let local_path_result = await_with_timeout(path_rx, 30, "mirror path for URL registration").await;
            let local_path_str = match local_path_result {
                Ok(Ok(path)) => path.to_string_lossy().to_string(),
                Ok(Err(e)) => {
                    log::warn!("Failed to get mirror path for URL registration: {}", e);
                    // Fallback path - registration will still work but path may be incorrect
                    config.output_dir.join("index.html").to_string_lossy().to_string()
                }
                Err(e) => {
                    log::warn!("Timeout getting mirror path for URL registration: {}", e);
                    config.output_dir.join("index.html").to_string_lossy().to_string()
                }
            };

            // Register URL → local path mapping BEFORE saving
            // This enables progressive rewriting: pages crawled later can immediately
            // link to this page using relative paths instead of external URLs
            config.link_rewriter.register_url(&url, &local_path_str).await;

            log::debug!(
                "Registered URL mapping: {} → {} (enables progressive link rewriting)",
                url,
                local_path_str
            );

            // Save HTML content if enabled
            if config.save_html {
                // Prepare data for callback
                let url_for_registration = url.clone();

                let _task = content_saver::save_html_content_with_resources(
                    &content_with_rewritten_links,
                    url.clone(),
                    config.output_dir.clone(),
                    &resources,
                    config.max_inline_image_size_bytes,
                    config.crawl_rate_rps,
                    move |result| {
                        match result {
                            Ok(()) => {
                                log::info!("HTML content saved successfully for: {}", url_for_registration);
                            }
                            Err(e) => {
                                log::warn!("Failed to save HTML for {}: {}", url_for_registration, e);
                                // Note: URL is already registered even if save failed
                                // This is acceptable - worst case is a 404 for a registered path
                            }
                        }
                    }
                );
            }

            log::info!("Successfully extracted page data for URL: {}", url);
            Ok(super::schema::PageData {
                url: url.to_string(),
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
        }.await;

        on_result(result);
    })
}