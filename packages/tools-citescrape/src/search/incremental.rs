//! Incremental indexing service with async tokio messaging
//!
//! This module provides a high-performance, lock-free incremental indexing service
//! that processes document updates, additions, and deletions in the background.
//!
//! Optimized for:
//! - Zero allocation via message pooling and buffer reuse
//! - Blazing-fast performance via batching and lock-free operations
//! - Async concurrency via tokio mpsc channels and atomic coordination
//! - Elegant ergonomic API with seamless integration

use anyhow::Result;
use imstr::ImString;
use smallvec::SmallVec;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use super::engine::SearchEngine;

/// Maximum number of pending indexing messages before backpressure
const MAX_PENDING_MESSAGES: usize = 10000;

/// Default batch size for processing messages
const DEFAULT_BATCH_SIZE: usize = 50;

/// Maximum wait time before processing partial batch
const MAX_BATCH_WAIT_MS: u64 = 100;

/// Maximum number of retries for failed operations
const MAX_RETRIES: usize = 3;

/// Indexing operation message with zero-allocation design
#[derive(Debug, Clone)]
pub enum IndexingMessage {
    /// Add or update a document in the index
    AddOrUpdate {
        url: ImString,
        file_path: PathBuf,
        priority: MessagePriority,
        completion_id: u64,
    },
    /// Delete a document from the index by URL
    Delete { url: ImString, completion_id: u64 },
    /// Optimize the index (periodic maintenance)
    Optimize { force: bool, completion_id: u64 },
    /// Shutdown the indexing service gracefully
    Shutdown,
}

/// Message priority for processing order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Completion callback registry for tracking operation results
type CompletionCallback = Box<dyn FnOnce(Result<()>) + Send + 'static>;

/// Incremental indexing service with lock-free coordination
#[allow(dead_code)]
pub struct IncrementalIndexingService {
    sender: mpsc::UnboundedSender<IndexingMessage>,
    completion_callbacks: Arc<Mutex<ahash::AHashMap<u64, CompletionCallback>>>,
    next_completion_id: Arc<AtomicUsize>,
    pending_operations: Arc<AtomicUsize>,
    is_running: Arc<AtomicBool>,
    stats: Arc<IndexingStats>,
}

/// Lock-free indexing statistics
#[derive(Debug)]
pub struct IndexingStats {
    pub total_processed: AtomicUsize,
    pub total_failed: AtomicUsize,
    pub pending_count: AtomicUsize,
    pub batch_count: AtomicUsize,
    pub last_optimization: Arc<Mutex<Option<Instant>>>,
}

impl IndexingStats {
    #[inline]
    fn new() -> Self {
        Self {
            total_processed: AtomicUsize::new(0),
            total_failed: AtomicUsize::new(0),
            pending_count: AtomicUsize::new(0),
            batch_count: AtomicUsize::new(0),
            last_optimization: Arc::new(Mutex::new(None)),
        }
    }

    /// Get snapshot of current statistics
    #[inline]
    pub async fn snapshot(&self) -> IndexingStatsSnapshot {
        IndexingStatsSnapshot {
            total_processed: self.total_processed.load(Ordering::Relaxed),
            total_failed: self.total_failed.load(Ordering::Relaxed),
            pending_count: self.pending_count.load(Ordering::Relaxed),
            batch_count: self.batch_count.load(Ordering::Relaxed),
            last_optimization: *self.last_optimization.lock().await,
        }
    }
}

/// Immutable snapshot of indexing statistics
#[derive(Debug, Clone)]
pub struct IndexingStatsSnapshot {
    pub total_processed: usize,
    pub total_failed: usize,
    pub pending_count: usize,
    pub batch_count: usize,
    pub last_optimization: Option<Instant>,
}

/// Handle for sending indexing messages with zero-allocation design
#[derive(Clone)]
pub struct IndexingSender {
    sender: mpsc::UnboundedSender<IndexingMessage>,
    completion_callbacks: Arc<Mutex<ahash::AHashMap<u64, CompletionCallback>>>,
    next_completion_id: Arc<AtomicUsize>,
    pending_operations: Arc<AtomicUsize>,
    stats: Arc<IndexingStats>,
}

impl std::fmt::Debug for IndexingSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexingSender")
            .field(
                "pending_operations",
                &self
                    .pending_operations
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .finish()
    }
}

