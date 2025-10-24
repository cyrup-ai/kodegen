//! Core search execution logic
//!
//! Handles performing searches, waiting for results, and extracting data
//! from search result pages.

use anyhow::{Context, Result, anyhow};
use chromiumoxide::page::Page;
use rand::Rng;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use url::Url;

use super::types::{
    LINK_SELECTOR, MAX_RESULTS, SEARCH_RESULT_SELECTOR, SEARCH_RESULTS_WAIT_TIMEOUT, SEARCH_URL,
    SNIPPET_SELECTOR, SearchResult, TITLE_SELECTOR,
};
use crate::crawl_engine::page_enhancer;

/// Perform a search with kromekover stealth injection
///
/// Applies kromekover stealth features to the page before navigating to `DuckDuckGo`,
/// then navigates directly to the search results URL. `DuckDuckGo` uses React-based
/// rendering, so we wait for results to appear after navigation.
///
/// # Arguments
/// * `page` - Blank page instance to enhance and use for search
/// * `query` - Search query string
///
/// # Based on
/// - packages/citescrape/src/crawl_engine/core.rs:231-259 (stealth pattern)
/// - `DuckDuckGo` DOM analysis from `tasks/DUCK_DUCK_GO_DOM_PARSING.md`
pub async fn perform_search(page: &Page, query: &str) -> Result<()> {
    // Apply kromekover stealth injection BEFORE navigation
    info!("Applying kromekover stealth injection");

    // Wait for stealth injection with timeout
    match tokio::time::timeout(
        Duration::from_secs(5),
        page_enhancer::enhance_page(page.clone()),
    )
    .await
    {
        Ok(Ok(())) => info!("Stealth injection complete"),
        Ok(Err(e)) => warn!("Stealth injection failed: {}", e),
        Err(_) => warn!("Stealth injection timeout"),
    }

    // Navigate directly to DuckDuckGo search results with proper URL encoding
    let mut search_url = Url::parse(SEARCH_URL).context("Failed to parse DuckDuckGo base URL")?;
    search_url
        .query_pairs_mut()
        .append_pair("q", query)
        .append_pair("ia", "web");

    info!("Navigating to DuckDuckGo search: {}", search_url);
    page.goto(search_url.as_str())
        .await
        .context("Failed to navigate to DuckDuckGo")?;

    // DuckDuckGo uses React for rendering - wait for initial load
    page.wait_for_navigation()
        .await
        .context("Failed to wait for initial page load")?;

    // Smart wait: Poll for results instead of fixed 3s delay
    // This is faster when results load quickly, but waits up to 5s if needed
    let poll_start = Instant::now();
    let max_wait = Duration::from_secs(5);
    let poll_interval = Duration::from_millis(200);

    info!("Waiting for React to render search results...");
    loop {
        // Check if results are present
        if page.find_element(SEARCH_RESULT_SELECTOR).await.is_ok() {
            let elapsed = poll_start.elapsed();
            debug!(
                "Search results appeared after {:.2}s",
                elapsed.as_secs_f64()
            );
            break;
        }

        // Check timeout
        if poll_start.elapsed() >= max_wait {
            // Check if we got a CAPTCHA or error page
            let url = page.url().await.ok().flatten().unwrap_or_default();
            if url.contains("/sorry/") || url.contains("captcha") {
                return Err(anyhow!(
                    "DuckDuckGo presented a CAPTCHA page. Try again later or use a different network."
                ));
            }

            return Err(anyhow!(
                "Timeout waiting for DuckDuckGo results to render. \
                 React took longer than {}s to load results. \
                 This may indicate network issues or DuckDuckGo changes.",
                max_wait.as_secs()
            ));
        }

        // Wait before next poll
        tokio::time::sleep(poll_interval).await;
    }

    Ok(())
}

