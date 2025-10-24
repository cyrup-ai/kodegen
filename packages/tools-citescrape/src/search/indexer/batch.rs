//! Batch indexing configuration and processing logic

use super::super::engine::SearchEngine;
use super::super::types::IndexingPhase;
use super::markdown::process_markdown_content_optimized;
use super::progress::{AtomicProgress, ErrorCollector};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use imstr::ImString;
use log::debug;
use rayon::prelude::*;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tantivy::DateTime as TantivyDateTime;
use tantivy::{IndexWriter, TantivyDocument};

// Thread-local buffer pool for zero-allocation decompression
thread_local! {
    static DECOMPRESSION_BUFFER: std::cell::RefCell<Vec<u8>> =
        std::cell::RefCell::new(Vec::with_capacity(1024 * 1024));
    static MARKDOWN_BUFFER: std::cell::RefCell<String> =
        std::cell::RefCell::new(String::with_capacity(1024 * 1024));
}

/// File size and compression safety limits
#[derive(Debug, Clone)]
pub struct IndexingLimits {
    /// Maximum compressed file size in MB (default: 20)
    pub max_compressed_mb: usize,
    /// Maximum decompressed content size in MB (default: 100)
    pub max_decompressed_mb: usize,
    /// Maximum compression ratio to detect zip bombs (default: 20.0)
    pub max_compression_ratio: f64,
}

impl Default for IndexingLimits {
    fn default() -> Self {
        Self {
            max_compressed_mb: 20,
            max_decompressed_mb: 100,
            max_compression_ratio: 20.0,
        }
    }
}

/// Batch indexing configuration for optimal performance
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Number of files to process in each batch
    pub batch_size: usize,
    /// Maximum number of parallel workers
    pub max_workers: usize,
    /// Enable progress sampling for very large directories
    pub enable_sampling: bool,
    /// Sample rate for progress updates (1 = every file, 10 = every 10th file)
    pub sample_rate: usize,
    /// Maximum errors to collect before stopping
    pub max_errors: usize,
    /// File size and compression safety limits
    pub limits: IndexingLimits,
    /// Cancellation token for aborting long-running operations
    pub cancellation_token: Option<Arc<AtomicBool>>,
}

impl Default for BatchConfig {
    #[inline]
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            batch_size: 100,
            max_workers: cpu_count,
            enable_sampling: true,
            sample_rate: if cpu_count > 8 { 10 } else { 5 },
            max_errors: 1000,
            limits: IndexingLimits::default(),
            cancellation_token: None,
        }
    }
}

impl BatchConfig {
    /// Check if the operation has been cancelled
    #[inline]
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token
            .as_ref()
            .is_some_and(|token| token.load(Ordering::Acquire))
    }
}

/// Context for batch processing operations
pub(crate) struct BatchContext<'a> {
    pub engine: &'a SearchEngine,
    pub progress: &'a Arc<AtomicProgress>,
    pub errors: &'a Arc<ErrorCollector>,
    pub tx: &'a crate::runtime::async_stream::StreamSender<
        Result<super::super::types::IndexProgress>,
        1024,
    >,
    pub start_time: Instant,
    pub config: &'a BatchConfig,
}

