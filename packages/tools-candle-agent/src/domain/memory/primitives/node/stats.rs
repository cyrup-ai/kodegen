use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::SystemTime;

use crate::domain::util::unix_timestamp_nanos;

/// Atomic access statistics for concurrent monitoring
#[derive(Debug)]
pub struct MemoryNodeStats {
    /// Total access count
    pub access_count: AtomicU64,
    /// Read operation count
    pub read_count: AtomicU64,
    /// Write operation count
    pub write_count: AtomicU64,
    /// Relationship access count
    pub relationship_count: AtomicUsize,
    /// Last access timestamp (as nanos since `UNIX_EPOCH`)
    pub last_access_nanos: AtomicU64,
}

impl Default for MemoryNodeStats {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryNodeStats {
    /// Create new stats with zero counters
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            access_count: AtomicU64::new(0),
            read_count: AtomicU64::new(0),
            write_count: AtomicU64::new(0),
            relationship_count: AtomicUsize::new(0),
            last_access_nanos: AtomicU64::new(0),
        }
    }

    /// Record read access atomically
    #[inline]
    pub fn record_read(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        self.read_count.fetch_add(1, Ordering::Relaxed);
        self.update_last_access();
    }

    /// Record write access atomically
    #[inline]
    pub fn record_write(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        self.write_count.fetch_add(1, Ordering::Relaxed);
        self.update_last_access();
    }

    /// Update last access timestamp atomically
    #[inline]
    fn update_last_access(&self) {
        let now_nanos = unix_timestamp_nanos();
        self.last_access_nanos.store(now_nanos, Ordering::Relaxed);
    }

    /// Get access count
    #[inline]
    pub fn access_count(&self) -> u64 {
        self.access_count.load(Ordering::Relaxed)
    }

    /// Get read count
    #[inline]
    pub fn read_count(&self) -> u64 {
        self.read_count.load(Ordering::Relaxed)
    }

    /// Get write count
    #[inline]
    pub fn write_count(&self) -> u64 {
        self.write_count.load(Ordering::Relaxed)
    }

    /// Get relationship count
    #[inline]
    pub fn relationship_count(&self) -> usize {
        self.relationship_count.load(Ordering::Relaxed)
    }

    /// Get last access time
    #[inline]
    pub fn last_access_time(&self) -> Option<SystemTime> {
        let nanos = self.last_access_nanos.load(Ordering::Relaxed);
        if nanos == 0 {
            None
        } else {
            SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_nanos(nanos))
        }
    }
}
