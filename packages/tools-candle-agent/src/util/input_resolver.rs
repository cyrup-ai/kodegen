//! Smart input resolution utility
//!
//! Detects whether input is a file path, URL, or literal text and resolves accordingly.

use anyhow::{Context, Result};
use std::path::Path;

/// Resolve input using smart detection: file path, URL, or literal text
///
/// # Detection Logic
/// 1. If path exists on filesystem → Load from file
/// 2. If starts with http:// or https:// → Fetch from URL
/// 3. Otherwise → Use as literal text
///
/// # Arguments
/// * `input` - The input string to resolve
///
/// # Returns
/// Result containing the resolved string content
///
/// # Examples
/// ```no_run
/// use kodegen_candle_agent::util::input_resolver::resolve_input;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // File path
/// let content = resolve_input("./README.md").await?;
///
/// // URL
/// let content = resolve_input("https://example.com/doc.txt").await?;
///
/// // Literal text
/// let content = resolve_input("Hello, world!").await?;
/// # Ok(())
/// # }
/// ```
pub async fn resolve_input(input: &str) -> Result<String> {
    if Path::new(input).exists() {
        // File path - load from disk
        tokio::fs::read_to_string(input)
            .await
            .context(format!("Failed to read file: {}", input))
    } else if input.starts_with("http://") || input.starts_with("https://") {
        // URL - fetch from web with timeout and retries
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        let mut last_error = None;
        for attempt in 0..3 {
            match client.get(input).send().await {
                Ok(response) => {
                    return response
                        .text()
                        .await
                        .context("Failed to read response body");
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(100 * (1 << attempt)))
                            .await;
                    }
                }
            }
        }

        // All retry attempts exhausted - last_error must be Some at this point
        match last_error {
            Some(err) => Err(err.into()),
            None => Err(anyhow::anyhow!(
                "URL fetch failed after all retries with no error recorded"
            )),
        }
    } else {
        // Literal text - use as-is
        Ok(input.to_string())
    }
}

/// Synchronous version for immediate resolution (literal text only)
///
/// # Arguments
/// * `input` - The input string to resolve
///
/// # Returns
/// Result containing the resolved string content
///
/// # Note
/// This function returns the input as-is for literal text.
/// For file paths and URLs, use the async version `resolve_input`.
pub fn resolve_input_sync(input: &str) -> Result<String> {
    Ok(input.to_string())
}

