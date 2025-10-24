//! Download coordination to prevent concurrent downloads of same file
//!
//! Uses global `DashMap` to coordinate downloads across all workers.
//! Pattern follows existing usage in pool.rs and macros.rs.

use dashmap::DashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

/// Global registry of download locks keyed by "repo/filename"
///
/// Ensures only one download happens at a time per file across all workers.
/// When multiple workers spawn simultaneously, first acquires lock and downloads,
/// others block until download completes then use cached file.
///
/// # Pattern
/// Follows same `DashMap` pattern as:
/// - pool.rs: `workers: DashMap<String, Vec<W>>`
/// - macros.rs: `recording_sessions: DashMap<Uuid, MacroRecordingSession>`
static DOWNLOAD_LOCKS: LazyLock<DashMap<String, Arc<Mutex<()>>>> = LazyLock::new(DashMap::new);

/// Acquire download lock for specific `HuggingFace` file
///
/// # Arguments
/// * `repo` - Repository identifier (e.g., "unsloth/Qwen3-1.7B-GGUF")
/// * `filename` - File name within repo (e.g., "model.gguf")
///
/// # Returns
/// Arc to Mutex that coordinates downloads. Hold lock while downloading.
///
/// # Example
/// ```rust
/// let lock = acquire_download_lock("unsloth/Qwen3-1.7B-GGUF", "model.gguf").await;
/// let _guard = lock.lock().await;  // Blocks if another worker is downloading
/// // ... download file ...
/// // _guard drops here, releasing lock
/// ```
pub async fn acquire_download_lock(repo: &str, filename: &str) -> Arc<Mutex<()>> {
    let key = format!("{repo}/{filename}");

    DOWNLOAD_LOCKS
        .entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .value()
        .clone()
}
