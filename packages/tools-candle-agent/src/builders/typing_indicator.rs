//! Typing indicator builder implementations
//!
//! All typing indicator construction logic and builder patterns.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool};

use atomic_counter::ConsistentCounter;
use crossbeam_skiplist::SkipMap;
use tokio::sync::broadcast;

use crate::domain::chat::realtime::typing::{CandleTypingIndicator as TypingIndicator, CandleTypingState as TypingState};

/// Typing indicator builder for ergonomic configuration
#[derive(Debug, Clone)]
pub struct TypingIndicatorBuilder {
    expiry_duration_secs: u64,
    cleanup_interval_secs: u64,
    event_buffer_size: usize,
}

impl TypingIndicatorBuilder {
    /// Create new builder with defaults
    #[inline]
    pub fn new() -> Self {
        Self {
            expiry_duration_secs: 30,      // 30 seconds default
            cleanup_interval_secs: 10,     // 10 seconds default
            event_buffer_size: 10000,      // Large buffer for performance
        }
    }

    /// Set typing expiry duration
    #[inline]
    pub fn expiry_duration(mut self, seconds: u64) -> Self {
        self.expiry_duration_secs = seconds;
        self
    }

    /// Set cleanup interval
    #[inline]
    pub fn cleanup_interval(mut self, seconds: u64) -> Self {
        self.cleanup_interval_secs = seconds;
        self
    }

    /// Set event buffer size
    #[inline]
    pub fn event_buffer_size(mut self, size: usize) -> Self {
        self.event_buffer_size = size;
        self
    }

    /// Build the typing indicator
    #[inline]
    pub fn build(self) -> TypingIndicator {
        let (event_broadcaster, _) = broadcast::channel(self.event_buffer_size);

        TypingIndicator {
            typing_states: Arc::new(SkipMap::new()),
            expiry_duration_nanos: Arc::new(AtomicU64::new(self.expiry_duration_secs * 1_000_000_000)),
            cleanup_interval_nanos: Arc::new(AtomicU64::new(self.cleanup_interval_secs * 1_000_000_000)),
            event_broadcaster,
            active_users: Arc::new(ConsistentCounter::new(0)),
            typing_events: Arc::new(ConsistentCounter::new(0)),
            cleanup_task_active: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for TypingIndicatorBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}