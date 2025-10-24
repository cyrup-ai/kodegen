//! Document indexing pipeline for markdown content
//!
//! This module handles the processing and indexing of markdown documents,
//! including decompression, content extraction, and dual indexing.
//!
//! Optimized for:
//! - Zero allocation where possible
//! - Blazing-fast performance
//! - Lock-free concurrent operations
//! - Elegant, ergonomic API

mod batch;
mod discovery;
mod markdown;
mod progress;

pub(crate) use batch::index_single_file_sync;
pub use batch::{BatchConfig, IndexingLimits};

use super::engine::SearchEngine;
use super::types::{IndexProgress, IndexingPhase};
use crate::runtime::AsyncStream;
use ahash::AHashSet;
use anyhow::Result;
use batch::BatchContext;
use imstr::ImString;
use progress::{AtomicProgress, ErrorCollector};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Handle for cancelling a long-running batch indexing operation
#[derive(Clone)]
pub struct CancellationHandle {
    token: Arc<AtomicBool>,
}

impl CancellationHandle {
    /// Cancel the associated indexing operation
    pub fn cancel(&self) {
        self.token.store(true, Ordering::Release);
    }

    /// Check if the operation has been cancelled
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.token.load(Ordering::Acquire)
    }
}

/// Markdown document indexer with blazing-fast performance
#[derive(Clone)]
pub struct MarkdownIndexer {
    engine: SearchEngine,
}

impl MarkdownIndexer {
    /// Create a new markdown indexer
    #[inline]
    #[must_use]
    pub fn new(engine: SearchEngine) -> Self {
        Self { engine }
    }

