//! DEPRECATED: This module is deprecated in favor of the web_search module.
//!
//! Please use:
//! - Module: `crate::web_search` for core search functionality
//! - MCP Tool: `crate::mcp::WebSearchTool` for MCP integration
//!
//! This file is kept for backward compatibility only and will be removed
//! in a future version.

#![deprecated(
    since = "0.2.0",
    note = "Use crate::web_search module and crate::mcp::WebSearchTool instead"
)]

use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser, BrowserConfigBuilder};
use chromiumoxide::cdp::browser_protocol::network::EventRequestWillBeSent;
use chromiumoxide::page::Page;
use futures::StreamExt;
use std::time::Duration;
use tokio::task;
use tracing::{info, error, warn};
use std::fs::File;
use rand::Rng;
use serde_json::json;
use crate::runtime::{spawn_async, AsyncTask};

const GOOGLE_SEARCH_URL: &str = "https://www.google.com";
const SEARCH_BOX_SELECTOR: &str = "textarea[name='q']";
const SEARCH_BUTTON_SELECTOR: &str = "input[name='btnK'], button[name='btnK']";
const SEARCH_RESULT_SELECTOR: &str = "div.g";
const TITLE_SELECTOR: &str = "h3";
const SNIPPET_SELECTOR: &str = "div.VwiC3b";
const LINK_SELECTOR: &str = "div.yuRUbf > a";
const SEARCH_RESULTS_WAIT_TIMEOUT: u64 = 10;
// const TYPING_DELAY: u64 = 100;
const ACTION_DELAY: u64 = 500;
const MAX_RETRIES: u32 = 3;
const MAX_RESULTS: usize = 10;

pub fn run(
    query: String,
    on_result: impl FnOnce(Result<()>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result: Result<()> = async {
            info!("Starting Google search for query: {}", query);

            // Use blocking channel for browser launch
            let (tx, rx) = tokio::sync::oneshot::channel();
            let task = launch_browser(move |result| {
                let _ = tx.send(result);
            });
            let _guard = crate::runtime::TaskGuard::new(task, "launch_browser");
            let browser = rx.await.map_err(|_| anyhow::anyhow!("Failed to receive browser"))??;

            // Use blocking channel for page creation
            let (tx, rx) = tokio::sync::oneshot::channel();
            let task = create_page(browser, move |result| {
                let _ = tx.send(result);
            });
            let _guard = crate::runtime::TaskGuard::new(task, "create_page");
            let (mut browser, page) = rx.await.map_err(|_| anyhow::anyhow!("Failed to receive page"))??;

            let results = retry_with_backoff(|| async {
                // Use blocking channel for search
                let (tx, rx) = tokio::sync::oneshot::channel();
                let task = perform_search(page.clone(), query.clone(), move |result| {
                    let _ = tx.send(result);
                });
                let _guard = crate::runtime::TaskGuard::new(task, "perform_search");
                rx.await.map_err(|_| anyhow::anyhow!("Failed to perform search"))??;

                let _ = wait_for_search_results(&page).await?;
                extract_results(&page).await?
            }, MAX_RETRIES).await?;

            print_results(&results);
            
            // Use blocking channel for save
            let (tx, rx) = tokio::sync::oneshot::channel();
            let task = save_results_to_file(&results, &query, move |result| {
                let _ = tx.send(result);
            });
            let _guard = crate::runtime::TaskGuard::new(task, "save_results_to_file");
            rx.await.map_err(|_| anyhow::anyhow!("Failed to save results to file"))??;

            browser.close().await.context("Failed to close browser")?;
            info!("Search completed");
            Ok(())
        }.await;
        
        on_result(result);
    })
}

fn launch_browser(
    on_result: impl FnOnce(Result<Browser>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            let browser_config = BrowserConfigBuilder::default()
                .request_timeout(Duration::from_secs(30))
                .window_size(1920, 1080)
                .arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
                .arg("--disable-blink-features=AutomationControlled")
                .arg("--exclude-switches=enable-automation")
                .arg("--disable-infobars")
                .arg("--disable-dev-shm-usage")
                .arg("--disable-gpu")
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build browser config: {e}"))?;

            info!("Launching browser with config: {:?}", browser_config);
            let (browser, mut handler) = Browser::launch(browser_config)
                .await
                .context("Failed to launch browser")?;

            task::spawn(async move {
                while let Some(h) = handler.next().await {
                    if let Err(e) = h {
                        error!("Browser handler error: {:?}", e);
                    }
                }
            });

            Ok(browser)
        }.await;
        
        on_result(result);
    })
}

fn create_page(
    browser: Browser,
    on_result: impl FnOnce(Result<(Browser, Page)>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            let page = browser.new_page(GOOGLE_SEARCH_URL).await.context("Failed to create new page")?;
            page.wait_for_navigation().await.context("Failed to wait for navigation")?;
            info!("Navigated to Google homepage");
            Ok((browser, page))
        }.await;
        
        on_result(result);
    })
}