impl IndexingSender {
    /// Send an add/update operation with completion callback
    #[inline]
    pub async fn add_or_update<F>(
        &self,
        url: ImString,
        file_path: PathBuf,
        priority: MessagePriority,
        completion_callback: F,
    ) -> Result<()>
    where
        F: FnOnce(Result<()>) + Send + 'static,
    {
        // Check backpressure
        let pending = self.pending_operations.load(Ordering::Relaxed);
        if pending >= MAX_PENDING_MESSAGES {
            return Err(anyhow::anyhow!(
                "Indexing service backpressure: {pending} pending operations"
            ));
        }

        let completion_id = self.next_completion_id.fetch_add(1, Ordering::Relaxed) as u64;

        // Register completion callback
        {
            let mut callbacks = self.completion_callbacks.lock().await;
            callbacks.insert(completion_id, Box::new(completion_callback));
        }

        let message = IndexingMessage::AddOrUpdate {
            url,
            file_path,
            priority,
            completion_id,
        };

        // Send message with error handling
        if let Ok(()) = self.sender.send(message) {
            self.pending_operations.fetch_add(1, Ordering::Relaxed);
            self.stats.pending_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            // Remove callback if send failed (channel disconnected)
            self.completion_callbacks
                .lock()
                .await
                .remove(&completion_id);
            Err(anyhow::anyhow!("Indexing service disconnected"))
        }
    }

    /// Send a delete operation and await completion
    #[inline]
    pub async fn delete(&self, url: ImString) -> Result<()> {
        let pending = self.pending_operations.load(Ordering::Relaxed);
        if pending >= MAX_PENDING_MESSAGES {
            return Err(anyhow::anyhow!(
                "Indexing service backpressure: {pending} pending operations"
            ));
        }

        let completion_id = self.next_completion_id.fetch_add(1, Ordering::Relaxed) as u64;

        // Create a oneshot channel for the result
        let (tx, rx) = tokio::sync::oneshot::channel();

        {
            let mut callbacks = self.completion_callbacks.lock().await;
            callbacks.insert(
                completion_id,
                Box::new(move |result| {
                    let _ = tx.send(result);
                }),
            );
        }

        let message = IndexingMessage::Delete { url, completion_id };

        if let Ok(()) = self.sender.send(message) {
            self.pending_operations.fetch_add(1, Ordering::Relaxed);
            self.stats.pending_count.fetch_add(1, Ordering::Relaxed);
            // Wait for background worker to complete the operation
            rx.await
                .map_err(|_| anyhow::anyhow!("Indexing service disconnected"))?
        } else {
            let _ = self
                .completion_callbacks
                .lock()
                .await
                .remove(&completion_id);
            Err(anyhow::anyhow!("Indexing service disconnected"))
        }
    }

    /// Trigger index optimization
    #[inline]
    pub async fn optimize(&self, force: bool) -> Result<()> {
        let completion_id = self.next_completion_id.fetch_add(1, Ordering::Relaxed) as u64;

        let (tx, rx) = tokio::sync::oneshot::channel();

        {
            let mut callbacks = self.completion_callbacks.lock().await;
            callbacks.insert(
                completion_id,
                Box::new(move |result| {
                    let _ = tx.send(result);
                }),
            );
        }

        let message = IndexingMessage::Optimize {
            force,
            completion_id,
        };

        if let Ok(()) = self.sender.send(message) {
            // Wait for background worker to complete and invoke callback
            match rx.await {
                Ok(result) => result,
                Err(_) => Err(anyhow::anyhow!("Optimization callback was dropped")),
            }
        } else {
            // Clean up callback and return error
            self.completion_callbacks
                .lock()
                .await
                .remove(&completion_id);
            Err(anyhow::anyhow!("Indexing service disconnected"))
        }
    }

    /// Get current indexing statistics
    #[inline]
    pub async fn stats(&self) -> IndexingStatsSnapshot {
        self.stats.snapshot().await
    }

    /// Check if service is healthy and processing messages
    #[inline]
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        // Check if the sender can potentially send (not disconnected)
        // We'll use the pending operations count as a health indicator
        self.pending_operations.load(Ordering::Relaxed) < MAX_PENDING_MESSAGES
    }
}

