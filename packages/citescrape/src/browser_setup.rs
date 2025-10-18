//! Automatic Chromium download and version management
//!
//! Dynamically fetches and downloads the latest stable Chromium version
//! with caching, retry logic, and concurrent access protection.

use anyhow::{Context, Result};
use chromiumoxide::fetcher::{BrowserFetcher, BrowserFetcherOptions, Revision};
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Cache TTL for Chromium revision (1 hour)
const CACHE_TTL: Duration = Duration::from_secs(3600);

/// Minimum valid revision (revisions before 1M are too old)
const MIN_VALID_REVISION: u32 = 1_000_000;

/// Maximum valid revision (sanity check for API errors)
const MAX_VALID_REVISION: u32 = 10_000_000;

/// Maximum retry attempts for API calls
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Initial retry delay in milliseconds
const INITIAL_RETRY_DELAY_MS: u64 = 100;

/// Static HTTP client with timeout configuration
///
/// Reused across all API calls for connection pooling and efficiency.
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(1)
        .build()
        .unwrap_or_else(|e| {
            panic!("Failed to build HTTP client (fatal configuration error): {}", e)
        })
});

/// Cached Chromium revision with timestamp
struct CachedRevision {
    revision: Revision,
    fetched_at: Instant,
}

/// Cache for Chromium revision to avoid repeated API calls
static REVISION_CACHE: Lazy<Mutex<Option<CachedRevision>>> = Lazy::new(|| Mutex::new(None));

/// Mutex to ensure only one download happens at a time
static CHROMIUM_SETUP: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

/// Validates that a revision number is within expected bounds
///
/// Checks that the revision is reasonable to catch API errors or corrupted data.
fn validate_revision(revision_num: u32) -> Result<()> {
    if revision_num < MIN_VALID_REVISION {
        anyhow::bail!(
            "Revision {} is too old (< {}), possible API error or corrupted data",
            revision_num,
            MIN_VALID_REVISION
        );
    }

    if revision_num > MAX_VALID_REVISION {
        anyhow::bail!(
            "Revision {} is suspiciously high (> {}), possible API error",
            revision_num,
            MAX_VALID_REVISION
        );
    }

    Ok(())
}

