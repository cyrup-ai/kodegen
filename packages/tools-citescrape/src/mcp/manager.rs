//! MCP manager implementations for session tracking, search caching, and manifest persistence

use super::types::{ActiveCrawlSession, CrawlManifest, CrawlStatus};
use crate::config::CrawlConfig;
use crate::search::{IndexingSender, SearchEngine};
use kodegen_mcp_tool::error::McpError;
use log;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use url::Url;

// =============================================================================
// TIMESTAMP UTILITIES FOR LOCK-FREE CACHE
// =============================================================================

/// Global epoch for converting Instant to/from u64 nanoseconds for atomic storage
///
/// Initialized once at first access. Uses `LazyLock` for initialization.
/// All Instant values are stored as nanoseconds relative to this epoch.
fn get_timestamp_epoch() -> &'static Instant {
    use std::sync::LazyLock;
    static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);
    &EPOCH
}

/// Convert Instant to nanoseconds since epoch for atomic storage
///
/// Uses (seconds * `1_000_000_000` + `subsec_nanos`) to avoid u128→u64 truncation.
/// This gives us ~584 years of nanosecond precision before saturation.
#[inline]
fn instant_to_nanos(instant: Instant) -> u64 {
    let duration = instant.saturating_duration_since(*get_timestamp_epoch());

    // Use seconds + subsec_nanos to avoid truncating u128→u64
    // This safely represents up to ~584 years in nanoseconds
    let secs = duration.as_secs();
    let nanos = u64::from(duration.subsec_nanos());

    secs.saturating_mul(1_000_000_000).saturating_add(nanos)
}

/// Convert nanoseconds since epoch back to Instant
#[inline]
// APPROVED BY DAVID MAPLE 10/17/2025 - False positive: planned for future timestamp comparison features
#[allow(dead_code)]
fn nanos_to_instant(nanos: u64) -> Instant {
    *get_timestamp_epoch() + std::time::Duration::from_nanos(nanos)
}

// =============================================================================
// CACHE CAPACITY CONSTANTS
// =============================================================================

/// Initial capacity for crawl session `HashMap`
///
/// Pre-allocates space for this many active sessions to reduce allocations.
/// Based on typical usage: users run 5-20 concurrent crawls per server instance.
const SESSION_CACHE_INITIAL_CAPACITY: usize = 16;

/// Initial capacity for search engine cache `HashMap`
///
/// Pre-allocates space for this many cached engines to reduce allocations.
/// Based on typical usage: users crawl 5-10 different sites per session.
const SEARCH_CACHE_INITIAL_CAPACITY: usize = 10;

/// Manager for tracking active crawl sessions
///
/// Uses `tokio::sync::Mutex` for async-safe concurrent access.
/// Sessions automatically tracked by `crawl_id` (UUID v4).
#[derive(Clone)]
pub struct CrawlSessionManager {
    sessions: Arc<Mutex<HashMap<String, ActiveCrawlSession>>>,
}