/// Wait for search results to appear in the DOM
///
/// Polls the page for `SEARCH_RESULT_SELECTOR` elements with a 100ms interval,
/// timing out after `SEARCH_RESULTS_WAIT_TIMEOUT` seconds.
///
/// This is necessary because `page.wait_for_navigation()` returns when the HTTP
/// response arrives, but Google renders search results via JavaScript afterward.
/// We must verify the DOM actually contains result elements before scraping.
///
/// # Arguments
/// * `page` - Page to monitor for search result loading
pub async fn wait_for_results(page: &Page) -> Result<()> {
    let timeout_duration = Duration::from_secs(SEARCH_RESULTS_WAIT_TIMEOUT);
    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    info!("Waiting for search results to appear in DOM");

    loop {
        // Try to find search result elements in DOM
        match page.find_element(SEARCH_RESULT_SELECTOR).await {
            Ok(_) => {
                info!("Search results found in DOM after {:?}", start.elapsed());
                return Ok(());
            }
            Err(_) if start.elapsed() >= timeout_duration => {
                // Timeout - provide diagnostic info including full HTML
                let url = page
                    .url()
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "unknown".to_string());

                // Get page HTML for debugging
                let html = page
                    .content()
                    .await
                    .unwrap_or_else(|_| "Failed to retrieve page HTML".to_string());

                eprintln!("\n========== PAGE HTML DEBUG ==========");
                eprintln!("URL: {url}");
                eprintln!("Expected selector: {SEARCH_RESULT_SELECTOR}");
                eprintln!("HTML length: {} bytes", html.len());
                eprintln!("\n--- First 2000 chars of HTML ---");
                eprintln!("{}", &html[..html.len().min(2000)]);
                eprintln!("\n--- Last 2000 chars of HTML ---");
                let start_idx = html.len().saturating_sub(2000);
                eprintln!("{}", &html[start_idx..]);
                eprintln!("========== END HTML DEBUG ==========\n");

                // Also save full HTML to file for inspection
                if let Err(e) = tokio::fs::write("/tmp/search_page_debug.html", &html).await {
                    eprintln!("Failed to write debug HTML to /tmp/search_page_debug.html: {e}");
                } else {
                    eprintln!("Full HTML saved to: /tmp/search_page_debug.html");
                }

                return Err(anyhow!(
                    "Timeout waiting for search results. Page URL: {url}. \
                     Expected selector '{SEARCH_RESULT_SELECTOR}' not found after {timeout_duration:?}"
                ));
            }
            Err(_) => {
                // Not found yet, wait and retry
                tokio::time::sleep(poll_interval).await;
            }
        }
    }
}

/// Extract search results from the page
///
/// Extracts title, URL, and snippet for each result up to `MAX_RESULTS`.
/// Uses fail-fast approach: URLs must exist (critical), titles use fallback text,
/// snippets gracefully default to "No description available".
///
/// # Arguments
/// * `page` - Page containing search results
///
/// # Returns
/// Vector of `SearchResult` structs
///
/// # Based on
/// - packages/citescrape/src/google_search.rs:208-263
///
/// # Note
/// Extraction logic is inlined because `chromiumoxide::Element` doesn't implement
/// Clone, making it difficult to reuse elements across multiple extraction calls.
pub async fn extract_results(page: &Page) -> Result<Vec<SearchResult>> {
    let search_results = page
        .find_elements(SEARCH_RESULT_SELECTOR)
        .await
        .context("Failed to find search results")?;

    info!("Found {} search results", search_results.len());

    // Fail fast if no results (likely CAPTCHA, error page, or DuckDuckGo DOM change)
    if search_results.is_empty() {
        // Get current URL for diagnostics
        let url = page.url().await.ok().flatten().unwrap_or_default();

        // Check for known error conditions
        if url.contains("/sorry/") || url.contains("captcha") {
            return Err(anyhow!(
                "DuckDuckGo CAPTCHA detected. No search results available. \
                 Try again later or use a different network connection."
            ));
        }

        // Check if this might be a "no results" page from DDG
        if let Ok(body) = page.find_element("body").await
            && let Ok(Some(text)) = body.inner_text().await
            && text.to_lowercase().contains("no results")
        {
            return Err(anyhow!(
                "DuckDuckGo returned zero results for this query. \
                 Try a different search term."
            ));
        }

        return Err(anyhow!(
            "No search results found on DuckDuckGo. This may indicate:\n\
             • DuckDuckGo DOM structure changed (selector '{SEARCH_RESULT_SELECTOR}' not found)\n\
             • Network/connection issues\n\
             • DuckDuckGo is temporarily unavailable\n\
             Current URL: {url}"
        ));
    }

    let mut results = Vec::new();

    for (index, result) in search_results.into_iter().enumerate().take(MAX_RESULTS) {
        // Extract title - REQUIRED
        let title = result
            .find_element(TITLE_SELECTOR)
            .await
            .with_context(|| {
                format!(
                    "DuckDuckGo result {}: Title element not found with selector '{}'. \
                 DOM structure may have changed.",
                    index + 1,
                    TITLE_SELECTOR
                )
            })?
            .inner_text()
            .await
            .with_context(|| format!("Failed to get title text for result {}", index + 1))?
            .unwrap_or_else(|| format!("Untitled Result {}", index + 1));

        // Extract URL - CRITICAL, must succeed
        let url = result
            .find_element(LINK_SELECTOR)
            .await
            .with_context(|| {
                format!(
                    "DuckDuckGo result {}: Link element not found with selector '{}'. \
                 DOM structure may have changed.",
                    index + 1,
                    LINK_SELECTOR
                )
            })?
            .attribute("href")
            .await
            .with_context(|| format!("Failed to get href attribute for result {}", index + 1))?
            .ok_or_else(|| {
                anyhow!(
                    "DuckDuckGo result {}: Link href attribute is empty. \
                 This shouldn't happen - may indicate a DuckDuckGo UI change.",
                    index + 1
                )
            })?;

        // Extract snippet - OPTIONAL, can fallback
        let snippet = match result.find_element(SNIPPET_SELECTOR).await {
            Ok(el) => el
                .inner_text()
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "No description available".to_string()),
            Err(_) => "No description available".to_string(),
        };

        results.push(SearchResult {
            rank: index + 1,
            title,
            url,
            snippet,
        });
    }

    Ok(results)
}