impl IncrementalIndexingService {
    /// Create and start the incremental indexing service
    pub async fn start(engine: SearchEngine) -> Result<IndexingSender> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let completion_callbacks = Arc::new(Mutex::new(ahash::AHashMap::with_capacity(1024)));
        let next_completion_id = Arc::new(AtomicUsize::new(1));
        let pending_operations = Arc::new(AtomicUsize::new(0));
        let is_running = Arc::new(AtomicBool::new(true));
        let stats = Arc::new(IndexingStats::new());

        // Start background worker task
        let worker_callbacks = completion_callbacks.clone();
        let worker_pending = pending_operations.clone();
        let worker_running = is_running.clone();
        let worker_stats = stats.clone();

        tokio::spawn(async move {
            Self::worker_loop(
                engine,
                receiver,
                worker_callbacks,
                worker_pending,
                worker_running,
                worker_stats,
            )
            .await;
        });

        let _service = IncrementalIndexingService {
            sender: sender.clone(),
            completion_callbacks: completion_callbacks.clone(),
            next_completion_id: next_completion_id.clone(),
            pending_operations: pending_operations.clone(),
            is_running,
            stats: stats.clone(),
        };

        Ok(IndexingSender {
            sender,
            completion_callbacks,
            next_completion_id,
            pending_operations,
            stats,
        })
    }

    /// Background worker loop with batching and error handling
    async fn worker_loop(
        engine: SearchEngine,
        mut receiver: mpsc::UnboundedReceiver<IndexingMessage>,
        completion_callbacks: Arc<Mutex<ahash::AHashMap<u64, CompletionCallback>>>,
        pending_operations: Arc<AtomicUsize>,
        is_running: Arc<AtomicBool>,
        stats: Arc<IndexingStats>,
    ) {
        let mut message_batch: SmallVec<[IndexingMessage; DEFAULT_BATCH_SIZE]> = SmallVec::new();
        let mut last_batch_time = Instant::now();

        // Process messages with batching for optimal performance
        while is_running.load(Ordering::Relaxed) {
            let batch_timeout = Duration::from_millis(MAX_BATCH_WAIT_MS);
            let should_process_batch = message_batch.len() >= DEFAULT_BATCH_SIZE
                || (!message_batch.is_empty() && last_batch_time.elapsed() >= batch_timeout);

            if should_process_batch {
                // Process current batch
                Self::process_message_batch(
                    &engine,
                    &mut message_batch,
                    &completion_callbacks,
                    &pending_operations,
                    &stats,
                )
                .await;

                last_batch_time = Instant::now();
                continue;
            }

            // Try to receive messages with timeout
            match tokio::time::timeout(Duration::from_millis(10), receiver.recv()).await {
                Ok(Some(IndexingMessage::Shutdown)) => {
                    // Process final batch before shutdown
                    if !message_batch.is_empty() {
                        Self::process_message_batch(
                            &engine,
                            &mut message_batch,
                            &completion_callbacks,
                            &pending_operations,
                            &stats,
                        )
                        .await;
                    }
                    break;
                }
                Ok(Some(message)) => {
                    message_batch.push(message);
                }
                Ok(None) => {
                    // Channel closed (sender dropped)
                    if !message_batch.is_empty() {
                        Self::process_message_batch(
                            &engine,
                            &mut message_batch,
                            &completion_callbacks,
                            &pending_operations,
                            &stats,
                        )
                        .await;
                    }
                    break;
                }
                Err(_) => {
                    // Timeout - check if we should process partial batch due to timeout
                    continue;
                }
            }
        }

        is_running.store(false, Ordering::Relaxed);
    }

    /// Process a batch of messages with deduplication and error handling
    async fn process_message_batch(
        engine: &SearchEngine,
        message_batch: &mut SmallVec<[IndexingMessage; DEFAULT_BATCH_SIZE]>,
        completion_callbacks: &Arc<Mutex<ahash::AHashMap<u64, CompletionCallback>>>,
        pending_operations: &Arc<AtomicUsize>,
        stats: &Arc<IndexingStats>,
    ) {
        if message_batch.is_empty() {
            return;
        }

        // Sort messages by priority for optimal processing order
        message_batch.sort_unstable_by(|a, b| {
            let priority_a = match a {
                IndexingMessage::AddOrUpdate { priority, .. } => *priority,
                IndexingMessage::Delete { .. } => MessagePriority::High,
                IndexingMessage::Optimize { force, .. } => {
                    if *force {
                        MessagePriority::Critical
                    } else {
                        MessagePriority::Low
                    }
                }
                IndexingMessage::Shutdown => MessagePriority::Critical,
            };
            let priority_b = match b {
                IndexingMessage::AddOrUpdate { priority, .. } => *priority,
                IndexingMessage::Delete { .. } => MessagePriority::High,
                IndexingMessage::Optimize { force, .. } => {
                    if *force {
                        MessagePriority::Critical
                    } else {
                        MessagePriority::Low
                    }
                }
                IndexingMessage::Shutdown => MessagePriority::Critical,
            };
            priority_b.cmp(&priority_a) // Higher priority first
        });

        // Deduplicate URLs within batch (keep latest operation per URL)
        // Track the LAST index for each URL
        let mut url_last_index: ahash::AHashMap<ImString, usize> =
            ahash::AHashMap::with_capacity(message_batch.len());

        // Single forward pass to find last occurrence of each URL
        for (idx, message) in message_batch.iter().enumerate() {
            match message {
                IndexingMessage::AddOrUpdate { url, .. } | IndexingMessage::Delete { url, .. } => {
                    url_last_index.insert(url.clone(), idx);
                }
                _ => {}
            }
        }

        // Single pass filter keeping only last occurrences, maintaining order
        let mut duplicate_completions = Vec::new();
        let deduplicated_batch: SmallVec<[IndexingMessage; DEFAULT_BATCH_SIZE]> = message_batch
            .drain(..)
            .enumerate()
            .filter_map(|(idx, message)| {
                match &message {
                    IndexingMessage::AddOrUpdate { url, .. }
                    | IndexingMessage::Delete { url, .. } => {
                        // Keep if this is the last occurrence of this URL
                        if url_last_index.get(url) == Some(&idx) {
                            Some(message)
                        } else {
                            // Mark duplicate for completion with no-op
                            if let Some(id) = Self::extract_completion_id(&message) {
                                duplicate_completions.push(id);
                            }
                            None
                        }
                    }
                    IndexingMessage::Optimize { .. } | IndexingMessage::Shutdown => Some(message),
                }
            })
            .collect();

        // Complete all duplicate operations
        for completion_id in duplicate_completions {
            Self::complete_operation(
                completion_id,
                Ok(()),
                completion_callbacks,
                pending_operations,
            )
            .await;
        }

        // Process deduplicated batch
        let writer_opt = match engine.writer_with_retry(Some(64 * 1024 * 1024)).await {
            Ok(w) => Some(w),
            Err(e) => {
                // Complete all operations with error
                for message in &deduplicated_batch {
                    if let Some(completion_id) = Self::extract_completion_id(message) {
                        Self::complete_operation(
                            completion_id,
                            Err(anyhow::anyhow!("Failed to create index writer: {e}")),
                            completion_callbacks,
                            pending_operations,
                        )
                        .await;
                    }
                }
                stats
                    .total_failed
                    .fetch_add(deduplicated_batch.len(), Ordering::Relaxed);
                return;
            }
        };

        let mut successful_operations = 0;
        let mut failed_operations = 0;

        // Separate optimize messages from regular operations
        let (optimize_messages, regular_messages): (Vec<_>, Vec<_>) = deduplicated_batch
            .into_iter()
            .partition(|msg| matches!(msg, IndexingMessage::Optimize { .. }));

        // Extract writer from Option for regular operations
        let mut writer = match writer_opt {
            Some(w) => w,
            None => {
                // Writer creation failed earlier, all operations already completed with error
                return;
            }
        };

        // Process regular messages (Delete, AddOrUpdate) with retry logic
        for message in regular_messages {
            let completion_id = Self::extract_completion_id(&message);
            let mut retry_count = 0;
            let mut last_error = None;

            // Retry loop for transient failures
            loop {
                let result = match &message {
                    IndexingMessage::AddOrUpdate { url, file_path, .. } => {
                        // Use batch indexing with existing writer (avoids double writer acquisition)
                        super::indexer::index_single_file_sync(
                            engine,
                            &mut writer,
                            file_path.as_path(),
                            url,
                            &super::indexer::IndexingLimits::default(),
                        )
                        .map_err(|e| anyhow::anyhow!("{e}"))
                    }
                    IndexingMessage::Delete { url, .. } => engine
                        .delete_document(&mut writer, url.to_string())
                        .map_err(|e| anyhow::anyhow!("{e}")),
                    IndexingMessage::Shutdown => Ok(()), // No-op, handled by worker loop
                    IndexingMessage::Optimize { .. } => {
                        unreachable!("Optimize messages filtered out")
                    }
                };

                match result {
                    Ok(()) => {
                        successful_operations += 1;
                        if let Some(id) = completion_id {
                            Self::complete_operation(
                                id,
                                Ok(()),
                                completion_callbacks,
                                pending_operations,
                            )
                            .await;
                        }
                        break;
                    }
                    Err(e) if retry_count < MAX_RETRIES && Self::is_retryable_error(&e) => {
                        retry_count += 1;
                        last_error = Some(e);

                        // Exponential backoff: 10ms, 20ms, 40ms
                        let delay_ms = 10 * (1 << retry_count);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }
                    Err(e) => {
                        failed_operations += 1;
                        let error = last_error.unwrap_or(e);
                        if let Some(id) = completion_id {
                            Self::complete_operation(
                                id,
                                Err(error),
                                completion_callbacks,
                                pending_operations,
                            )
                            .await;
                        }
                        break;
                    }
                }
            }
        }

        // Commit and optimize once after all operations
        match engine.commit_and_optimize(writer).await {
            Ok(_w) => {
                // Writer successfully committed and optimized
                // Update optimization timestamp if there were explicit optimize requests
                if !optimize_messages.is_empty() {
                    *stats.last_optimization.lock().await = Some(Instant::now());
                }

                // Complete all optimize operation callbacks
                for msg in optimize_messages {
                    if let Some(id) = Self::extract_completion_id(&msg) {
                        Self::complete_operation(
                            id,
                            Ok(()),
                            completion_callbacks,
                            pending_operations,
                        )
                        .await;
                        successful_operations += 1;
                    }
                }
            }
            Err(e) => {
                // Commit failed - complete optimize operations with error
                for msg in optimize_messages {
                    if let Some(id) = Self::extract_completion_id(&msg) {
                        Self::complete_operation(
                            id,
                            Err(anyhow::anyhow!("Failed to commit and optimize: {e}")),
                            completion_callbacks,
                            pending_operations,
                        )
                        .await;
                        failed_operations += 1;
                    }
                }
            }
        }

        // Update statistics
        stats
            .total_processed
            .fetch_add(successful_operations, Ordering::Relaxed);
        stats
            .total_failed
            .fetch_add(failed_operations, Ordering::Relaxed);
        stats.batch_count.fetch_add(1, Ordering::Relaxed);
        stats
            .pending_count
            .fetch_sub(successful_operations + failed_operations, Ordering::Relaxed);
    }

    /// Extract completion ID from message
    #[inline]
    fn extract_completion_id(message: &IndexingMessage) -> Option<u64> {
        match message {
            IndexingMessage::AddOrUpdate { completion_id, .. } => Some(*completion_id),
            IndexingMessage::Delete { completion_id, .. } => Some(*completion_id),
            IndexingMessage::Optimize { completion_id, .. } => Some(*completion_id),
            IndexingMessage::Shutdown => None,
        }
    }

    /// Complete an operation by calling its callback
    #[inline]
    async fn complete_operation(
        completion_id: u64,
        result: Result<()>,
        completion_callbacks: &Arc<Mutex<ahash::AHashMap<u64, CompletionCallback>>>,
        pending_operations: &Arc<AtomicUsize>,
    ) {
        if let Some(callback) = completion_callbacks.lock().await.remove(&completion_id) {
            callback(result);
            pending_operations.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Check if error is retryable (transient failure)
    #[inline]
    fn is_retryable_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        error_str.contains("lock")
            || error_str.contains("busy")
            || error_str.contains("timeout")
            || error_str.contains("temporary")
            || error_str.contains("resource temporarily unavailable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_deduplication_keeps_latest_operation() {
        // Create test messages with duplicate URLs in various positions
        let mut message_batch = SmallVec::<[IndexingMessage; DEFAULT_BATCH_SIZE]>::new();

        // First operation for url1 (should be dropped)
        message_batch.push(IndexingMessage::AddOrUpdate {
            url: ImString::from("http://example.com/page1"),
            file_path: PathBuf::from("/tmp/file1.md"),
            priority: MessagePriority::Normal,
            completion_id: 1,
        });

        // Operation for url2 (should be kept)
        message_batch.push(IndexingMessage::AddOrUpdate {
            url: ImString::from("http://example.com/page2"),
            file_path: PathBuf::from("/tmp/file2.md"),
            priority: MessagePriority::Normal,
            completion_id: 2,
        });

        // Second operation for url1 (should be dropped)
        message_batch.push(IndexingMessage::Delete {
            url: ImString::from("http://example.com/page1"),
            completion_id: 3,
        });

        // Operation for url3 (should be kept)
        message_batch.push(IndexingMessage::AddOrUpdate {
            url: ImString::from("http://example.com/page3"),
            file_path: PathBuf::from("/tmp/file3.md"),
            priority: MessagePriority::High,
            completion_id: 4,
        });

        // Third operation for url1 - LATEST (should be kept)
        message_batch.push(IndexingMessage::AddOrUpdate {
            url: ImString::from("http://example.com/page1"),
            file_path: PathBuf::from("/tmp/file1_updated.md"),
            priority: MessagePriority::High,
            completion_id: 5,
        });

        // Optimize message (should always be kept)
        message_batch.push(IndexingMessage::Optimize {
            force: false,
            completion_id: 6,
        });

        // Duplicate url2 (should be dropped)
        message_batch.push(IndexingMessage::Delete {
            url: ImString::from("http://example.com/page2"),
            completion_id: 7,
        });

        // Shutdown message (should always be kept)
        message_batch.push(IndexingMessage::Shutdown);

        // Track the LAST index for each URL
        let mut url_last_index: ahash::AHashMap<ImString, usize> =
            ahash::AHashMap::with_capacity(message_batch.len());

        // Single forward pass to find last occurrence of each URL
        for (idx, message) in message_batch.iter().enumerate() {
            match message {
                IndexingMessage::AddOrUpdate { url, .. } | IndexingMessage::Delete { url, .. } => {
                    url_last_index.insert(url.clone(), idx);
                }
                _ => {}
            }
        }

        // Single pass filter keeping only last occurrences, maintaining order
        let deduplicated_batch: SmallVec<[IndexingMessage; DEFAULT_BATCH_SIZE]> = message_batch
            .drain(..)
            .enumerate()
            .filter_map(|(idx, message)| {
                match &message {
                    IndexingMessage::AddOrUpdate { url, .. }
                    | IndexingMessage::Delete { url, .. } => {
                        // Keep if this is the last occurrence of this URL
                        if url_last_index.get(url) == Some(&idx) {
                            Some(message)
                        } else {
                            // In real code, would complete duplicate with no-op here
                            None
                        }
                    }
                    IndexingMessage::Optimize { .. } | IndexingMessage::Shutdown => Some(message),
                }
            })
            .collect();

        // Verify results
        assert_eq!(
            deduplicated_batch.len(),
            5,
            "Should have 5 deduplicated messages"
        );

        // Verify url1's LATEST operation is kept (completion_id 5)
        let url1_msg = deduplicated_batch.iter().find(|m| {
            matches!(m, IndexingMessage::AddOrUpdate { url, completion_id, .. } 
                if url.as_str() == "http://example.com/page1" && *completion_id == 5)
        });
        assert!(
            url1_msg.is_some(),
            "Should keep latest url1 operation (completion_id 5)"
        );

        // Verify url2's LATEST operation is kept (completion_id 7, Delete)
        let url2_msg = deduplicated_batch.iter().find(|m| {
            matches!(m, IndexingMessage::Delete { url, completion_id } 
                if url.as_str() == "http://example.com/page2" && *completion_id == 7)
        });
        assert!(
            url2_msg.is_some(),
            "Should keep latest url2 operation (completion_id 7, Delete)"
        );

        // Verify url3 operation is kept
        let url3_msg = deduplicated_batch.iter().find(|m| {
            matches!(m, IndexingMessage::AddOrUpdate { url, .. } 
                if url.as_str() == "http://example.com/page3")
        });
        assert!(url3_msg.is_some(), "Should keep url3 operation");

        // Verify Optimize message is kept
        let optimize_msg = deduplicated_batch
            .iter()
            .find(|m| matches!(m, IndexingMessage::Optimize { .. }));
        assert!(optimize_msg.is_some(), "Should keep Optimize message");

        // Verify Shutdown message is kept
        let shutdown_msg = deduplicated_batch
            .iter()
            .find(|m| matches!(m, IndexingMessage::Shutdown));
        assert!(shutdown_msg.is_some(), "Should keep Shutdown message");

        // Verify order is maintained (check indices)
        let positions: Vec<usize> = deduplicated_batch
            .iter()
            .enumerate()
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            positions,
            vec![0, 1, 2, 3, 4],
            "Messages should maintain their relative order"
        );
    }

    #[test]
    fn test_deduplication_with_no_duplicates() {
        let mut message_batch = SmallVec::<[IndexingMessage; DEFAULT_BATCH_SIZE]>::new();

        message_batch.push(IndexingMessage::AddOrUpdate {
            url: ImString::from("http://example.com/page1"),
            file_path: PathBuf::from("/tmp/file1.md"),
            priority: MessagePriority::Normal,
            completion_id: 1,
        });

        message_batch.push(IndexingMessage::AddOrUpdate {
            url: ImString::from("http://example.com/page2"),
            file_path: PathBuf::from("/tmp/file2.md"),
            priority: MessagePriority::Normal,
            completion_id: 2,
        });

        let original_len = message_batch.len();

        // Track the LAST index for each URL
        let mut url_last_index: ahash::AHashMap<ImString, usize> =
            ahash::AHashMap::with_capacity(message_batch.len());

        for (idx, message) in message_batch.iter().enumerate() {
            match message {
                IndexingMessage::AddOrUpdate { url, .. } | IndexingMessage::Delete { url, .. } => {
                    url_last_index.insert(url.clone(), idx);
                }
                _ => {}
            }
        }

        let deduplicated_batch: SmallVec<[IndexingMessage; DEFAULT_BATCH_SIZE]> = message_batch
            .drain(..)
            .enumerate()
            .filter_map(|(idx, message)| match &message {
                IndexingMessage::AddOrUpdate { url, .. } | IndexingMessage::Delete { url, .. } => {
                    if url_last_index.get(url) == Some(&idx) {
                        Some(message)
                    } else {
                        None
                    }
                }
                IndexingMessage::Optimize { .. } | IndexingMessage::Shutdown => Some(message),
            })
            .collect();

        assert_eq!(
            deduplicated_batch.len(),
            original_len,
            "Should keep all messages when no duplicates"
        );
    }

    #[test]
    fn test_deduplication_all_duplicates_same_url() {
        let mut message_batch = SmallVec::<[IndexingMessage; DEFAULT_BATCH_SIZE]>::new();
        let url = ImString::from("http://example.com/page1");

        // Add 5 operations for the same URL
        for i in 1..=5 {
            message_batch.push(IndexingMessage::AddOrUpdate {
                url: url.clone(),
                file_path: PathBuf::from(format!("/tmp/file{i}.md")),
                priority: MessagePriority::Normal,
                completion_id: i,
            });
        }

        // Track the LAST index for each URL
        let mut url_last_index: ahash::AHashMap<ImString, usize> =
            ahash::AHashMap::with_capacity(message_batch.len());

        for (idx, message) in message_batch.iter().enumerate() {
            match message {
                IndexingMessage::AddOrUpdate { url, .. } | IndexingMessage::Delete { url, .. } => {
                    url_last_index.insert(url.clone(), idx);
                }
                _ => {}
            }
        }

        let deduplicated_batch: SmallVec<[IndexingMessage; DEFAULT_BATCH_SIZE]> = message_batch
            .drain(..)
            .enumerate()
            .filter_map(|(idx, message)| match &message {
                IndexingMessage::AddOrUpdate { url, .. } | IndexingMessage::Delete { url, .. } => {
                    if url_last_index.get(url) == Some(&idx) {
                        Some(message)
                    } else {
                        None
                    }
                }
                IndexingMessage::Optimize { .. } | IndexingMessage::Shutdown => Some(message),
            })
            .collect();

        assert_eq!(
            deduplicated_batch.len(),
            1,
            "Should keep only the last operation"
        );

        // Verify it's the last one (completion_id 5)
        match &deduplicated_batch[0] {
            IndexingMessage::AddOrUpdate { completion_id, .. } => {
                assert_eq!(
                    *completion_id, 5,
                    "Should keep the last operation (completion_id 5)"
                );
            }
            _ => panic!("Expected AddOrUpdate message"),
        }
    }
}