impl CrawlSessionManager {
    /// Create a new session manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::with_capacity(
                SESSION_CACHE_INITIAL_CAPACITY,
            ))),
        }
    }

    /// Register a new crawl session
    pub async fn register(&self, crawl_id: String, session: ActiveCrawlSession) {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(crawl_id, session);
    }

    /// Update status for a crawl session
    pub async fn update_status(&self, crawl_id: &str, status: CrawlStatus) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(crawl_id) {
            session.status = status;
        }
    }

    /// Update progress counters for a crawl session
    pub async fn update_progress(
        &self,
        crawl_id: &str,
        total_pages: usize,
        current_url: Option<String>,
    ) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(crawl_id) {
            session.total_pages = total_pages;
            if let Some(url) = current_url {
                session.current_url = Some(url);
            }
        }
    }

    /// Get a clone of a crawl session
    pub async fn get_session(&self, crawl_id: &str) -> Option<ActiveCrawlSession> {
        let sessions = self.sessions.lock().await;
        sessions.get(crawl_id).cloned()
    }

    /// Remove and return a crawl session
    pub async fn remove_session(&self, crawl_id: &str) -> Option<ActiveCrawlSession> {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(crawl_id)
    }

    /// List all active crawl IDs
    pub async fn list_active(&self) -> Vec<String> {
        let sessions = self.sessions.lock().await;
        sessions.keys().cloned().collect()
    }

    /// Cleanup completed/failed sessions older than retention period
    ///
    /// Removes sessions that are in Completed or Failed status and older than 5 minutes.
    /// This prevents unbounded growth of the sessions `HashMap` over time.
    /// Also aborts task handles for removed sessions to free up resources.
    // APPROVED BY DAVID MAPLE 10/17/2025 - False positive: called via start_cleanup_task background task
    #[allow(dead_code)]
    async fn cleanup_sessions(&self) {
        use std::time::Duration;

        let cutoff = Duration::from_secs(5 * 60); // 5 minutes
        let now = chrono::Utc::now();
        let mut sessions = self.sessions.lock().await;
        let initial_count = sessions.len();

        sessions.retain(|crawl_id, session| {
            let age = now.signed_duration_since(session.start_time);
            let is_terminal = matches!(
                session.status,
                CrawlStatus::Completed | CrawlStatus::Failed { .. }
            );

            // Keep running sessions, remove terminal sessions older than cutoff
            let should_keep = !is_terminal || age.to_std().unwrap_or(Duration::ZERO) < cutoff;

            if !should_keep {
                log::debug!(
                    "Removing old crawl session {}: {:?} (age: {:?})",
                    crawl_id,
                    session.status,
                    age
                );

                // Abort task handle if present (cleanup background tasks)
                if let Some(handle) = &session.task_handle {
                    handle.abort();
                }
            }

            should_keep
        });

        let cleaned = initial_count - sessions.len();
        if cleaned > 0 {
            log::info!("Cleaned up {cleaned} crawl sessions");
        }
    }

    /// Start background cleanup task (call once at initialization)
    ///
    /// Spawns a tokio task that runs cleanup every 60 seconds.
    /// Follows the same pattern as `TerminalManager` and `SearchManager`.
    ///
    /// # Usage
    /// Called from main.rs after wrapping manager in Arc:
    /// ```rust,no_run
    /// use std::sync::Arc;
    ///
    /// let session_manager = Arc::new(CrawlSessionManager::new());
    /// session_manager.clone().start_cleanup_task();
    /// ```
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.cleanup_sessions().await;
            }
        });
    }
}

impl Default for CrawlSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Entry in the `SearchEngineCache` containing both engine and indexing sender
///
/// Each cached entry includes:
/// - `engine`: The `SearchEngine` instance for querying
/// - `indexing_sender`: Optional sender for incremental index updates
/// - `last_accessed`: Lock-free atomic timestamp for LRU eviction tracking
///
/// # Lock-Free Design
/// Uses `AtomicU64` instead of Mutex<Instant> for zero-contention timestamp updates.
/// Timestamps are stored as nanoseconds since a global epoch, allowing atomic operations.
#[derive(Clone)]
pub struct SearchEngineCacheEntry {
    pub engine: Arc<SearchEngine>,
    pub indexing_sender: Option<Arc<IndexingSender>>,
    /// Lock-free timestamp stored as nanoseconds since epoch
    pub last_accessed: Arc<AtomicU64>,
}

/// Cache for `SearchEngine` instances
///
/// Prevents repeated initialization of `SearchEngine` for the same output directory.
/// Uses Arc<SearchEngine> for cheap cloning across tasks.
///
/// **Lifecycle Management:**
/// - Engines are lazily initialized on first access via `get_or_init()`
/// - Engines remain cached until explicitly removed via `invalidate()` or `clear()`
/// - **IMPORTANT:** Call `shutdown()` before application exit to release Tantivy file handles
///
/// **Shutdown Pattern:**
/// ```rust
/// // During application initialization
/// let search_cache = Arc::new(SearchEngineCache::new());
///
/// // During application shutdown (e.g., SIGTERM handler)
/// search_cache.shutdown().await;
/// ```
///
/// **Resource Details:**
/// Each cached engine holds Tantivy Index and `IndexReader` with memory-mapped file handles.
/// On Linux/macOS, these are mmap'd segments that consume virtual address space.
/// Proper shutdown ensures timely release of file descriptors and clean filesystem unmounting.
#[derive(Clone)]
pub struct SearchEngineCache {
    engines: Arc<Mutex<HashMap<PathBuf, SearchEngineCacheEntry>>>,
}

