//! LRU cache implementation with TTL support

use crate::github::search_repositories::types::{RepoCacheEntry, RepositoryResult};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::time::Duration;

/// LRU cache with TTL support
pub(crate) struct SearchCache {
    pub(crate) lru: LruCache<String, RepoCacheEntry>,
    pub(crate) ttl: Duration,
}

impl SearchCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        // Ensure capacity is at least 1 to create valid NonZeroUsize
        let non_zero_capacity = NonZeroUsize::new(capacity.max(1)).unwrap_or(NonZeroUsize::MIN);
        Self {
            lru: LruCache::new(non_zero_capacity),
            ttl,
        }
    }

    pub fn get_if_valid(&mut self, key: &str, current_sha: &str) -> Option<RepositoryResult> {
        if let Some(entry) = self.lru.get(key)
            && !entry.is_expired(self.ttl) && entry.commit_hash == current_sha
        {
            return Some(entry.result.clone());
        }
        None
    }

    pub fn put(&mut self, key: String, result: RepositoryResult, commit_hash: String) {
        let entry = RepoCacheEntry {
            result,
            commit_hash,
            cached_at: chrono::Utc::now(),
        };
        self.lru.put(key, entry);
    }

    #[allow(dead_code)]
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.lru.len(), self.lru.cap().get())
    }

    pub fn cleanup_expired(&mut self) {
        let expired_keys: Vec<String> = self
            .lru
            .iter()
            .filter(|(_, entry)| entry.is_expired(self.ttl))
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.lru.pop(&key);
        }
    }
}
