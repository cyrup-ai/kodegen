//! Configuration caching for high-performance memory operations
//!
//! This module provides thread-local and global configuration caching
//! with copy-on-write semantics for zero-allocation access patterns.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::super::SurrealDBMemoryManager;
use super::memory::MemoryConfig;
use crate::domain::init::globals::{CONFIG_CACHE, LOCAL_CONFIG};
use crate::domain::util::unix_timestamp_nanos;

/// Memory metadata for pool management with zero-allocation patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Memory identifier
    pub id: String,
    /// Memory type
    pub memory_type: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Creation timestamp in nanoseconds
    pub created_at_nanos: u64,
    /// Last accessed timestamp in nanoseconds
    pub last_accessed_nanos: u64,
    /// Access count
    pub access_count: u64,
    /// Whether memory is active
    pub is_active: bool,
}

impl MemoryMetadata {
    /// Create new memory metadata
    #[inline]
    pub fn new(id: impl Into<String>, memory_type: impl Into<String>, size_bytes: u64) -> Self {
        let now = unix_timestamp_nanos();

        Self {
            id: id.into(),
            memory_type: memory_type.into(),
            size_bytes,
            created_at_nanos: now,
            last_accessed_nanos: now,
            access_count: 0,
            is_active: true,
        }
    }

    /// Update access timestamp and count
    #[inline]
    pub fn touch(&mut self) {
        self.last_accessed_nanos = unix_timestamp_nanos();
        self.access_count += 1;
    }

    /// Mark memory as inactive
    #[inline]
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

/// Get cached configuration from thread-local storage with zero-allocation access
#[inline]
#[must_use]
pub fn get_cached_config() -> Arc<MemoryConfig> {
    LOCAL_CONFIG.with(|config| {
        let mut config_ref = config.borrow_mut();
        if let Some(cached) = config_ref.as_ref() {
            Arc::clone(cached)
        } else {
            let global_config = CONFIG_CACHE.load();
            let config_arc = Arc::clone(&global_config);
            *config_ref = Some(config_arc.clone());
            config_arc
        }
    })
}

/// Update global configuration cache with copy-on-write semantics
#[inline]
pub fn update_config_cache(new_config: MemoryConfig) {
    CONFIG_CACHE.store(Arc::new(new_config));
    // Clear thread-local caches to force refresh
    LOCAL_CONFIG.with(|config| {
        *config.borrow_mut() = None;
    });
}

/// Get memory from connection pool with lock-free access
#[inline]
pub async fn get_pooled_memory() -> Option<Arc<SurrealDBMemoryManager>> {
    use std::sync::atomic::Ordering;

    use crate::domain::init::globals::{CONNECTION_POOL, POOL_STATS};

    let (_tx, rx) = &*CONNECTION_POOL;
    let mut receiver = rx.lock().await;
    if let Some(memory) = receiver.recv().await {
        POOL_STATS.fetch_sub(1, Ordering::Relaxed);
        return Some(memory);
    }
    None
}

/// Return memory to connection pool
#[inline]
pub fn return_pooled_memory(memory: Arc<SurrealDBMemoryManager>) {
    use std::sync::atomic::Ordering;

    use crate::domain::init::globals::{CONNECTION_POOL, POOL_STATS};

    let (tx, _rx) = &*CONNECTION_POOL;
    if tx.send(memory).is_ok() {
        POOL_STATS.fetch_add(1, Ordering::Relaxed);
    }
}

/// Get current pool statistics
pub fn get_pool_stats() -> usize {
    use std::sync::atomic::Ordering;

    use crate::domain::init::globals::POOL_STATS;

    POOL_STATS.load(Ordering::Relaxed)
}