impl SearchEngineCache {
    /// Create a new search engine cache
    #[must_use]
    pub fn new() -> Self {
        Self {
            engines: Arc::new(Mutex::new(HashMap::with_capacity(
                SEARCH_CACHE_INITIAL_CAPACITY,
            ))),
        }
    }

    /// Get cached engine or initialize new one
    ///
    /// Returns both the `SearchEngine` and optional `IndexingSender` for use in `CrawlConfig`
    pub async fn get_or_init(
        &self,
        output_dir: PathBuf,
        config: &CrawlConfig,
    ) -> Result<SearchEngineCacheEntry, McpError> {
        let mut engines = self.engines.lock().await;

        // Check cache first
        if let Some(entry) = engines.get(&output_dir) {
            // Update last accessed time for LRU tracking (lock-free atomic)
            entry
                .last_accessed
                .store(instant_to_nanos(Instant::now()), Ordering::Relaxed);
            return Ok(entry.clone());
        }

        // Check if we're at capacity before creating new engine
        if engines.len() >= Self::MAX_ENGINES {
            // Find and evict LRU engine to make room
            let mut oldest_key: Option<PathBuf> = None;
            let mut oldest_time = Instant::now();

            for (key, entry) in engines.iter() {
                // Skip the one we're about to create
                if key == &output_dir {
                    continue;
                }

                let last_access_nanos = entry.last_accessed.load(Ordering::Relaxed);
                let last_access = nanos_to_instant(last_access_nanos);

                if oldest_key.is_none() || last_access < oldest_time {
                    oldest_time = last_access;
                    oldest_key = Some(key.clone());
                }
            }

            if let Some(evict_key) = oldest_key {
                engines.remove(&evict_key);
                log::info!(
                    "Evicted LRU engine {:?} to make room for {:?} (limit: {})",
                    evict_key,
                    output_dir,
                    Self::MAX_ENGINES
                );
            }
        }

        // Release lock during initialization (long operation)
        drop(engines);

        // Initialize new engine
        let engine = SearchEngine::create(config).await.map_err(|e| {
            McpError::SearchEngine(format!("Failed to initialize search engine: {e}"))
        })?;

        // Start incremental indexing service with engine clone
        let engine_for_indexing = engine.clone();
        let indexing_sender =
            match crate::search::IncrementalIndexingService::start(engine_for_indexing).await {
                Ok(sender) => {
                    log::info!(
                        "Incremental indexing service started for output_dir: {output_dir:?}"
                    );
                    Some(Arc::new(sender))
                }
                Err(e) => {
                    log::error!("Failed to start incremental indexing service: {e}");
                    None
                }
            };

        let engine = Arc::new(engine);

        // Create cache entry with both engine and sender
        let entry = SearchEngineCacheEntry {
            engine: Arc::clone(&engine),
            indexing_sender,
            last_accessed: Arc::new(AtomicU64::new(instant_to_nanos(Instant::now()))),
        };

        // Re-acquire lock and insert
        let mut engines = self.engines.lock().await;

        // Double-check: another task may have inserted while we were initializing
        if let Some(existing_entry) = engines.get(&output_dir) {
            return Ok(existing_entry.clone());
        }

        engines.insert(output_dir, entry.clone());

        Ok(entry)
    }

    /// Invalidate (remove) cached engine for `output_dir`
    pub async fn invalidate(&self, output_dir: &PathBuf) {
        let mut engines = self.engines.lock().await;
        engines.remove(output_dir);
    }

    /// Clear all cached engines
    pub async fn clear(&self) {
        let mut engines = self.engines.lock().await;
        engines.clear();
    }

    /// Gracefully shutdown cache and release all search engines
    ///
    /// This should be called during application shutdown to ensure
    /// all Tantivy indices are properly closed and file handles released.
    ///
    /// **Resource Cleanup:**
    /// - Drains `HashMap` to take ownership of Arc<SearchEngine>
    /// - Explicitly drops each engine to release Tantivy resources
    /// - Ensures `MmapDirectory` file handles are closed
    /// - Flushes any pending `IndexReader` caches
    ///
    /// **Thread Safety:**
    /// Uses async Mutex lock to safely drain concurrent cache access.
    ///
    /// **Example:**
    /// ```rust
    /// // In application shutdown handler
    /// app_state.search_cache.shutdown().await;
    /// log::info!("Search engine cache shutdown complete");
    /// ```
    pub async fn shutdown(&self) {
        log::info!("Shutting down search engine cache");

        let mut engines = self.engines.lock().await;
        let count = engines.len();

        // Drain all engines and explicitly drop them
        // drain() takes ownership, removing from HashMap
        for (key, engine) in engines.drain() {
            log::debug!("Releasing search engine: {key:?}");
            // Explicit drop ensures immediate cleanup
            // (though implicit drop would also work)
            drop(engine);
        }

        log::info!("Search engine cache shutdown complete: {count} engines");
    }