    /// Discover markdown files with zero-allocation streaming
    pub fn discover_markdown_files_stream<'a>(
        &'a self,
        root_dir: &'a Path,
    ) -> impl Iterator<Item = Result<(PathBuf, ImString)>> + 'a {
        discovery::discover_markdown_files_stream(root_dir)
    }

    /// Batch index all markdown files in a directory with blazing-fast performance
    #[must_use]
    pub fn batch_index_directory(
        &self,
        directory: PathBuf,
        config: BatchConfig,
    ) -> (AsyncStream<Result<IndexProgress>, 1024>, CancellationHandle) {
        let engine = self.engine.clone();
        let (tx, stream) = AsyncStream::channel();

        // Create cancellation token
        let cancel_token = Arc::new(AtomicBool::new(false));
        let cancel_handle = CancellationHandle {
            token: cancel_token.clone(),
        };

        // Pass token to config
        let mut config = config;
        config.cancellation_token = Some(cancel_token);

        // Pre-collect file paths for Send-safety - this eliminates the iterator lifetime issue
        let file_paths = batch::pre_collect_files(&directory, &engine);

        // Use runtime for async execution with owned, Send-safe data
        tokio::spawn(async move {
            let start_time = Instant::now();
            let progress = Arc::new(AtomicProgress::new());
            let errors = Arc::new(ErrorCollector::new());

            // Get a single writer for all batches (Tantivy only allows one writer at a time)
            // Get writer with retry logic to handle transient lock failures
            let mut writer = match engine.writer_with_retry(Some(128 * 1024 * 1024)).await {
                Ok(w) => w,
                Err(e) => {
                    let _ =
                        tx.try_send(Err(anyhow::anyhow!("Failed to acquire index writer: {e}")));
                    return;
                }
            };

            // Discovery phase with deduplication
            let estimated_files = file_paths.len();
            let mut seen_urls = AHashSet::<ImString>::with_capacity(estimated_files.min(100000));
            let mut file_batch = Vec::<(PathBuf, ImString)>::with_capacity(config.batch_size);

            // Process pre-collected files for zero-allocation discovery
            for (path, url) in file_paths {
                // Check for cancellation
                if config.is_cancelled() {
                    // Send cancellation progress
                    let cancel_progress = progress.snapshot(
                        IndexingPhase::Cancelled,
                        ImString::from("Operation cancelled by user"),
                        start_time,
                    );
                    let _ = tx.try_send(Ok(cancel_progress));
                    return;
                }

                if seen_urls.insert(url.clone()) {
                    file_batch.push((path, url));
                    progress.discovered.fetch_add(1, Ordering::Relaxed);

                    // Process batch when full
                    if file_batch.len() >= config.batch_size {
                        let batch = std::mem::replace(
                            &mut file_batch,
                            Vec::with_capacity(config.batch_size),
                        );
                        let ctx = BatchContext {
                            engine: &engine,
                            progress: &progress,
                            errors: &errors,
                            tx: &tx,
                            start_time,
                            config: &config,
                        };
                        batch::process_batch_with_writer(&ctx, batch, &mut writer);

                        // Check if batch processing triggered cancellation (e.g., max_errors exceeded)
                        if config.is_cancelled() {
                            tracing::warn!(
                                "Stopping batch processing: max_errors ({}) exceeded",
                                config.max_errors
                            );
                            break; // Exit the for loop that processes file_paths
                        }
                    }
                }
            }

            // Send discovery complete progress with actual file count
            let disc_count = progress.discovered.load(Ordering::Relaxed);
            let discovery_progress = progress.snapshot(
                IndexingPhase::Discovering,
                ImString::from(format!("Discovered {disc_count} files")),
                start_time,
            );
            if tx.try_send(Ok(discovery_progress)).is_err() {
                return;
            }

            // Mark discovery complete
            progress.discovery_complete.store(true, Ordering::Relaxed);

            // Check for cancellation before final batch
            if config.is_cancelled() {
                let cancel_progress = progress.snapshot(
                    IndexingPhase::Cancelled,
                    ImString::from("Operation cancelled by user"),
                    start_time,
                );
                let _ = tx.try_send(Ok(cancel_progress));
                return;
            }

            // Process final batch
            if !file_batch.is_empty() {
                let ctx = BatchContext {
                    engine: &engine,
                    progress: &progress,
                    errors: &errors,
                    tx: &tx,
                    start_time,
                    config: &config,
                };
                batch::process_batch_with_writer(&ctx, file_batch, &mut writer);
            }

            // Commit and reload only if not cancelled
            if config.is_cancelled() {
                let cancel_progress = progress.snapshot(
                    IndexingPhase::Cancelled,
                    ImString::from("Operation cancelled by user"),
                    start_time,
                );
                let _ = tx.try_send(Ok(cancel_progress));
                return;
            }
            batch::commit_and_reload(writer, &engine, &errors).await;

            // Final optimization phase
            let mut final_progress = progress.snapshot(
                IndexingPhase::Optimizing,
                ImString::from("Optimizing index..."),
                start_time,
            );
            if tx.try_send(Ok(final_progress.clone())).is_err() {
                return;
            }

            // Send completion
            final_progress.phase = IndexingPhase::Complete;
            final_progress.current_file = ImString::from(format!(
                "Indexing complete: {} succeeded, {} failed",
                progress.processed.load(Ordering::Relaxed),
                progress.failed.load(Ordering::Relaxed)
            ));
            final_progress.errors = errors.snapshot();

            let _ = tx.try_send(Ok(final_progress));
        });

        (stream, cancel_handle)
    }

    /// Index a single file with zero-allocation optimizations
    async fn index_single_file_optimized(
        engine: &SearchEngine,
        file_path: &Path,
        url: &ImString,
    ) -> Result<()> {
        let engine = engine.clone();
        let file_path = file_path.to_path_buf();
        let url = url.clone();

        // Get writer with retry logic to handle transient lock failures
        let mut writer = engine
            .writer_with_retry(Some(64 * 1024 * 1024))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to acquire index writer: {e}"))?;

        // Index the file (synchronous)
        batch::index_single_file_sync(
            &engine,
            &mut writer,
            &file_path,
            &url,
            &batch::IndexingLimits::default(),
        )?;

        // Commit using Tantivy's async Future-based API (non-blocking)
        let prepared = writer
            .prepare_commit()
            .map_err(|e| anyhow::anyhow!("Failed to prepare commit: {e}"))?;
        prepared
            .commit_future()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to commit: {e}"))?;

        // Reload reader to see changes
        engine
            .reader()
            .reload()
            .map_err(|e| anyhow::anyhow!("Failed to reload reader: {e}"))?;

        Ok(())
    }

    /// Index a single markdown file
    pub async fn index_file(&self, file_path: &Path, url: &ImString) -> Result<()> {
        let engine = self.engine.clone();
        let file_path = file_path.to_path_buf();
        let url = url.clone();

        Self::index_single_file_optimized(&engine, &file_path, &url).await
    }

    /// Get index statistics
    #[inline]
    pub async fn get_index_stats(&self) -> Result<super::engine::IndexStats> {
        self.engine.get_stats().await
    }
}
