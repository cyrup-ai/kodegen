use anyhow::{Context, Result};
use chromiumoxide::cdp::browser_protocol::network::{EventResponseReceived, Headers};
use chromiumoxide::listeners::EventStream;
use flate2::read::GzDecoder;
use futures::StreamExt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::timeout;
use url::Url;

use crate::content_saver::CacheMetadata;

/// Get the mirror path for a URL synchronously (internal helper for cache checking)
pub fn get_mirror_path_sync(url: &str, output_dir: &Path, filename: &str) -> Result<PathBuf> {
    let url = Url::parse(url).context("Failed to parse URL")?;
    let domain = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;
    let path = if url.path() == "/" {
        PathBuf::new()
    } else {
        PathBuf::from(url.path().trim_start_matches('/'))
    };

    let mirror_path = output_dir.join(domain).join(path).join(filename);
    Ok(mirror_path)
}

/// Read the cached etag from a gzip file's header comment
///
/// Looks for the expected cache file path based on URL and reads
/// the etag stored in the gzip header comment metadata.
pub async fn read_cached_etag(url: &str, output_dir: &Path) -> Result<Option<String>> {
    // Get expected cache path (this is pure computation, can stay sync)
    let cache_path = get_mirror_path_sync(url, output_dir, "index.md")?;
    let gz_path = cache_path.with_extension("md.gz");

    // Quick async file existence check - non-blocking
    if !tokio::fs::try_exists(&gz_path).await? {
        return Ok(None);
    }

    // Spawn blocking for file I/O + decompression + parsing
    // Moves work to dedicated blocking thread pool (max 512 threads)
    tokio::task::spawn_blocking(move || -> Result<Option<String>> {
        // All blocking work happens on blocking thread pool
        let file = File::open(&gz_path).context("Failed to open cached gzip file")?;
        let gz = GzDecoder::new(file);

        let header = gz
            .header()
            .ok_or_else(|| anyhow::anyhow!("No gzip header found"))?;

        let comment = std::str::from_utf8(header.comment().unwrap_or(&[]))
            .context("Invalid UTF-8 in gzip comment")?;

        let metadata: CacheMetadata =
            serde_json::from_str(comment).context("Failed to parse cache metadata JSON")?;

        Ok(Some(metadata.etag))
    })
    .await
    .map_err(|e| anyhow::anyhow!("Spawn blocking join error: {e}"))?
}

/// Extract etag from Network Response headers
///
/// The headers come from CDP EventResponseReceived.response.headers
/// which is a JSON object like {"etag": "abc123", "content-type": "text/html"}
#[must_use]
pub fn extract_etag_from_headers(headers: &Headers) -> Option<String> {
    let headers_json = headers.inner();

    headers_json.as_object()?.get("etag")?.as_str().map(|s| {
        // Strip "W/" prefix for weak etags
        s.strip_prefix("W/").unwrap_or(s).to_string()
    })
}

/// Async helper: Check if received etag matches expected etag
///
/// Listens to responseReceived events until we find the main document response.
/// Returns true if etag matches (cache hit), false otherwise.
///
/// # Arguments
/// * `events` - Event stream of network responses from CDP
/// * `url` - The URL to match against response events
/// * `expected_etag` - The etag from cached file to compare
/// * `timeout_duration` - How long to wait for response (configurable per crawl)
pub async fn check_etag_from_events(
    events: &mut EventStream<EventResponseReceived>,
    url: &str,
    expected_etag: &str,
    timeout_duration: Duration,
) -> bool {
    // Use provided timeout instead of hardcoded value
    let result = timeout(timeout_duration, async {
        while let Some(event) = events.next().await {
            // Only check main document, not images/css/js
            if event.response.url == url {
                if let Some(received_etag) = extract_etag_from_headers(&event.response.headers) {
                    return received_etag == expected_etag;
                }
                // No etag header = not a match
                return false;
            }
        }
        false
    })
    .await;

    result.unwrap_or(false)
}