fn perform_search(
    page: Page,
    query: String,
    on_result: impl FnOnce(Result<()>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            info!("Navigating to Google homepage");
            page.goto(GOOGLE_SEARCH_URL).await?;
            tokio::time::sleep(Duration::from_secs(2)).await;
            info!("Waited 2 seconds after navigation");

            info!("Attempting to find search box with selector: {}", SEARCH_BOX_SELECTOR);
            let search_box = match page.find_element(SEARCH_BOX_SELECTOR).await {
                Ok(element) => element,
                Err(e) => {
                    error!("Failed to find search box: {:?}", e);
                    let html = page.content().await?;
                    error!("Page HTML: {}", html);
                    return Err(anyhow::anyhow!("Failed to find search box"));
                }
            };

            info!("Found search box, attempting to click");
            search_box.click().await.context("Failed to click search box")?;
            tokio::time::sleep(Duration::from_millis(ACTION_DELAY)).await;

            info!("Typing search query: {}", query);
            search_box.type_str(&query).await.context("Failed to type search query")?;
            tokio::time::sleep(Duration::from_millis(ACTION_DELAY)).await;

            info!("Attempting to find search button");
            let search_button = page.find_element(SEARCH_BUTTON_SELECTOR).await
                .context("Failed to find search button")?;

            info!("Clicking search button");
            search_button.click().await.context("Failed to click search button")?;

            info!("Waiting for search results");
            page.wait_for_navigation().await.context("Failed to wait for search results")?;
            Ok(())
        }.await;
        
        on_result(result);
    })
}

fn wait_for_search_results(page: &Page) -> AsyncTask<Result<()>> {
    let page = page.clone();
    spawn_async(async move {
        let mut network_events = page.event_listener::<EventRequestWillBeSent>().await?;
        let timeout = tokio::time::sleep(Duration::from_secs(SEARCH_RESULTS_WAIT_TIMEOUT));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(event) = network_events.next() => {
                    if event.request.url.contains("search?") {
                        info!("Detected search results network request");
                        break;
                    }
                }
                _ = &mut timeout => {
                    warn!("Timeout waiting for search results");
                    break;
                }
            }
        }
        Ok(())
    })
}

// Extract search results with inline element processing.
// Note: Extraction logic is inlined rather than using helper functions because
// chromiumoxide::Element doesn't implement Clone, making it difficult to reuse
// elements across multiple extraction calls. The inline approach is simpler and
// more maintainable for this use case.
fn extract_results(page: &Page) -> AsyncTask<Result<Vec<serde_json::Value>>> {
    let page = page.clone();
    spawn_async(async move {
        let search_results = page.find_elements(SEARCH_RESULT_SELECTOR)
            .await
            .context("Failed to find search results")?;
        info!("Found {} search results", search_results.len());

        let mut results = Vec::new();

        for (index, result) in search_results.into_iter().enumerate().take(MAX_RESULTS) {
            // Extract title
            let title = match result.find_element(TITLE_SELECTOR).await {
                Ok(el) => match el.inner_text().await {
                    Ok(opt_text) => opt_text.unwrap_or_else(|| "N/A".to_string()),
                    Err(_) => "N/A".to_string()
                },
                Err(_) => "N/A".to_string()
            };

            // Extract link
            let link = match result.find_element(LINK_SELECTOR).await {
                Ok(el) => match el.attribute("href").await {
                    Ok(Some(attr)) => attr,
                    _ => "N/A".to_string()
                },
                Err(_) => "N/A".to_string()
            };

            // Extract snippet
            let snippet = match result.find_element(SNIPPET_SELECTOR).await {
                Ok(el) => match el.inner_text().await {
                    Ok(opt_text) => opt_text.unwrap_or_else(|| "N/A".to_string()),
                    Err(_) => "N/A".to_string()
                },
                Err(_) => "N/A".to_string()
            };

            let result_json = json!({
                "rank": index + 1,
                "title": title,
                "url": link,
                "snippet": snippet
            });

            results.push(result_json);
        }

        Ok(results)
    })
}

fn print_results(results: &[serde_json::Value]) {
    for result in results {
        println!("Result {}:", result["rank"]);
        println!("Title: {}", result["title"]);
        println!("URL: {}", result["url"]);
        println!("Snippet: {}", result["snippet"]);
        println!();
    }
}

async fn retry_with_backoff<F, Fut, T, E>(
    f: F,
    max_retries: u32,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if retries >= max_retries {
                    return Err(e);
                }
                let delay = 2u64.pow(retries) * 1000 + rand::rng().random_range(0..1000);
                warn!("Request failed, retrying in {}ms. Error: {:?}", delay, e);
                tokio::time::sleep(Duration::from_millis(delay)).await;
                retries += 1;
            }
        }
    }
}

fn save_results_to_file(
    results: &[serde_json::Value], 
    query: &str,
    on_result: impl FnOnce(Result<()>) + Send + 'static,
) -> AsyncTask<()> {
    let results = results.to_vec();
    let query = query.to_string();
    
    spawn_async(async move {
        let result = (|| -> Result<()> {
            let filename = format!("{}_results.json", query.replace(" ", "_"));
            let file = File::create(&filename).context("Failed to create file")?;
            serde_json::to_writer_pretty(file, &results).context("Failed to write JSON to file")?;
            info!("Results saved to file: {}", filename);
            Ok(())
        })();
        
        on_result(result);
    })
}
