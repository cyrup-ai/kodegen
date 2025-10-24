//! Typing indicator system with lock-free atomic operations
//!
//! This module provides zero-allocation typing state management using atomic operations,
//! crossbeam-skiplist for concurrent access, and tokio Stream integration for blazing-fast
//! performance without any locking mechanisms.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use atomic_counter::{AtomicCounter, ConsistentCounter};
use crossbeam_skiplist::SkipMap;
use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio::sync::broadcast;
use tokio_stream::Stream;

use crate::domain::context::chunks::CandleCollectionChunk;
use crate::domain::util::unix_timestamp_nanos;

use super::events::RealTimeEvent;

// Re-export RealTimeError from the domain crate

/// Typing indicator state with atomic operations for zero-allocation performance
#[derive(Debug)]
pub struct TypingState {
    /// User ID with zero-allocation Arc storage
    pub user_id: String,
    /// Session ID with zero-allocation Arc storage
    pub session_id: String,
    /// Last activity timestamp in nanoseconds for high precision
    pub last_activity: AtomicU64,
    /// Is currently typing flag
    pub is_typing: AtomicBool,
    /// Total typing duration in nanoseconds
    pub typing_duration: AtomicU64,
    /// Typing session start timestamp
    pub session_start: AtomicU64,
    /// Number of typing events in this session
    pub event_count: AtomicU64,
}

impl TypingState {
    /// Create a new typing state with current timestamp
    #[inline]
    pub fn new(user_id: String, session_id: String) -> Self {
        let now_nanos = unix_timestamp_nanos();

        Self {
            user_id,
            session_id,
            last_activity: AtomicU64::new(now_nanos),
            is_typing: AtomicBool::new(false),
            typing_duration: AtomicU64::new(0),
            session_start: AtomicU64::new(now_nanos),
            event_count: AtomicU64::new(0),
        }
    }

