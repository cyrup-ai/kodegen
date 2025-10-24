//! Command execution context with zero allocation patterns
//!
//! Provides execution context tracking with owned strings allocated once for maximum performance.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::super::metadata::ResourceUsage;
use crate::domain::util::unix_timestamp_micros;

/// Command execution context with owned strings allocated once for performance
#[derive(Debug)]
pub struct CommandExecutionContext {
    /// Unique identifier for this execution
    pub execution_id: u64,
    /// Command name being executed (owned string allocated once)
    pub command_name: String,
    /// Start time in microseconds since epoch
    pub start_time: u64,
    /// Resource usage tracking
    pub resource_usage: ResourceUsage,
    /// User identifier (owned string allocated once)
    pub user_id: Option<String>,
    /// Session identifier (owned string allocated once)
    pub session_id: Option<String>,
    /// Execution environment (owned string allocated once)
    pub environment: String,
    /// Execution priority (1-255, lower is higher priority)
    pub priority: u8,
    /// Maximum allowed execution time in milliseconds
    pub timeout_ms: u64,
    /// Execution metadata
    pub metadata: Option<serde_json::Value>,
    /// Atomic execution counter for sequence numbering
    execution_counter: AtomicU64,
    /// Atomic event counter for event sequence numbering
    event_counter: AtomicUsize,
}

impl CommandExecutionContext {
    /// Create new execution context with current timestamp
    #[inline]
    pub fn new(
        execution_id: u64,
        command_name: impl Into<String>,
        environment: impl Into<String>,
    ) -> Self {
        let start_time = unix_timestamp_micros();

        Self {
            execution_id,
            command_name: command_name.into(),
            start_time,
            resource_usage: ResourceUsage::new_with_start_time(),
            user_id: None,
            session_id: None,
            environment: environment.into(),
            priority: 128,      // Default medium priority
            timeout_ms: 30_000, // Default 30 second timeout
            metadata: None,
            execution_counter: AtomicU64::new(0),
            event_counter: AtomicUsize::new(0),
        }
    }

    /// Set user identifier - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set session identifier - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set execution priority - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set execution timeout - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set metadata - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Get elapsed execution time in microseconds
    #[inline]
    pub fn elapsed_time_us(&self) -> u64 {
        let current_time = unix_timestamp_micros();
        current_time.saturating_sub(self.start_time)
    }

    /// Check if execution has timed out
    #[inline]
    pub fn is_timed_out(&self) -> bool {
        self.elapsed_time_us() > (self.timeout_ms * 1000)
    }

    /// Get remaining time in microseconds before timeout
    #[inline]
    pub fn remaining_time_us(&self) -> u64 {
        let timeout_us = self.timeout_ms * 1000;
        let elapsed = self.elapsed_time_us();
        timeout_us.saturating_sub(elapsed)
    }

    /// Get next execution sequence number (atomic operation)
    #[inline]
    pub fn next_execution_id(&self) -> u64 {
        self.execution_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Get next event sequence number (atomic operation)
    #[inline]
    pub fn next_event_id(&self) -> usize {
        self.event_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Get elapsed time in microseconds (alias for `elapsed_time_us` for compatibility)
    #[inline]
    pub fn elapsed_time(&self) -> u64 {
        self.elapsed_time_us()
    }
}
