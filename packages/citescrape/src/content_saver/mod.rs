//! Content saving utilities for web scraping
//!
//! ## Channel Naming Convention
//!
//! All async operations use descriptive channel names following this pattern:
//! `<context>_<purpose>_<direction>`
//!
//! Components:
//! - `context`: Operation context (md, json, page, html, html_res, etc.)
//! - `purpose`: Channel purpose (path, compress, inline, delete, etc.)
//! - `direction`: tx (sender) or rx (receiver)
//!
//! Examples:
//! - `md_path_tx` - Markdown path resolution sender
//! - `json_compress_rx` - JSON compression receiver
//! - `html_res_inline_tx` - HTML resource inlining sender
//! - `page_compress_rx` - Page data compression receiver
//!
//! Simple names (e.g., `path_tx`) are acceptable when:
//! - Function has only one channel of that type
//! - Context is clear from surrounding code
//! - No risk of confusion with other channels

use anyhow::Result;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};

/// Helper function to await oneshot channels with timeout protection
/// 
/// Prevents hung tasks by applying a timeout to channel receive operations.
/// If the sending task hangs or is blocked indefinitely, this will return
/// an error after the specified timeout instead of waiting forever.
/// 
/// # Arguments
/// 
/// * `rx` - The oneshot receiver to await
/// * `timeout_secs` - Timeout duration in seconds
/// * `operation` - Description of the operation for error messages
/// 
/// # Returns
/// 
/// * `Ok(T)` - Successfully received value from channel
/// * `Err` - Either timeout exceeded or channel was closed
pub(crate) async fn await_with_timeout<T>(
    rx: oneshot::Receiver<T>,
    timeout_secs: u64,
    operation: &str,
) -> Result<T> {
    match timeout(Duration::from_secs(timeout_secs), rx).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) => Err(anyhow::anyhow!("Channel closed for: {}", operation)),
        Err(_) => Err(anyhow::anyhow!("Timeout waiting for: {}", operation)),
    }
}

/// Helper function to log channel send errors with proper context
/// 
/// When oneshot channels fail to send results, this function logs the error
/// with full context including whether the operation succeeded or failed,
/// and what the actual error was (if any). This improves debugging by
/// capturing information that would otherwise be lost.
/// 
/// # Arguments
/// 
/// * `send_result` - The result from calling `tx.send(result)`
/// * `operation` - Description of the operation (e.g., "inline_all_resources")
/// * `context` - Additional context (e.g., URL being processed)
/// 
/// # Type Parameters
/// 
/// * `T` - The success type of the inner Result
/// * `E` - The error type of the inner Result (must implement Display)
pub(crate) fn log_send_error<T, E>(
    send_result: std::result::Result<(), Result<T, E>>,
    operation: &str,
    context: &str,
) where 
    E: std::fmt::Display 
{
    if let Err(unsent_result) = send_result {
        match unsent_result {
            Ok(_) => {
                log::warn!(
                    "{}: channel closed for {} (operation succeeded but receiver dropped)",
                    context,
                    operation
                );
            }
            Err(e) => {
                log::warn!(
                    "{}: channel closed for {} with error: {} (receiver dropped)",
                    context,
                    operation,
                    e
                );
            }
        }
    }
}

// Module declarations
pub mod cache_check;
mod compression;
mod html_saver;
mod indexing;
mod json_saver;
mod markdown_saver;
pub mod markdown_converter;

// Re-export public API from cache_check module
pub use cache_check::{read_cached_etag, extract_etag_from_headers, check_etag_from_events, get_mirror_path_sync};

// Re-export public API from compression module
pub use compression::{CacheMetadata, save_compressed_file};

// Re-export public API from html_saver module
pub use html_saver::{save_html_content, save_html_content_with_resources};

// Re-export public API from indexing module
pub use indexing::optimize_search_index;

// Re-export public API from json_saver module
pub use json_saver::{save_json_data, save_page_data};

// Re-export public API from markdown_saver module
pub use markdown_saver::save_markdown_content;