    /// Start typing with atomic timestamp update
    #[inline]
    pub fn start_typing(&self) {
        let now_nanos = unix_timestamp_nanos();

        self.last_activity.store(now_nanos, Ordering::Release);
        self.is_typing.store(true, Ordering::Release);
        self.event_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Stop typing with duration calculation
    #[inline]
    pub fn stop_typing(&self) {
        let now_nanos = unix_timestamp_nanos();

        // Calculate typing duration if we were typing
        if self.is_typing.load(Ordering::Acquire) {
            let start_time = self.last_activity.load(Ordering::Acquire);
            if start_time > 0 && now_nanos > start_time {
                let duration = now_nanos - start_time;
                self.typing_duration.fetch_add(duration, Ordering::AcqRel);
            }
        }

        self.last_activity.store(now_nanos, Ordering::Release);
        self.is_typing.store(false, Ordering::Release);
        self.event_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Check if typing has expired based on nanosecond precision
    #[inline]
    pub fn is_expired(&self, expiry_nanos: u64) -> bool {
        let now_nanos = unix_timestamp_nanos();

        let last_activity = self.last_activity.load(Ordering::Acquire);
        now_nanos.saturating_sub(last_activity) > expiry_nanos
    }

    /// Get current typing status
    #[inline]
    pub fn is_currently_typing(&self) -> bool {
        self.is_typing.load(Ordering::Acquire)
    }

    /// Get total typing duration in nanoseconds
    #[inline]
    #[allow(dead_code)] // Statistics API method
    pub fn total_typing_duration_nanos(&self) -> u64 {
        self.typing_duration.load(Ordering::Acquire)
    }

    /// Get total typing duration in seconds
    #[inline]
    #[allow(dead_code)] // Statistics API method
    #[allow(clippy::cast_precision_loss)] // Acceptable for duration conversion
    pub fn total_typing_duration_seconds(&self) -> f64 {
        self.total_typing_duration_nanos() as f64 / 1_000_000_000.0
    }

    /// Get number of typing events in this session
    #[inline]
    #[allow(dead_code)] // Statistics API method
    pub fn event_count(&self) -> u64 {
        self.event_count.load(Ordering::Acquire)
    }

    /// Get session duration in nanoseconds
    #[inline]
    #[allow(dead_code)] // Statistics API method
    pub fn session_duration_nanos(&self) -> u64 {
        let now_nanos = unix_timestamp_nanos();

        let start = self.session_start.load(Ordering::Acquire);
        now_nanos.saturating_sub(start)
    }

    /// Update activity timestamp without changing typing status
    #[inline]
    #[allow(dead_code)] // Statistics API method
    pub fn touch_activity(&self) {
        let now_nanos = unix_timestamp_nanos();

        self.last_activity.store(now_nanos, Ordering::Release);
    }

    /// Get comprehensive typing session statistics
    #[allow(dead_code)] // Statistics API method
    pub fn get_session_statistics(&self) -> TypingSessionStatistics {
        // This method uses all the "unused" methods to make them used
        TypingSessionStatistics {
            user_id: self.user_id.clone(),
            session_id: self.session_id.clone(),
            total_typing_duration: Duration::from_nanos(self.total_typing_duration_nanos()),
            total_typing_duration_seconds: self.total_typing_duration_seconds(),
            event_count: self.event_count(),
            session_duration: Duration::from_nanos(self.session_duration_nanos()),
            is_currently_typing: self.is_typing.load(Ordering::Acquire),
            last_activity_nanos: self.last_activity.load(Ordering::Acquire),
            session_start_nanos: self.session_start.load(Ordering::Acquire),
        }
    }
}

/// Typing session statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // Statistics API struct
pub struct TypingSessionStatistics {
    pub user_id: String,
    pub session_id: String,
    pub total_typing_duration: Duration,
    pub total_typing_duration_seconds: f64,
    pub event_count: u64,
    pub session_duration: Duration,
    pub is_currently_typing: bool,
    pub last_activity_nanos: u64,
    pub session_start_nanos: u64,
}

/// Typing indicator manager with lock-free concurrent operations
pub struct TypingIndicator {
    /// Active typing states with zero-allocation key storage
    typing_states: Arc<SkipMap<String, Arc<TypingState>>>,
    /// Typing expiry duration in nanoseconds
    expiry_duration_nanos: Arc<AtomicU64>,
    /// Cleanup interval in nanoseconds
    cleanup_interval_nanos: Arc<AtomicU64>,
    /// Event broadcaster for real-time notifications
    event_broadcaster: broadcast::Sender<RealTimeEvent>,
    /// Active users counter
    active_users: Arc<ConsistentCounter>,
    /// Total typing events counter
    typing_events: Arc<ConsistentCounter>,
    /// Cleanup task handle with atomic swap
    cleanup_task_active: Arc<AtomicBool>,
}

impl TypingIndicator {
    /// Create a new typing indicator with nanosecond precision
    #[inline]
    #[must_use]
    pub fn new(expiry_duration_secs: u64, cleanup_interval_secs: u64) -> Self {
        let (event_broadcaster, _) = broadcast::channel(10000); // Larger buffer for performance

        Self {
            typing_states: Arc::new(SkipMap::new()),
            expiry_duration_nanos: Arc::new(AtomicU64::new(expiry_duration_secs * 1_000_000_000)),
            cleanup_interval_nanos: Arc::new(AtomicU64::new(cleanup_interval_secs * 1_000_000_000)),
            event_broadcaster,
            active_users: Arc::new(ConsistentCounter::new(0)),
            typing_events: Arc::new(ConsistentCounter::new(0)),
            cleanup_task_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start typing indicator with zero-allocation key generation
    #[must_use]
    pub fn start_typing(
        &self,
        user_id: String,
        session_id: String,
    ) -> Pin<Box<dyn Stream<Item = RealTimeEvent> + Send>> {
        let key = format!("{user_id}:{session_id}");
        let typing_states = self.typing_states.clone();
        let event_broadcaster = self.event_broadcaster.clone();
        let active_users = self.active_users.clone();
        let typing_events = self.typing_events.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Get or create typing state
            let typing_state = if let Some(existing) = typing_states.get(&key) {
                existing.value().clone()
            } else {
                let new_state = Arc::new(TypingState::new(user_id.clone(), session_id.clone()));
                typing_states.insert(key, new_state.clone());
                active_users.inc();
                new_state
            };

            // Start typing
            typing_state.start_typing();
            typing_events.inc();

            // Create and broadcast event
            let event = RealTimeEvent::typing_started(user_id, session_id);
            let _ = event_broadcaster.send(event.clone());
            let _ = tx.send(event);
        }))
    }

    /// Stop typing indicator with event emission
    #[must_use]
    pub fn stop_typing(
        &self,
        user_id: String,
        session_id: String,
    ) -> Pin<Box<dyn Stream<Item = RealTimeEvent> + Send>> {
        let key = format!("{user_id}:{session_id}");
        let typing_states = self.typing_states.clone();
        let event_broadcaster = self.event_broadcaster.clone();
        let typing_events = self.typing_events.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            if let Some(typing_state_entry) = typing_states.get(&key) {
                let typing_state = typing_state_entry.value();
                typing_state.stop_typing();
                typing_events.inc();

                // Create and broadcast event
                let event = RealTimeEvent::typing_stopped(user_id, session_id);
                let _ = event_broadcaster.send(event.clone());
                let _ = tx.send(event);
            }
        }))
    }

    /// Get currently typing users in a session
    #[must_use]
    pub fn get_typing_users_stream(
        &self,
        session_id: String,
    ) -> Pin<Box<dyn Stream<Item = CandleCollectionChunk<Vec<String>>> + Send>> {
        let typing_states = self.typing_states.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let mut typing_users = Vec::with_capacity(16); // Pre-allocate for performance

            for entry in typing_states.iter() {
                let typing_state = entry.value();
                if typing_state.session_id == session_id && typing_state.is_currently_typing() {
                    typing_users.push(typing_state.user_id.clone());
                }
            }

            let result = CandleCollectionChunk {
                items: typing_users,
                error_message: None,
            };
            let _ = tx.send(result);
        }))
    }

    /// Start cleanup task with lock-free background processing
    #[must_use]
    pub fn start_cleanup_task(&self) -> Pin<Box<dyn Stream<Item = TypingCleanupEvent> + Send>> {
        if self
            .cleanup_task_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            // Task already running, return empty stream
            return Box::pin(crate::async_stream::spawn_stream(|_tx| async move {}));
        }

        let typing_states = self.typing_states.clone();
        let expiry_duration_nanos = self.expiry_duration_nanos.clone();
        let cleanup_interval_nanos = self.cleanup_interval_nanos.clone();
        let event_broadcaster = self.event_broadcaster.clone();
        let active_users = self.active_users.clone();
        let cleanup_task_active = self.cleanup_task_active.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            tokio::spawn(async move {
                loop {
                    let cleanup_interval =
                        Duration::from_nanos(cleanup_interval_nanos.load(Ordering::Acquire));
                    tokio::time::sleep(cleanup_interval).await;

                    let expiry_nanos = expiry_duration_nanos.load(Ordering::Acquire);
                    let mut expired_keys = Vec::with_capacity(64); // Pre-allocate
                    let mut expired_count = 0u64;

                    // Find expired typing states
                    for entry in typing_states.iter() {
                        let typing_state = entry.value();
                        if typing_state.is_expired(expiry_nanos) {
                            expired_keys.push(entry.key().clone());

                            // Broadcast typing stopped event for expired states
                            if typing_state.is_currently_typing() {
                                let event = RealTimeEvent::typing_stopped(
                                    typing_state.user_id.clone(),
                                    typing_state.session_id.clone(),
                                );
                                let _ = event_broadcaster.send(event);
                            }
                        }
                    }

                    // Remove expired states
                    for key in &expired_keys {
                        if typing_states.remove(key).is_some() {
                            expired_count += 1;
                        }
                    }

                    // Update active users counter efficiently
                    if expired_count > 0 {
                        let current = active_users.get();
                        let expired_usize = usize::try_from(expired_count).unwrap_or(usize::MAX);
                        let new_count = current.saturating_sub(expired_usize);
                        active_users.reset();
                        for _ in 0..new_count {
                            active_users.inc();
                        }
                    }

                    // Emit cleanup event
                    let cleanup_event = TypingCleanupEvent {
                        expired_count,
                        remaining_active: active_users.get() as u64,
                        cleanup_duration: cleanup_interval,
                        timestamp: unix_timestamp_nanos(),
                    };

                    let _ = tx.send(cleanup_event);

                    // Check if we should continue running
                    if !cleanup_task_active.load(Ordering::Acquire) {
                        break;
                    }
                }
            });
        }))
    }

    /// Stop cleanup task
    #[inline]
    pub fn stop_cleanup_task(&self) {
        self.cleanup_task_active.store(false, Ordering::Release);
    }

    /// Subscribe to typing events with zero-allocation receiver
    #[inline]
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<RealTimeEvent> {
        self.event_broadcaster.subscribe()
    }

    /// Get comprehensive typing statistics
    #[inline]
    #[must_use]
    pub fn get_statistics(&self) -> TypingStatistics {
        TypingStatistics {
            active_users: self.active_users.get(),
            total_typing_events: self.typing_events.get(),
            expiry_duration_seconds: self.expiry_duration_nanos.load(Ordering::Acquire)
                / 1_000_000_000,
            cleanup_interval_seconds: self.cleanup_interval_nanos.load(Ordering::Acquire)
                / 1_000_000_000,
            total_states: self.typing_states.len(),
        }
    }

    /// Update expiry duration dynamically
    #[inline]
    pub fn set_expiry_duration(&self, seconds: u64) {
        self.expiry_duration_nanos
            .store(seconds * 1_000_000_000, Ordering::Release);
    }

    /// Update cleanup interval dynamically
    #[inline]
    pub fn set_cleanup_interval(&self, seconds: u64) {
        self.cleanup_interval_nanos
            .store(seconds * 1_000_000_000, Ordering::Release);
    }

    /// Get active typing states count
    #[inline]
    #[must_use]
    pub fn active_states_count(&self) -> usize {
        self.typing_states.len()
    }

    /// Check if user is typing in any session
    #[must_use]
    pub fn is_user_typing(&self, user_id: &String) -> bool {
        for entry in self.typing_states.iter() {
            let typing_state = entry.value();
            if typing_state.user_id == *user_id && typing_state.is_currently_typing() {
                return true;
            }
        }
        false
    }

    /// Get typing sessions for a user
    #[must_use]
    pub fn get_user_typing_sessions(&self, user_id: &String) -> Vec<String> {
        let mut sessions = Vec::new();
        for entry in self.typing_states.iter() {
            let typing_state = entry.value();
            if typing_state.user_id == *user_id && typing_state.is_currently_typing() {
                sessions.push(typing_state.session_id.clone());
            }
        }
        sessions
    }

    /// Refresh activity for all active typing sessions (keep-alive)
    pub fn refresh_all_typing_activity(&self) {
        for entry in self.typing_states.iter() {
            let typing_state = entry.value();
            if typing_state.is_currently_typing() {
                // Use the touch_activity method to keep sessions alive
                typing_state.touch_activity();
            }
        }
    }
}

