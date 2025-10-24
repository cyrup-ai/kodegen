//! Memory item definitions for working and long-term memory storage

use crate::domain::util::unix_timestamp_nanos;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Working memory item for lock-free queue operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryItem {
    /// Item identifier
    pub id: Uuid,
    /// Memory content reference
    pub content: String,
    /// Activation strength
    pub activation: f32,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Time-to-live for automatic cleanup
    pub ttl: Duration,
    /// Access frequency counter for consolidation logic
    pub access_count: usize,
    /// Mark ephemeral memories that should not consolidate
    pub is_transient: bool,
}

impl WorkingMemoryItem {
    /// Create new working memory item
    #[inline]
    pub fn new(content: impl Into<Arc<str>>, activation: f32, ttl: Duration) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: content.into().to_string(),
            activation: activation.clamp(0.0, 1.0),
            created_at: SystemTime::now(),
            ttl,
            access_count: 0,
            is_transient: false,
        }
    }

    /// Check if item has expired
    #[inline]
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().unwrap_or(Duration::ZERO) > self.ttl
    }
}

/// Cognitive memory entry for long-term storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveMemoryEntry {
    /// Memory content
    pub content: String,
    /// Associative strength
    pub strength: f32,
    /// Access frequency
    pub access_count: u64,
    /// Last access time
    pub last_access: SystemTime,
    /// Decay rate for forgetting
    pub decay_rate: f32,
}

impl CognitiveMemoryEntry {
    /// Create new cognitive memory entry
    #[inline]
    pub fn new(content: impl Into<Arc<str>>, strength: f32) -> Self {
        Self {
            content: content.into().to_string(),
            strength: strength.clamp(0.0, 1.0),
            access_count: 0,
            last_access: SystemTime::now(),
            decay_rate: 0.01, // Default 1% decay per access
        }
    }

    /// Record access and update strength
    #[inline]
    pub fn access(&mut self) {
        self.access_count += 1;
        self.last_access = SystemTime::now();
        // Strengthen memory with each access, but with diminishing returns
        self.strength = (self.strength + 0.1 * (1.0 - self.strength)).min(1.0);
    }

    /// Apply decay based on time since last access
    #[inline]
    pub fn apply_decay(&mut self) {
        let elapsed = self.last_access.elapsed().unwrap_or(Duration::ZERO);
        let decay_factor = (-self.decay_rate * elapsed.as_secs_f32()).exp();
        self.strength *= decay_factor;
    }

    /// Calculate relevance score based on strength and recency
    #[inline]
    #[must_use]
    pub fn relevance_score(&self) -> f32 {
        let recency_factor = if let Ok(elapsed) = self.last_access.elapsed() {
            (-elapsed.as_secs_f32() / 86400.0).exp() // Decay over 24 hours
        } else {
            0.0
        };
        self.strength * recency_factor
    }
}

/// Cognitive performance statistics
#[derive(Debug)]
pub struct CognitiveStats {
    /// Working memory access count
    pub working_memory_accesses: AtomicU64,
    /// Long-term memory access count
    pub long_term_memory_accesses: AtomicU64,
    /// Quantum operations count
    pub quantum_operations: AtomicU64,
    /// Attention updates count
    pub attention_updates: AtomicU64,
    /// Last update timestamp
    pub last_update_nanos: AtomicU64,
}

impl CognitiveStats {
    /// Create new cognitive stats
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            working_memory_accesses: AtomicU64::new(0),
            long_term_memory_accesses: AtomicU64::new(0),
            quantum_operations: AtomicU64::new(0),
            attention_updates: AtomicU64::new(0),
            last_update_nanos: AtomicU64::new(0),
        }
    }

    /// Record working memory access
    #[inline]
    pub fn record_working_memory_access(&self) {
        self.working_memory_accesses.fetch_add(1, Ordering::Relaxed);
        self.update_timestamp();
    }

    /// Record long-term memory access
    #[inline]
    pub fn record_long_term_memory_access(&self) {
        self.long_term_memory_accesses
            .fetch_add(1, Ordering::Relaxed);
        self.update_timestamp();
    }

    /// Record quantum operation
    #[inline]
    pub fn record_quantum_operation(&self) {
        self.quantum_operations.fetch_add(1, Ordering::Relaxed);
        self.update_timestamp();
    }

    /// Record attention update
    #[inline]
    pub fn record_attention_update(&self) {
        self.attention_updates.fetch_add(1, Ordering::Relaxed);
        self.update_timestamp();
    }

    /// Update last access timestamp
    #[inline]
    fn update_timestamp(&self) {
        let now_nanos = unix_timestamp_nanos();
        self.last_update_nanos.store(now_nanos, Ordering::Relaxed);
    }
}

impl Default for CognitiveStats {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