/// Fetches the latest stable Chromium revision from Google's API with retry logic
///
/// Uses exponential backoff: 100ms, 200ms, 400ms delays between attempts.
/// Retries on network errors and server errors (5xx), but not on client errors (4xx).
async fn fetch_latest_revision_with_retry() -> Result<Revision> {
    #[derive(serde::Deserialize)]
    struct ChromiumRelease {
        chromium_main_branch_position: u32,
    }

    let url = "https://chromiumdash.appspot.com/fetch_releases?channel=Stable&num=1";
    let mut attempt = 0;

    loop {
        attempt += 1;

        match HTTP_CLIENT.get(url).send().await {
            Ok(response) => {
                // Check HTTP status
                let status = response.status();
                if !status.is_success() {
                    if attempt < MAX_RETRY_ATTEMPTS && status.is_server_error() {
                        let delay = Duration::from_millis(INITIAL_RETRY_DELAY_MS * (1 << (attempt - 1)));
                        warn!(
                            "API returned {}, retrying in {:?} (attempt {}/{})",
                            status, delay, attempt, MAX_RETRY_ATTEMPTS
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    anyhow::bail!("API request failed with status: {}", status);
                }

                // Parse JSON
                let releases: Vec<ChromiumRelease> = match response.json().await {
                    Ok(r) => r,
                    Err(e) => anyhow::bail!("Failed to parse API response: {}", e),
                };

                let revision_num = releases
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("No stable Chromium releases found in API response"))?
                    .chromium_main_branch_position;

                // Validate revision
                validate_revision(revision_num)
                    .context("Revision validation failed")?;

                let revision = Revision::from(revision_num);
                info!("Fetched latest stable Chromium revision: {}", revision);
                return Ok(revision);
            }
            Err(e) if attempt < MAX_RETRY_ATTEMPTS => {
                // Retry on network errors
                let delay = Duration::from_millis(INITIAL_RETRY_DELAY_MS * (1 << (attempt - 1)));
                warn!(
                    "API call failed: {} - retrying in {:?} (attempt {}/{})",
                    e, delay, attempt, MAX_RETRY_ATTEMPTS
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                return Err(e).context(format!(
                    "Failed to fetch latest Chromium revision after {} attempts",
                    MAX_RETRY_ATTEMPTS
                ));
            }
        }
    }
}

/// Gets the latest Chromium revision with caching
///
/// Checks cache first (TTL: 1 hour). If cache is valid, returns immediately.
/// If cache is expired or empty, fetches from API with retry logic.
async fn get_latest_revision_cached() -> Result<Revision> {
    let mut cache = REVISION_CACHE.lock().await;

    // Check if cache is valid
    if let Some(cached) = cache.as_ref() {
        if cached.fetched_at.elapsed() < CACHE_TTL {
            info!("Using cached Chromium revision: {} (age: {:?})", 
                  cached.revision, 
                  cached.fetched_at.elapsed());
            return Ok(cached.revision.clone());
        }
        info!("Cache expired (age: {:?}), fetching fresh revision", cached.fetched_at.elapsed());
    }

    // Cache miss or expired - fetch from API
    let revision = fetch_latest_revision_with_retry().await?;

    // Update cache
    *cache = Some(CachedRevision {
        revision: revision.clone(),
        fetched_at: Instant::now(),
    });

    Ok(revision)
}

/// Ensures Chromium is available and returns path to executable
///
/// This function is thread-safe and handles concurrent calls efficiently:
/// - First call: Fetches revision, downloads Chromium (~100MB), returns path
/// - Concurrent calls: Wait for first call to complete, then return same path
/// - Subsequent calls: Return cached path immediately
///
/// Features:
/// - Revision caching (1-hour TTL) to minimize API calls
/// - Retry logic (3 attempts with exponential backoff)
/// - HTTP timeouts (5s connect, 10s total)
/// - Revision validation (sanity checks)
/// - Race condition protection (Mutex)
/// - Async I/O throughout
///
/// # Errors
///
/// Returns error if:
/// - Network unavailable for API query or download
/// - API returns invalid data after retries
/// - Cache directory cannot be created
/// - Download fails or is corrupted
/// - Downloaded executable not found
pub async fn ensure_chromium() -> Result<PathBuf> {
    // Acquire lock to prevent concurrent downloads
    let mut setup_guard = CHROMIUM_SETUP.lock().await;

    // Fast path: already set up
    if let Some(path) = setup_guard.as_ref() {
        info!("Using existing Chromium at: {}", path.display());
        return Ok(path.clone());
    }

    info!("Setting up Chromium (first time or cache cleared)...");

    // Get latest revision (with caching and retry)
    let revision = get_latest_revision_cached()
        .await
        .context("Failed to get Chromium revision")?;

    info!("Using Chromium revision: {}", revision);

    // Build fetcher options
    let fetcher_options = BrowserFetcherOptions::builder()
        .with_revision(revision)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build browser fetcher options: {}", e))?;

    let fetcher = BrowserFetcher::new(fetcher_options);

    // Download/fetch Chromium (fetcher handles caching internally)
    info!("Fetching Chromium (will download on first run or if version changed, ~100MB)...");
    let revision_info = fetcher
        .fetch()
        .await
        .context("Failed to fetch Chromium binary - check network connection and disk space")?;

    info!("Chromium downloaded to: {}", revision_info.executable_path.display());

    // Verify executable exists using async I/O
    match tokio::fs::try_exists(&revision_info.executable_path).await {
        Ok(true) => {
            info!("Chromium executable verified at: {}", revision_info.executable_path.display());
        }
        Ok(false) => {
            anyhow::bail!(
                "Downloaded Chromium executable not found at expected path: {}",
                revision_info.executable_path.display()
            );
        }
        Err(e) => {
            anyhow::bail!(
                "Cannot access Chromium executable at {}: {}",
                revision_info.executable_path.display(),
                e
            );
        }
    }

    // Cache the path for future calls
    *setup_guard = Some(revision_info.executable_path.clone());

    Ok(revision_info.executable_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_revision() {
        // Valid revisions
        assert!(validate_revision(1_500_000).is_ok());
        assert!(validate_revision(2_000_000).is_ok());

        // Too old
        assert!(validate_revision(500_000).is_err());
        assert!(validate_revision(0).is_err());

        // Too high (suspicious)
        assert!(validate_revision(20_000_000).is_err());
    }

    #[tokio::test]
    async fn test_fetch_latest_revision() {
        // This tests the real API - may fail if network is down
        let result = fetch_latest_revision_with_retry().await;
        match result {
            Ok(revision) => {
                let revision_num: u32 = revision.into();
                assert!(
                    revision_num > MIN_VALID_REVISION,
                    "Revision {} should be > {}",
                    revision_num,
                    MIN_VALID_REVISION
                );
                println!("Fetched revision: {}", revision_num);
            }
            Err(e) => {
                println!("API call failed (network may be unavailable): {}", e);
                // Don't fail test if network is down
            }
        }
    }

    #[tokio::test]
    #[ignore] // Ignore by default since it downloads ~100MB on first run
    async fn test_ensure_chromium() {
        let path = ensure_chromium()
            .await
            .expect("Failed to ensure chromium");
        assert!(path.exists(), "Chromium executable should exist");

        // Verify it's a file
        let metadata = tokio::fs::metadata(&path)
            .await
            .expect("Failed to get file metadata");
        assert!(metadata.is_file(), "Should be a file, not a directory");
    }

    #[tokio::test]
    #[ignore] // Ignore by default
    async fn test_concurrent_ensure_chromium() {
        // Test race condition protection - launch 5 concurrent calls
        let handles: Vec<_> = (0..5)
            .map(|i| {
                tokio::spawn(async move {
                    let result = ensure_chromium().await;
                    println!("Task {} completed: {:?}", i, result.is_ok());
                    result
                })
            })
            .collect();

        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.expect("Task panicked"))
            .collect();

        // All should succeed
        assert!(results.iter().all(|r| r.is_ok()), "All concurrent calls should succeed");

        // All should return the same path
        let paths: Vec<_> = results.into_iter().map(|r| r.unwrap()).collect();
        let first_path = &paths[0];
        assert!(
            paths.iter().all(|p| p == first_path),
            "All calls should return the same path"
        );
    }
}

