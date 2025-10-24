//! Search manager for coordinating streaming search operations
//!
//! This module provides the main `SearchManager` API for starting, managing,
//! and retrieving results from background search tasks.

use super::super::types::{
    GetMoreSearchResultsResponse, SearchSession, SearchSessionInfo, SearchSessionOptions,
    SearchType, StartSearchResponse,
};
use super::config::{
    ABSOLUTE_MAX_RESULTS, ACTIVE_SESSION_RETENTION_SECS, CLEANUP_INTERVAL_SECS,
    COMPLETED_SESSION_RETENTION_SECS, DEFAULT_MAX_RESULTS,
};
use super::context::SearchContext;
use crate::validate_path;
use kodegen_mcp_tool::error::McpError;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, watch};
use uuid::Uuid;

/// Manager for streaming search operations
#[derive(Clone)]
pub struct SearchManager {
    sessions: Arc<RwLock<HashMap<String, SearchSession>>>,
    config_manager: kodegen_tools_config::ConfigManager,
}

impl SearchManager {
    /// Create a new search manager
    #[must_use]
    pub fn new(config_manager: kodegen_tools_config::ConfigManager) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config_manager,
        }
    }

    /// Start a new streaming search session
    ///
    /// # Errors
    /// Returns error if path validation fails or search cannot be started
    pub async fn start_search(
        &self,
        mut options: SearchSessionOptions,
    ) -> Result<StartSearchResponse, McpError> {
        // Enforce result limits before starting search
        let effective_max_results = match options.max_results {
            None => {
                // No limit specified - use default
                options.max_results = Some(DEFAULT_MAX_RESULTS as u32);
                DEFAULT_MAX_RESULTS
            }
            Some(requested) => {
                // Limit specified - cap at absolute maximum
                let capped = requested.min(ABSOLUTE_MAX_RESULTS as u32);
                if capped < requested {
                    log::warn!(
                        "Search max_results capped from {requested} to {capped} (absolute limit)"
                    );
                }
                options.max_results = Some(capped);
                capped as usize
            }
        };

        log::debug!("Starting search with effective max_results: {effective_max_results}");

        // Generate unique session ID using UUID v4 with collision detection
        let mut collision_count = 0;

        let session_id = loop {
            let id = Uuid::new_v4().to_string();

            let sessions = self.sessions.read().await;
            if !sessions.contains_key(&id) {
                drop(sessions);
                break id;
            }

            // This should NEVER happen
            collision_count += 1;
            log::error!(
                "UUID v4 collision #{collision_count} detected: {id}. This indicates a serious problem with the RNG!"
            );

            if collision_count >= 10 {
                // If we've tried 10 times and all collided, something is seriously wrong
                return Err(McpError::Other(anyhow::anyhow!(
                    "Unable to generate unique session ID after 10 attempts. System RNG may be compromised."
                )));
            }
        };

        // Validate path
        let validated_path = validate_path(&options.root_path, &self.config_manager).await?;

        // Create cancellation channel (false = not cancelled initially)
        let (cancellation_tx, cancellation_rx) = watch::channel(false);

        // Create first-result notification channel
        let (first_result_tx, mut first_result_rx) = watch::channel(false);

        // Create session
        let session = SearchSession {
            id: session_id.clone(),
            cancellation_tx,
            first_result_tx,
            results: Arc::new(RwLock::new(Vec::new())),
            is_complete: Arc::new(AtomicBool::new(false)),
            is_error: Arc::new(RwLock::new(false)),
            error: Arc::new(RwLock::new(None)),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            last_read_time_atomic: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            was_incomplete: Arc::new(RwLock::new(false)),
            search_type: options.search_type.clone(),
            pattern: options.pattern.clone(),
            timeout_ms: options.timeout_ms,
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            max_results: effective_max_results,
            output_mode: options.output_mode,
            seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
            file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
        };

        // Store session
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);

        // Capture sort options before moving options into spawn_search_task
        let sort_by = options.sort_by;
        let sort_direction = options.sort_direction;

        // Spawn background search task
        self.spawn_search_task(session_id.clone(), options, validated_path, cancellation_rx);

        // If sorting is enabled, wait for search to complete before returning
        if sort_by.is_some() {
            // Wait for search to complete or timeout
            let timeout_duration = Duration::from_secs(30); // Max 30s wait for sorting
            let wait_start = Instant::now();

            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;

                let sessions = self.sessions.read().await;
                let session = sessions
                    .get(&session_id)
                    .ok_or_else(|| McpError::Other(anyhow::anyhow!("Session lost during wait")))?;

                let is_complete = session.is_complete.load(Ordering::Acquire);
                let is_error = *session.is_error.read().await;

                if is_complete || is_error || wait_start.elapsed() >= timeout_duration {
                    break;
                }
            }

            // Apply sorting to results
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&session_id)
                .ok_or_else(|| McpError::Other(anyhow::anyhow!("Session lost after search")))?;

            let mut results = session.results.write().await;

            if let Some(sort_criterion) = sort_by {
                use crate::search::sorting::{
                    SortBy as SortCriterion, SortDirection as SortDir, sort_results,
                };
                use crate::search::types::{SortBy, SortDirection};

                let sort_by_criterion = match sort_criterion {
                    SortBy::Path => SortCriterion::Path,
                    SortBy::Modified => SortCriterion::Modified,
                    SortBy::Accessed => SortCriterion::Accessed,
                    SortBy::Created => SortCriterion::Created,
                };

                let sort_dir = match sort_direction.unwrap_or(SortDirection::Ascending) {
                    SortDirection::Ascending => SortDir::Ascending,
                    SortDirection::Descending => SortDir::Descending,
                };

                sort_results(&mut results, sort_by_criterion, sort_dir);
            }
        } else {
            // No sorting: wait for first result OR 40ms timeout (streaming mode)
            use tokio::time::timeout;
            let _ = timeout(Duration::from_millis(40), first_result_rx.changed()).await;
        }

        // Return initial state
        let sessions = self.sessions.read().await;
        let session = sessions.get(&session_id).ok_or_else(|| {
            McpError::Other(anyhow::anyhow!("Session lost during initialization"))
        })?;

        // Read status fields
        let is_complete = session.is_complete.load(Ordering::Acquire);
        let is_error = *session.is_error.read().await;

        // Lock coupling: Hold results lock while reading total_matches and taking initial slice
        // This ensures the returned results and total_matches are consistent snapshots
        let (initial_results, total_matches) = {
            let results = session.results.read().await;
            let total_matches = session.total_matches.load(Ordering::SeqCst);
            let initial_results = results.iter().take(10).cloned().collect();
            (initial_results, total_matches)
        };

        // Check if results were limited
        let results_limited = if total_matches >= effective_max_results {
            Some(true)
        } else {
            None
        };

        Ok(StartSearchResponse {
            session_id: session_id.clone(),
            is_complete,
            is_error,
            results: initial_results,
            total_results: total_matches,
            runtime_ms: u64::try_from(session.start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
            error_count: session.error_count.load(Ordering::SeqCst),
            max_results: effective_max_results,
            results_limited,
        })
    }

    /// Spawn background search task using ripgrep libraries
    fn spawn_search_task(
        &self,
        session_id: String,
        options: SearchSessionOptions,
        root: PathBuf,
        cancellation_rx: watch::Receiver<bool>,
    ) {
        let sessions = Arc::clone(&self.sessions);
        let timeout_duration = options.timeout_ms.map(Duration::from_millis);

        // Spawn the actual search task
        let search_handle = tokio::task::spawn_blocking({
            let sessions = Arc::clone(&sessions);
            let session_id = session_id.clone();
            move || {
                // Get session references and create context
                let (mut ctx, search_type) = {
                    let sessions_guard = sessions.blocking_read();
                    if let Some(session) = sessions_guard.get(&session_id) {
                        let ctx = SearchContext::from_session(session, cancellation_rx);
                        (ctx, session.search_type.clone())
                    } else {
                        return; // Session not found
                    }
                };

                // Branch based on list_files_only or search type
                if options.list_files_only {
                    super::files_mode::execute(&options, &root, &mut ctx);
                } else if search_type == SearchType::Content {
                    super::content_search::execute(&options, &root, &mut ctx);
                } else {
                    super::file_search::execute(&options, &root, &mut ctx);
                }
            }
        });

        // If timeout is specified, spawn a monitoring task
        if let Some(timeout) = timeout_duration {
            tokio::spawn(async move {
                // Wait for either search completion or timeout
                let timeout_result = tokio::time::timeout(timeout, search_handle).await;

                match timeout_result {
                    Ok(_) => {
                        // Search completed before timeout - nothing to do
                    }
                    Err(_elapsed) => {
                        // Timeout occurred - send cancellation signal
                        log::warn!("Search session {session_id} timed out");

                        let sessions_guard = sessions.read().await;
                        if let Some(session) = sessions_guard.get(&session_id) {
                            // Only proceed if session still exists
                            let _ = session.cancellation_tx.send(true);

                            // Use try_write to avoid blocking
                            if let Ok(mut incomplete) = session.was_incomplete.try_write() {
                                *incomplete = true;
                            }
                        } else {
                            log::debug!(
                                "Timeout fired but session {session_id} already cleaned up"
                            );
                        }
                    }
                }
            });
        } else {
            // No timeout - just detach the search task
            tokio::spawn(async move {
                let _ = search_handle.await;
            });
        }
    }

    /// Terminate a search session by sending cancellation signal
    ///
    /// Sends cancellation signal to the blocking task, causing it to exit cleanly at next loop iteration.
    ///
    /// # Errors
    /// Returns error if session cannot be accessed (should not occur in practice)
    pub async fn terminate_search(&self, session_id: &str) -> Result<bool, McpError> {
        let sessions = self.sessions.read().await;

        let Some(session) = sessions.get(session_id) else {
            return Ok(false); // Session not found
        };

        if session.is_complete.load(Ordering::Acquire) {
            return Ok(false); // Already complete, nothing to cancel
        }

        // Send cancellation signal - This actually stops the task!
        if let Ok(()) = session.cancellation_tx.send(true) {
            log::info!("Sent cancellation signal to search session: {session_id}");
            Ok(true) // Signal sent successfully
        } else {
            // Channel closed means receiver dropped (task already finished)
            log::debug!("Search session {session_id} already finished");
            Ok(false)
        }
    }

    /// Get paginated results from an active search session
    ///
    /// # Errors
    /// Returns error if session not found
    pub async fn get_more_results(
        &self,
        session_id: &str,
        offset: i64,
        length: usize,
    ) -> Result<GetMoreSearchResultsResponse, McpError> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id).ok_or_else(|| {
            McpError::InvalidArguments(format!("Search session {session_id} not found"))
        })?;

        // Read status fields first (no lock coupling needed for these)
        let is_complete = session.is_complete.load(Ordering::Acquire);
        let is_error = *session.is_error.read().await;
        let error = session.error.read().await.clone();
        let was_incomplete = *session.was_incomplete.read().await;

        // Lock coupling: Hold results lock while reading total_matches and performing slicing
        // This ensures total_results and total_matches are consistent snapshots from the same instant
        let (sliced_results, total_results, total_matches, has_more) = {
            let results = session.results.read().await; // Acquire read lock

            // Capture both counts WHILE holding lock
            let total_results = results.len();
            let total_matches = session.total_matches.load(Ordering::SeqCst);

            // Perform slicing WHILE holding lock (ensures consistency)
            let (sliced, has_more) = if offset < 0 {
                // Negative offset: tail slicing - O(tail_count)
                let tail_count = usize::try_from(-offset).unwrap_or(0);

                // Direct slice from end - no iterator overhead
                let start = results.len().saturating_sub(tail_count);
                let tail_results = results[start..].to_vec();

                (tail_results, false)
            } else {
                // Positive offset: range slicing - O(length)
                let start = usize::try_from(offset).unwrap_or(0);

                if start >= results.len() {
                    // Start past end - return empty
                    (Vec::new(), !is_complete)
                } else {
                    // Direct slice - no iterator overhead
                    let end = start.saturating_add(length).min(results.len());
                    let sliced = results[start..end].to_vec();
                    let has_more = end < results.len() || !is_complete;

                    (sliced, has_more)
                }
            };

            (sliced, total_results, total_matches, has_more)
        }; // Read lock released here - all values are consistent

        let elapsed_micros = session.start_time.elapsed().as_micros() as u64;
        session
            .last_read_time_atomic
            .store(elapsed_micros, Ordering::Relaxed);

        let error_count = session.error_count.load(Ordering::SeqCst);
        let errors = session.errors.read().await.clone();

        // Check if results were limited by max_results
        let results_limited = if total_matches >= session.max_results {
            Some(true)
        } else {
            None
        };

        Ok(GetMoreSearchResultsResponse {
            session_id: session_id.to_string(),
            results: sliced_results.clone(),
            returned_count: sliced_results.len(),
            total_results,
            total_matches,
            is_complete,
            is_error: is_error && error.is_some(),
            error,
            has_more_results: has_more,
            runtime_ms: u64::try_from(session.start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
            was_incomplete: if was_incomplete { Some(true) } else { None },
            error_count,
            errors,
            results_limited,
        })
    }

    /// List all active search sessions
    pub async fn list_active_sessions(&self) -> Vec<SearchSessionInfo> {
        let sessions = self.sessions.read().await;
        let mut result = Vec::new();

        for session in sessions.values() {
            let is_complete = session.is_complete.load(Ordering::Acquire);
            let is_error = *session.is_error.read().await;
            let total_results = session.results.read().await.len();

            result.push(SearchSessionInfo {
                id: session.id.clone(),
                search_type: match session.search_type {
                    SearchType::Files => "files".to_string(),
                    SearchType::Content => "content".to_string(),
                },
                pattern: session.pattern.clone(),
                is_complete,
                is_error,
                runtime_ms: u64::try_from(session.start_time.elapsed().as_millis())
                    .unwrap_or(u64::MAX),
                total_results,
                timeout_ms: session.timeout_ms,
                was_incomplete: if *session.was_incomplete.read().await {
                    Some(true)
                } else {
                    None
                },
            });
        }

        result
    }

    /// Clean up old completed sessions with differentiated retention.
    ///
    /// Removes sessions based on completion status:
    /// - Completed searches: 30 seconds retention
    /// - Active searches: 5 minutes retention
    ///
    /// Recently-read sessions are preserved regardless of completion status.
    pub async fn cleanup_sessions(&self) {
        let now = Instant::now();

        // Calculate different cutoff times for active vs completed searches
        let active_cutoff = now
            .checked_sub(Duration::from_secs(ACTIVE_SESSION_RETENTION_SECS))
            .unwrap_or(now);

        let completed_cutoff = now
            .checked_sub(Duration::from_secs(COMPLETED_SESSION_RETENTION_SECS))
            .unwrap_or(now);

        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();

        sessions.retain(|session_id, session| {
            // LOCK-FREE atomic loads
            let is_complete = session.is_complete.load(Ordering::Acquire);

            // Convert stored elapsed micros back to Instant
            let elapsed_micros = session.last_read_time_atomic.load(Ordering::Relaxed);
            let last_read = session
                .start_time
                .checked_add(Duration::from_micros(elapsed_micros))
                .unwrap_or(now);

            // Differentiated retention based on completion status
            let should_keep = if is_complete {
                // Completed searches: shorter retention (30 seconds)
                last_read > completed_cutoff
            } else {
                // Active searches: longer retention (5 minutes)
                last_read > active_cutoff
            };

            if !should_keep {
                let reason = if is_complete {
                    "completed and inactive for 30s"
                } else {
                    "active but no reads for 5min"
                };
                log::debug!("Cleaning up search session {session_id}: {reason}");
            }

            should_keep
        });

        let cleaned_count = initial_count - sessions.len();
        if cleaned_count > 0 {
            log::info!("Cleaned up {cleaned_count} search sessions");
        }
    }

    /// Start background cleanup task (call once on manager creation)
    ///
    /// Runs cleanup every minute with differentiated retention:
    /// - Active searches: 5 minutes retention
    /// - Completed searches: 30 seconds retention
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
            loop {
                interval.tick().await;
                self.cleanup_sessions().await;
            }
        });
    }
}
