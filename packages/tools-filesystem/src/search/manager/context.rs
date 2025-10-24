//! Search context for coordinating parallel search operations
//!
//! This module provides the `SearchContext` which bundles all the shared state
//! needed by search visitors to coordinate their work across threads.

use super::super::types::{
    FileCountData, SearchError, SearchOutputMode, SearchResult, SearchSession,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Context for executing searches, containing session state references
pub(super) struct SearchContext {
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) total_files: Arc<AtomicUsize>,
    pub(super) last_read_time_atomic: Arc<AtomicU64>,
    pub(super) is_complete: Arc<AtomicBool>,
    pub(super) is_error: Arc<RwLock<bool>>,
    pub(super) error: Arc<RwLock<Option<String>>>,
    pub(super) cancellation_rx: watch::Receiver<bool>,
    pub(super) first_result_tx: watch::Sender<bool>,
    pub(super) was_incomplete: Arc<RwLock<bool>>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    pub(super) output_mode: SearchOutputMode,
    pub(super) seen_files: Arc<RwLock<HashSet<String>>>,
    pub(super) file_counts: Arc<RwLock<HashMap<String, FileCountData>>>,
    pub(super) start_time: Instant,
}

impl SearchContext {
    /// Create a `SearchContext` from a `SearchSession` and cancellation receiver
    pub(super) fn from_session(
        session: &SearchSession,
        cancellation_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            results: Arc::clone(&session.results),
            total_matches: Arc::clone(&session.total_matches),
            total_files: Arc::clone(&session.total_files),
            last_read_time_atomic: Arc::clone(&session.last_read_time_atomic),
            is_complete: Arc::clone(&session.is_complete),
            is_error: Arc::clone(&session.is_error),
            error: Arc::clone(&session.error),
            cancellation_rx,
            first_result_tx: session.first_result_tx.clone(),
            was_incomplete: Arc::clone(&session.was_incomplete),
            error_count: Arc::clone(&session.error_count),
            errors: Arc::clone(&session.errors),
            output_mode: session.output_mode,
            seen_files: Arc::clone(&session.seen_files),
            file_counts: Arc::clone(&session.file_counts),
            start_time: session.start_time,
        }
    }
}