/// Classify errors into retryable vs permanent failures
///
/// Based on chromiumoxide error patterns from browser automation.
///
/// # Permanent Errors (return false)
/// - Browser/page closed or disconnected
/// - Session terminated
/// - Frame/target not found
/// - CAPTCHA detected
///
/// # Transient Errors (return true)  
/// - Network timeouts
/// - Connection refused (temporary)
/// - Rate limiting
///
/// # Unknown Errors (return true)
/// - Default to retrying for safety
fn is_retryable_error(error: &anyhow::Error) -> bool {
    let error_str = error.to_string().to_lowercase();

    // Permanent errors - browser/page state is broken, retry won't help
    if error_str.contains("browser closed")
        || error_str.contains("browser disconnected")
        || error_str.contains("page closed")
        || error_str.contains("target closed")
        || error_str.contains("session not found")
        || error_str.contains("session closed")
        || error_str.contains("no response from the chromium instance")
        || error_str.contains("channel")  // Channel errors = browser died
        || error_str.contains("frame") && error_str.contains("not found")
        || error_str.contains("captcha")
        || error_str.contains("websocket")
    // WebSocket errors = connection lost
    {
        return false;
    }

    // Transient errors - safe to retry, might succeed next time
    if error_str.contains("timeout")
        || error_str.contains("timed out")
        || error_str.contains("network")
        || error_str.contains("connection refused")
        || error_str.contains("connection reset")
        || error_str.contains("rate limit")
        || error_str.contains("429")
    // HTTP 429 Too Many Requests
    {
        return true;
    }

    // Unknown errors - retry conservatively (better safe than sorry)
    true
}

/// Retry a search operation with exponential backoff and error classification
///
/// Only retries errors that are likely transient (timeouts, network issues).
/// Fails fast on permanent errors (browser crashes, page closed).
///
/// # Arguments
/// * `f` - Async function to retry
/// * `max_retries` - Maximum number of retry attempts
///
/// # Returns
/// Result of the operation or final error after all retries
///
/// # Based on
/// - packages/citescrape/src/google_search.rs:275-299
pub async fn retry_with_backoff<F, Fut, T>(f: F, max_retries: u32) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Check if error is retryable BEFORE checking retry limit
                if !is_retryable_error(&e) {
                    warn!("Non-retryable error encountered, failing fast: {:?}", e);
                    return Err(e);
                }

                if retries >= max_retries {
                    warn!("Max retries ({}) exceeded: {:?}", max_retries, e);
                    return Err(e);
                }

                let delay = 2u64.pow(retries) * 1000 + rand::rng().random_range(0..1000);
                warn!(
                    "Retryable error, attempt {}/{}, retrying in {}ms: {:?}",
                    retries + 1,
                    max_retries,
                    delay,
                    e
                );
                tokio::time::sleep(Duration::from_millis(delay)).await;
                retries += 1;
            }
        }
    }
}
