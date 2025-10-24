//! Caching logic for anchor resolution performance optimization
//!
//! This module provides efficient caching mechanisms for anchor resolution
//! with proper cache invalidation and memory management.

use crate::parser::ast::Node;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Cached resolution result for performance optimization
#[derive(Debug, Clone)]
pub struct CachedResolution<'input> {
    pub resolved_node: Node<'input>,
    pub cached_at: Instant,
    pub access_count: usize,
}

impl<'input> CachedResolution<'input> {
    /// Create new cached resolution entry
    #[must_use] 
    pub fn new(resolved_node: Node<'input>) -> Self {
        Self {
            resolved_node,
            cached_at: Instant::now(),
            access_count: 1,
        }
    }

    /// Check if cache entry is stale (older than specified duration)
    #[must_use] 
    pub fn is_stale(&self, max_age: Duration) -> bool {
        self.cached_at.elapsed() > max_age
    }

    /// Get cache entry age
    #[inline]
    #[must_use] 
    pub fn age(&self) -> Duration {
        self.cached_at.elapsed()
    }

    /// Update access count and optionally refresh timestamp
    pub fn access(&mut self, refresh_timestamp: bool) {
        self.access_count += 1;
        if refresh_timestamp {
            self.cached_at = Instant::now();
        }
    }

    /// Get access frequency (accesses per second since creation)
    #[must_use] 
    pub fn access_frequency(&self) -> f64 {
        let age_seconds = self.age().as_secs_f64();
        if age_seconds > 0.0 {
            self.access_count as f64 / age_seconds
        } else {
            self.access_count as f64
        }
    }
}

/// Cache performance statistics
#[derive(Debug, Clone, Copy)]
pub struct CacheStatistics {
    pub cache_size: usize,
    pub total_accesses: usize,
    pub avg_access_count: f64,
    pub hit_rate: f64,
    pub memory_usage_estimate: usize,
}

impl CacheStatistics {
    /// Create cache statistics from resolution cache
    #[must_use] 
    pub fn from_cache(
        cache: &HashMap<String, CachedResolution<'_>>,
        total_resolution_attempts: usize,
    ) -> Self {
        let total_accesses: usize = cache.values().map(|cached| cached.access_count).sum();

        let avg_access_count = if !cache.is_empty() {
            total_accesses as f64 / cache.len() as f64
        } else {
            0.0
        };

        let hit_rate = if total_resolution_attempts > 0 {
            total_accesses as f64 / total_resolution_attempts as f64
        } else {
            0.0
        };

        // Rough memory usage estimate (string keys + cached nodes)
        let memory_usage_estimate = cache
            .keys()
            .map(|key| key.len() + std::mem::size_of::<CachedResolution<'_>>())
            .sum::<usize>();

        Self {
            cache_size: cache.len(),
            total_accesses,
            avg_access_count,
            hit_rate,
            memory_usage_estimate,
        }
    }
}

/// Cache management and optimization utilities
pub struct CacheManager<'input> {
    cache: HashMap<String, CachedResolution<'input>>,
    max_size: usize,
    max_age: Duration,
    total_resolution_attempts: usize,
}

impl<'input> CacheManager<'input> {
    /// Create new cache manager with specified limits
    #[must_use] 
    pub fn new(max_size: usize, max_age: Duration) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size.min(64)),
            max_size,
            max_age,
            total_resolution_attempts: 0,
        }
    }

    /// Get cached resolution if available and not stale
    pub fn get(&mut self, alias_name: &str) -> Option<&Node<'input>> {
        self.total_resolution_attempts += 1;

        // First check if entry exists and is stale, removing if needed
        let should_remove = if let Some(cached) = self.cache.get(alias_name) {
            cached.is_stale(self.max_age)
        } else {
            false
        };

        if should_remove {
            self.cache.remove(alias_name);
            return None;
        }

        // Now safely get the entry and update access
        if let Some(cached) = self.cache.get_mut(alias_name) {
            cached.access(false);
            Some(&cached.resolved_node)
        } else {
            None
        }
    }

    /// Store resolution in cache
    pub fn store(&mut self, alias_name: String, resolved_node: Node<'input>) {
        // Check if we need to make room
        if self.cache.len() >= self.max_size {
            self.evict_lru();
        }

        let cached = CachedResolution::new(resolved_node);
        self.cache.insert(alias_name, cached);
    }

    /// Evict least recently used entries to make room
    fn evict_lru(&mut self) {
        if self.cache.is_empty() {
            return;
        }

        let target_size = (self.max_size as f64 * 0.8) as usize;
        let to_remove = self.cache.len().saturating_sub(target_size);

        if to_remove == 0 {
            return;
        }

        // Collect entries sorted by access frequency (ascending)
        let mut entries: Vec<_> = self
            .cache
            .iter()
            .map(|(k, v)| (k.clone(), v.access_frequency(), v.cached_at))
            .collect();

        entries.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.2.cmp(&b.2))
        });

        // Remove least frequently accessed entries
        for (alias_name, _, _) in entries.into_iter().take(to_remove) {
            self.cache.remove(&alias_name);
        }
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Remove stale entries based on max_age
    pub fn cleanup_stale(&mut self) {
        let stale_keys: Vec<_> = self
            .cache
            .iter()
            .filter(|(_, cached)| cached.is_stale(self.max_age))
            .map(|(key, _)| key.clone())
            .collect();

        for key in stale_keys {
            self.cache.remove(&key);
        }
    }

    /// Get cache statistics
    #[must_use] 
    pub fn statistics(&self) -> CacheStatistics {
        CacheStatistics::from_cache(&self.cache, self.total_resolution_attempts)
    }

    /// Get cache size
    #[inline]
    #[must_use] 
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    #[inline]
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get cache utilization as percentage
    #[must_use] 
    pub fn utilization(&self) -> f64 {
        if self.max_size > 0 {
            (self.cache.len() as f64 / self.max_size as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Resize cache capacity
    pub fn resize(&mut self, new_max_size: usize) {
        self.max_size = new_max_size;

        // If new size is smaller, evict excess entries
        if self.cache.len() > new_max_size {
            let to_remove = self.cache.len() - new_max_size;
            for _ in 0..to_remove {
                self.evict_lru();
            }
        }
    }
}

impl<'input> Default for CacheManager<'input> {
    fn default() -> Self {
        Self::new(128, Duration::from_secs(300)) // 5 minutes default
    }
}

/// Cache configuration options
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_size: usize,
    pub max_age: Duration,
    pub eviction_threshold: f64,
    pub cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 128,
            max_age: Duration::from_secs(300),
            eviction_threshold: 0.8,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

impl CacheConfig {
    /// Create cache configuration optimized for performance
    #[must_use] 
    pub const fn performance_optimized() -> Self {
        Self {
            max_size: 512,
            max_age: Duration::from_secs(600),
            eviction_threshold: 0.9,
            cleanup_interval: Duration::from_secs(120),
        }
    }

    /// Create cache configuration optimized for memory usage
    #[must_use] 
    pub const fn memory_optimized() -> Self {
        Self {
            max_size: 32,
            max_age: Duration::from_secs(120),
            eviction_threshold: 0.7,
            cleanup_interval: Duration::from_secs(30),
        }
    }
}