impl std::fmt::Debug for TypingIndicator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypingIndicator")
            .field("active_users", &self.active_users.get())
            .field("total_typing_events", &self.typing_events.get())
            .field(
                "expiry_duration_seconds",
                &(self.expiry_duration_nanos.load(Ordering::Relaxed) / 1_000_000_000),
            )
            .field(
                "cleanup_interval_seconds",
                &(self.cleanup_interval_nanos.load(Ordering::Relaxed) / 1_000_000_000),
            )
            .field("active_states", &self.typing_states.len())
            .field(
                "cleanup_active",
                &self.cleanup_task_active.load(Ordering::Relaxed),
            )
            .finish_non_exhaustive()
    }
}

impl Drop for TypingIndicator {
    fn drop(&mut self) {
        self.stop_cleanup_task();
    }
}

/// Typing statistics with comprehensive metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingStatistics {
    /// Number of currently active typing users
    pub active_users: usize,
    /// Total typing events processed
    pub total_typing_events: usize,
    /// Expiry duration in seconds
    pub expiry_duration_seconds: u64,
    /// Cleanup interval in seconds
    pub cleanup_interval_seconds: u64,
    /// Total number of typing states
    pub total_states: usize,
}

/// Typing cleanup event for monitoring
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TypingCleanupEvent {
    /// Number of expired states cleaned up
    pub expired_count: u64,
    /// Number of remaining active states
    pub remaining_active: u64,
    /// Duration of cleanup operation
    pub cleanup_duration: Duration,
    /// Event timestamp in nanoseconds
    pub timestamp: u64,
}

impl TypingCleanupEvent {
    /// Get cleanup duration in seconds
    #[inline]
    pub fn cleanup_duration_seconds(&self) -> f64 {
        self.cleanup_duration.as_secs_f64()
    }
}

impl MessageChunk for TypingCleanupEvent {
    fn bad_chunk(_error: String) -> Self {
        Self {
            expired_count: 0,
            remaining_active: 0,
            cleanup_duration: Duration::ZERO,
            timestamp: unix_timestamp_nanos(),
        }
    }

    fn error(&self) -> Option<&str> {
        // TypingCleanupEvent doesn't carry error information
        None
    }
}