/// Process a batch of files with optimal performance (using shared writer)
pub(crate) fn process_batch_with_writer(
    ctx: &BatchContext<'_>,
    batch: Vec<(PathBuf, ImString)>,
    writer: &mut IndexWriter,
) {
    // Sort batch by file size for better load balancing
    let mut batch_with_size: Vec<_> = batch
        .into_iter()
        .filter_map(|(path, url)| std::fs::metadata(&path).ok().map(|m| (path, url, m.len())))
        .collect();

    // Process smallest files first for better progress reporting
    batch_with_size.sort_unstable_by_key(|(_, _, size)| *size);

    // PARALLEL PHASE: Prepare documents concurrently
    let documents: Vec<_> = batch_with_size
        .par_iter()
        .enumerate()
        .filter_map(|(file_idx, (file_path, url, _size))| {
            // Check cancellation FIRST
            if ctx.config.is_cancelled() {
                return None;
            }

            // Sample progress updates to reduce contention
            let should_report = !ctx.config.enable_sampling
                || file_idx % ctx.config.sample_rate == 0
                || file_idx == 0;

            if should_report {
                let current_progress = ctx.progress.snapshot(
                    IndexingPhase::Indexing,
                    ImString::from(file_path.to_string_lossy()),
                    ctx.start_time,
                );
                let _ = ctx.tx.try_send(Ok(current_progress));
            }

            // Prepare document in parallel (CPU-intensive work)
            match prepare_document_from_file(ctx.engine, file_path, url, &ctx.config.limits) {
                Ok(doc) => {
                    ctx.progress.processed.fetch_add(1, Ordering::Relaxed);
                    Some(doc)
                }
                Err(e) => {
                    ctx.progress.failed.fetch_add(1, Ordering::Relaxed);
                    ctx.errors.push(
                        ImString::from(file_path.to_string_lossy()),
                        ImString::from(e.to_string()),
                    );

                    // Check error limit and trigger early termination
                    if ctx.progress.failed.load(Ordering::Relaxed) >= ctx.config.max_errors {
                        debug!(
                            "Error limit reached ({} errors >= {} max), triggering early termination",
                            ctx.progress.failed.load(Ordering::Relaxed),
                            ctx.config.max_errors
                        );
                        // Set cancellation token to stop all parallel workers
                        if let Some(token) = &ctx.config.cancellation_token {
                            token.store(true, Ordering::Release);
                        }
                        return None;
                    }
                    None
                }
            }
        })
        .collect();

    // SEQUENTIAL PHASE: Add documents to writer (fast, ~0.5ms per doc)
    for doc in documents {
        let _ = writer.add_document(doc);
    }
}

/// Prepare a Tantivy document from a file (can be called in parallel)
fn prepare_document_from_file(
    engine: &SearchEngine,
    file_path: &Path,
    url: &ImString,
    limits: &IndexingLimits,
) -> Result<TantivyDocument> {
    // Reuse thread-local buffers (each thread has its own)
    DECOMPRESSION_BUFFER.with(|buffer| {
        MARKDOWN_BUFFER.with(|markdown| {
            let mut buffer = buffer.borrow_mut();
            let mut markdown = markdown.borrow_mut();
            
            // Clear buffers for reuse
            buffer.clear();
            markdown.clear();
            
            // Open file
            let mut file = fs::File::open(file_path)
                .with_context(|| format!("Failed to open file: {file_path:?}"))?;
            
            // Check compressed file size with configured limit
            let metadata = file.metadata()
                .with_context(|| format!("Failed to get metadata: {file_path:?}"))?;
            if metadata.len() > (limits.max_compressed_mb * 1_000_000) as u64 {
                return Err(anyhow::anyhow!(
                    "Compressed file too large: {:?} ({} MB > {} MB limit)",
                    file_path,
                    metadata.len() / 1_000_000,
                    limits.max_compressed_mb
                ));
            }
            
            // Pre-allocate buffer
            let file_size = metadata.len() as usize;
            let current_capacity = buffer.capacity();
            if current_capacity < file_size {
                buffer.reserve(file_size - current_capacity);
            }
            
            // Read file
            file.read_to_end(&mut buffer)
                .with_context(|| format!("Failed to read file: {file_path:?}"))?;
            
            // Check if file is gzipped by magic bytes
            let is_gzipped = buffer.len() >= 2 && &buffer[0..2] == b"\x1f\x8b";
            
            if is_gzipped {
                // Decompress with configured limits and compression ratio checking
                let mut decoder = GzDecoder::new(&buffer[..]);
                let mut decompressed_size = 0;
                let max_decompressed = limits.max_decompressed_mb * 1_000_000;
                let compressed_size = metadata.len() as usize;
                let mut temp_buffer = [0u8; 8192];
                
                loop {
                    match decoder.read(&mut temp_buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            decompressed_size += n;
                            
                            // Check decompressed size
                            if decompressed_size > max_decompressed {
                                return Err(anyhow::anyhow!(
                                    "Decompressed content too large: {:?} ({} MB > {} MB limit)",
                                    file_path,
                                    decompressed_size / 1_000_000,
                                    limits.max_decompressed_mb
                                ));
                            }
                            
                            // Check compression ratio to detect zip bombs
                            let ratio = decompressed_size as f64 / compressed_size as f64;
                            if ratio > limits.max_compression_ratio {
                                return Err(anyhow::anyhow!(
                                    "Suspicious compression ratio: {:?} ({:.1}:1 exceeds {:.1}:1 limit)",
                                    file_path,
                                    ratio,
                                    limits.max_compression_ratio
                                ));
                            }
                            
                            let chunk = std::str::from_utf8(&temp_buffer[..n])
                                .with_context(|| format!("Invalid UTF-8 in file: {file_path:?}"))?;
                            markdown.push_str(chunk);
                        }
                        Err(e) => {
                            return Err(anyhow::Error::new(e)
                                .context(format!("Failed to decompress file: {file_path:?}")));
                        }
                    }
                }
            } else {
                // Plain uncompressed file - read directly
                markdown.push_str(std::str::from_utf8(&buffer)
                    .with_context(|| format!("Invalid UTF-8 in file: {file_path:?}"))?);
            }
            
            // Process markdown
            let processed = process_markdown_content_optimized(&markdown, url, file_path)?;
            
            // Create document
            let mut doc = TantivyDocument::default();
            doc.add_text(engine.schema().url, processed.url.as_str());
            doc.add_text(engine.schema().path, processed.path.as_str());
            doc.add_text(engine.schema().title, processed.title.as_str());
            doc.add_text(engine.schema().raw_markdown, processed.raw_markdown.as_str());
            doc.add_text(engine.schema().plain_content, processed.plain_content.as_str());
            doc.add_text(engine.schema().snippet, processed.snippet.as_str());
            doc.add_date(
                engine.schema().crawl_date,
                TantivyDateTime::from_timestamp_secs(processed.crawl_date.timestamp())
            );
            doc.add_u64(engine.schema().file_size, processed.file_size);
            doc.add_u64(engine.schema().word_count, processed.word_count);
            
            Ok(doc)
        })
    })
}

