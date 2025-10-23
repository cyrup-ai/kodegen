//! Progress tracking and error collection for indexing operations

use super::super::types::{IndexProgress, IndexingPhase};
use chrono::Utc;
use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use imstr::ImString;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

/// Atomic progress tracker for lock-free updates
pub(crate) struct AtomicProgress {
    pub processed: AtomicUsize,
    pub failed: AtomicUsize,
    pub discovered: AtomicUsize,
    pub discovery_complete: AtomicBool,
}

impl AtomicProgress {
    #[inline]
    pub fn new() -> Self {
        Self {
            processed: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            discovered: AtomicUsize::new(0),
            discovery_complete: AtomicBool::new(false),
        }
    }

    #[inline]
    pub fn snapshot(
        &self,
        phase: IndexingPhase,
        current_file: ImString,
        start_time: Instant,
    ) -> IndexProgress {
        let processed = self.processed.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let discovered = self.discovered.load(Ordering::Relaxed);

        // Calculate ETA using exponential moving average
        let elapsed = start_time.elapsed();
        let estimated_completion = if processed > 0 && discovered > 0 {
            let per_file = elapsed.as_secs_f64() / processed as f64;
            let remaining = (discovered.saturating_sub(processed)) as f64 * per_file;
            Some(Utc::now() + chrono::Duration::seconds(remaining as i64))
        } else {
            None
        };

        IndexProgress {
            processed,
            total: discovered,
            failed,
            current_file,
            phase,
            files_discovered: discovered,
            discovery_complete: self.discovery_complete.load(Ordering::Relaxed),
            errors: Vec::new(), // Errors tracked separately
            started_at: start_time,
            estimated_completion,
        }
    }
}

/// Error collector with lock-free structures for concurrent access
///
/// Uses lock-free data structures to avoid mutex poisoning issues:
/// - `SegQueue` for error storage (lock-free concurrent push)
/// - `DashMap` for error counts (lock-free concurrent `HashMap`)
/// - `AtomicUsize` for overflow tracking
pub(crate) struct ErrorCollector {
    errors: SegQueue<(ImString, ImString)>,
    max_errors: usize,
    overflow_count: AtomicUsize,
    error_counts: DashMap<ImString, AtomicUsize>,
}

impl ErrorCollector {
    #[inline]
    pub fn new() -> Self {
        Self {
            errors: SegQueue::new(),
            max_errors: 1000,
            overflow_count: AtomicUsize::new(0),
            error_counts: DashMap::new(),
        }
    }

    #[inline]
    pub fn push(&self, file: ImString, error: ImString) {
        // Extract error category for statistics
        let error_category = Self::categorize_error(&error);

        // Lock-free increment of error counts using DashMap
        self.error_counts
            .entry(error_category)
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);

        // Lock-free push with overflow tracking
        // Check current queue size to enforce max_errors limit
        if self.errors.len() >= self.max_errors {
            self.overflow_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.errors.push((file, error));
        }
    }

    /// Categorize errors for better reporting
    #[inline]
    fn categorize_error(error: &ImString) -> ImString {
        let error_str = error.as_str();
        let category = if error_str.contains("Failed to open file") {
            "file_not_found"
        } else if error_str.contains("Failed to decompress") {
            "decompression_error"
        } else if error_str.contains("Invalid UTF-8") {
            "encoding_error"
        } else if error_str.contains("too large") {
            "size_limit_exceeded"
        } else if error_str.contains("Permission denied") {
            "permission_denied"
        } else if error_str.contains("tantivy") {
            "index_error"
        } else {
            "other"
        };
        ImString::from(category)
    }

    #[inline]
    pub fn snapshot(&self) -> Vec<(ImString, ImString)> {
        // Lock-free snapshot of error counts using DashMap iteration
        let counts_len = self.error_counts.len();

        // Estimate capacity for result vector
        let estimated_capacity = self.errors.len() + counts_len + 2;
        let mut result = Vec::with_capacity(estimated_capacity);

        // Add summary if there are multiple error types
        if counts_len > 1 {
            let summary = self
                .error_counts
                .iter()
                .map(|entry| {
                    let cat = entry.key();
                    let count = entry.value().load(Ordering::Relaxed);
                    format!("{cat}: {count}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            result.push((
                ImString::from("<summary>"),
                ImString::from(format!("Error counts: {summary}")),
            ));
        }

        // Snapshot errors by draining and re-adding to SegQueue
        // Note: This is atomic at the item level but not transaction-level
        // New errors pushed during snapshot may or may not be included
        let mut errors_snapshot: Vec<(ImString, ImString)> = Vec::new();
        while let Some(error) = self.errors.pop() {
            errors_snapshot.push(error);
        }

        // Re-add all errors back to the queue
        for error in &errors_snapshot {
            self.errors.push(error.clone());
        }

        // Add individual errors to result
        result.extend(errors_snapshot);

        // Add overflow notice
        let overflow = self.overflow_count.load(Ordering::Relaxed);
        if overflow > 0 {
            result.push((
                ImString::from("<overflow>"),
                ImString::from(format!("{overflow} additional errors not shown")),
            ));
        }

        result
    }
}
