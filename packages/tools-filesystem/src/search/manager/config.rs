//! Configuration constants for search operations
//!
//! This module contains all tuning parameters for search performance,
//! buffering, and session lifecycle management.

/// Maximum number of detailed errors to track (prevents memory bloat)
pub(super) const MAX_DETAILED_ERRORS: usize = 100;

/// Size of thread-local result buffer before flushing to shared results
/// Larger = less lock contention, more memory per thread
/// Smaller = more frequent updates, more lock ops
pub(super) const RESULT_BUFFER_SIZE: usize = 50;

/// Minimum time between `last_read_time` updates (prevents excessive lock ops)
pub(super) const LAST_READ_UPDATE_INTERVAL_MS: u64 = 100;

/// Minimum matches between `last_read_time` updates
pub(super) const LAST_READ_UPDATE_MATCH_THRESHOLD: usize = 50;

/// Default maximum results when client doesn't specify a limit
/// Covers 99% of typical use cases while preventing unbounded growth
pub(super) const DEFAULT_MAX_RESULTS: usize = 10_000;

/// Absolute maximum results, even if client requests more
/// Safety valve to prevent server OOM on large searches
pub(super) const ABSOLUTE_MAX_RESULTS: usize = 100_000;

/// Cleanup interval for session management
pub(super) const CLEANUP_INTERVAL_SECS: u64 = 60; // Check every minute

/// Retention period for active searches
pub(super) const ACTIVE_SESSION_RETENTION_SECS: u64 = 5 * 60; // 5 minutes

/// Retention period for completed searches
pub(super) const COMPLETED_SESSION_RETENTION_SECS: u64 = 30; // 30 seconds