/// Index a single file synchronously with existing writer (legacy wrapper)
pub(crate) fn index_single_file_sync(
    engine: &SearchEngine,
    writer: &mut IndexWriter,
    file_path: &Path,
    url: &ImString,
    limits: &IndexingLimits,
) -> Result<()> {
    let doc = prepare_document_from_file(engine, file_path, url, limits)?;
    writer
        .add_document(doc)
        .with_context(|| format!("Failed to add document to index: {}", url.as_str()))?;
    Ok(())
}

/// Commit writer and reload reader
pub(crate) async fn commit_and_reload(
    mut writer: IndexWriter,
    engine: &SearchEngine,
    errors: &Arc<ErrorCollector>,
) {
    // Commit using Tantivy's async Future-based API (non-blocking)
    match writer.prepare_commit() {
        Ok(prepared) => {
            match prepared.commit_future().await {
                Ok(_) => {
                    // Wait for merging threads to complete
                    let _ = writer.wait_merging_threads();

                    // Reload reader to see the committed changes
                    if let Err(e) = engine.reader().reload() {
                        errors.push(
                            ImString::from("reader_reload_failed"),
                            ImString::from(format!("Failed to reload reader: {e}")),
                        );
                    }
                }
                Err(e) => {
                    errors.push(
                        ImString::from("commit_future_failed"),
                        ImString::from(format!("Commit future failed: {e}")),
                    );
                }
            }
        }
        Err(e) => {
            errors.push(
                ImString::from("prepare_commit_failed"),
                ImString::from(format!("Prepare commit failed: {e}")),
            );
        }
    }
}

/// Pre-collect file paths for batch processing
pub(crate) fn pre_collect_files(
    directory: &Path,
    _engine: &SearchEngine,
) -> Vec<(PathBuf, ImString)> {
    use super::discovery::discover_markdown_files_stream;

    let mut paths = Vec::new();
    for result in discover_markdown_files_stream(directory) {
        match result {
            Ok((path, url)) => paths.push((path, url)),
            Err(_) => continue, // Skip errors during discovery
        }
    }
    paths
}