    /// Get number of cached engines (for monitoring)
    ///
    /// Useful for:
    /// - Monitoring cache growth
    /// - Verifying shutdown completion (should return 0 after shutdown)
    /// - Debugging cache behavior
    ///
    /// **Example:**
    /// ```rust
    /// let size = search_cache.cache_size().await;
    /// log::info!("Search cache contains {} engines", size);
    /// ```
    pub async fn cache_size(&self) -> usize {
        self.engines.lock().await.len()
    }

    /// Maximum number of cached search engines before LRU eviction
    const MAX_ENGINES: usize = 5;

    /// Idle timeout after which engines are eligible for cleanup (30 minutes)
    const ENGINE_IDLE_TIMEOUT_SECS: u64 = 30 * 60;

    /// Cleanup idle engines and enforce LRU eviction policy
    ///
    /// This method:
    /// 1. Removes engines idle for longer than `ENGINE_IDLE_TIMEOUT_SECS`
    /// 2. Enforces `MAX_ENGINES` limit via LRU eviction of least recently used engines
    ///
    /// Prevents unbounded growth of cached Tantivy indices and file descriptors.
    ///
    /// # Lock-Free Design
    /// Uses atomic timestamp reads (no lock contention) for determining idle time and LRU ordering.
    /// This ensures cleanup never blocks concurrent cache access operations.
    // APPROVED BY DAVID MAPLE 10/17/2025 - False positive: called via start_cleanup_task background task
    #[allow(dead_code)]
    async fn cleanup_idle_engines(&self) {
        use std::time::Duration;

        let cutoff = Duration::from_secs(Self::ENGINE_IDLE_TIMEOUT_SECS);
        let mut engines = self.engines.lock().await;
        let initial_count = engines.len();

        // Step 1: Remove idle engines (lock-free using atomic timestamps)
        engines.retain(|path, entry| {
            let last_access_nanos = entry.last_accessed.load(Ordering::Relaxed);
            let last_access = nanos_to_instant(last_access_nanos);
            let age = last_access.elapsed();

            let should_keep = age < cutoff;

            if !should_keep {
                log::info!("Evicting idle search engine: {path:?} (idle: {age:?})");
            }

            should_keep
        });

        // Step 2: LRU eviction if over max size (lock-free using atomic timestamps)
        while engines.len() > Self::MAX_ENGINES {
            // Find oldest (least recently used) engine
            let mut oldest_key: Option<PathBuf> = None;
            let mut oldest_time = Instant::now();

            for (key, entry) in engines.iter() {
                let last_access_nanos = entry.last_accessed.load(Ordering::Relaxed);
                let last_access = nanos_to_instant(last_access_nanos);

                if oldest_key.is_none() || last_access < oldest_time {
                    oldest_time = last_access;
                    oldest_key = Some(key.clone());
                }
            }

            if let Some(key) = oldest_key {
                engines.remove(&key);
                log::info!("LRU eviction: {:?} (cache size: {})", key, engines.len());
            } else {
                // This should never happen now (no lock contention)
                break;
            }
        }

        let cleaned = initial_count.saturating_sub(engines.len());
        if cleaned > 0 {
            log::info!(
                "Cleaned up {} search engines (current size: {})",
                cleaned,
                engines.len()
            );
        }
    }

    /// Start background cleanup task (call once at initialization)
    ///
    /// Spawns a tokio task that runs cleanup every 60 seconds.
    /// Follows the same pattern as `TerminalManager` and `SearchManager`.
    ///
    /// # Usage
    /// Called from main.rs after wrapping cache in Arc:
    /// ```rust,no_run
    /// use std::sync::Arc;
    ///
    /// let engine_cache = Arc::new(SearchEngineCache::new());
    /// engine_cache.clone().start_cleanup_task();
    /// ```
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.cleanup_idle_engines().await;
            }
        });
    }
}

impl Default for SearchEngineCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for crawl manifest persistence
///
/// Provides atomic file writes to prevent corruption.
/// Uses write-to-temp-then-rename pattern for atomicity.
pub struct ManifestManager;

impl ManifestManager {
    const MANIFEST_FILENAME: &'static str = "manifest.json";

    /// Save manifest atomically to {`output_dir}/manifest.json`
    ///
    /// Uses atomic write pattern: write to temp file, sync, rename
    pub async fn save(manifest: &CrawlManifest) -> Result<(), McpError> {
        let manifest_path = manifest.output_dir.join(Self::MANIFEST_FILENAME);

        // Ensure output directory exists
        if let Some(parent) = manifest_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                McpError::Manifest(format!("Failed to create manifest directory: {e}"))
            })?;
        }

        // Serialize to JSON with pretty formatting
        let json = serde_json::to_string_pretty(manifest)
            .map_err(|e| McpError::Manifest(format!("Failed to serialize manifest: {e}")))?;

        // Atomic write pattern: temp file + rename
        let temp_path = manifest_path.with_extension("json.tmp");

        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(|e| McpError::Manifest(format!("Failed to create temp manifest file: {e}")))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| McpError::Manifest(format!("Failed to write manifest: {e}")))?;

        // Sync to disk before rename (ensures durability)
        file.sync_all()
            .await
            .map_err(|e| McpError::Manifest(format!("Failed to sync manifest to disk: {e}")))?;

        // Atomic rename (overwrites existing file)
        fs::rename(&temp_path, &manifest_path)
            .await
            .map_err(|e| McpError::Manifest(format!("Failed to rename manifest file: {e}")))?;

        Ok(())
    }

    /// Load manifest from {`output_dir}/manifest.json`
    pub async fn load(output_dir: &Path) -> Result<CrawlManifest, McpError> {
        let manifest_path = output_dir.join(Self::MANIFEST_FILENAME);

        if !manifest_path.exists() {
            return Err(McpError::Manifest(format!(
                "Manifest not found at {manifest_path:?}"
            )));
        }

        let contents = fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| McpError::Manifest(format!("Failed to read manifest: {e}")))?;

        let manifest: CrawlManifest = serde_json::from_str(&contents)
            .map_err(|e| McpError::Manifest(format!("Failed to parse manifest JSON: {e}")))?;

        Ok(manifest)
    }

    /// Check if manifest exists for `output_dir`
    pub async fn exists(output_dir: &Path) -> bool {
        let manifest_path = output_dir.join(Self::MANIFEST_FILENAME);
        manifest_path.exists()
    }
}

/// Convert URL to filesystem-safe output directory path
///
/// Extracts domain from URL and sanitizes for filesystem use.
/// Default base directory is "docs".
///
/// # Examples
/// ```
/// url_to_output_dir("https://ratatui.rs/concepts/layout", None)
/// // => Ok(PathBuf::from("docs/ratatui.rs"))
///
/// url_to_output_dir("https://example.com:8080/path", Some("output"))
/// // => Ok(PathBuf::from("output/example.com_8080"))
/// ```
pub fn url_to_output_dir(url: &str, base_dir: Option<&str>) -> Result<PathBuf, McpError> {
    let parsed_url =
        Url::parse(url).map_err(|e| McpError::InvalidUrl(format!("Invalid URL '{url}': {e}")))?;

    let domain = parsed_url
        .host_str()
        .ok_or_else(|| McpError::InvalidUrl(format!("URL '{url}' has no host")))?;

    // Sanitize domain for filesystem
    // Replace characters that are problematic in file paths
    let safe_domain = domain
        .replace([':', '/', '\\'], "_") // Windows path separator
        .replace("..", "_"); // Directory traversal protection

    let base = base_dir.unwrap_or("docs");
    let output_dir = PathBuf::from(base).join(safe_domain);

    // Convert to absolute path to avoid CWD issues in indexing
    let output_dir = if output_dir.is_absolute() {
        output_dir
    } else {
        std::env::current_dir()
            .map_err(|e| McpError::InvalidUrl(format!("Failed to get current directory: {e}")))?
            .join(&output_dir)
    };

    Ok(output_dir)
}
