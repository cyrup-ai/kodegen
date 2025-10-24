//! Shared configuration types for memory system
//!
//! This module provides canonical definitions of configuration types that are
//! used across multiple memory subsystems to eliminate duplication.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Canonical retry configuration with exponential backoff and jitter
///
/// This is the single source of truth for retry configuration across all memory subsystems.
/// Combines fields from both database and LLM retry configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Enable retries (from LLM config)
    pub enabled: bool,
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Enable jitter to prevent thundering herd
    pub enable_jitter: bool,
    /// Retryable HTTP status codes (from LLM config)
    pub retryable_status_codes: Vec<u16>,
}

impl Default for RetryConfig {
    /// Create default retry configuration with exponential backoff
    #[inline]
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            enable_jitter: true,
            retryable_status_codes: vec![429, 500, 502, 503, 504],
        }
    }
}

impl RetryConfig {
    /// Create optimized retry configuration for high-performance scenarios
    #[inline]
    #[must_use]
    pub fn optimized() -> Self {
        Self {
            enabled: true,
            max_retries: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 1.5,
            enable_jitter: true,
            retryable_status_codes: vec![429, 500, 502, 503, 504, 408, 409, 423, 424],
        }
    }

    /// Create aggressive retry configuration for critical operations
    #[inline]
    #[must_use]
    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            max_retries: 10,
            initial_delay: Duration::from_millis(25),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            enable_jitter: true,
            retryable_status_codes: vec![
                429, 500, 502, 503, 504, 408, 409, 423, 424, 425, 510, 511,
            ],
        }
    }

    /// Create minimal retry configuration for testing
    #[inline]
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            max_retries: 1,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_multiplier: 1.0,
            enable_jitter: false,
            retryable_status_codes: vec![429, 500, 502, 503],
        }
    }

    /// Create disabled retry configuration
    #[inline]
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            max_retries: 0,
            initial_delay: Duration::from_millis(0),
            max_delay: Duration::from_millis(0),
            backoff_multiplier: 1.0,
            enable_jitter: false,
            retryable_status_codes: vec![],
        }
    }

    /// Check if a status code is retryable
    #[inline]
    #[must_use]
    pub fn is_retryable_status(&self, status_code: u16) -> bool {
        self.enabled && self.retryable_status_codes.contains(&status_code)
    }

    /// Calculate delay for a given retry attempt with optional jitter
    #[inline]
    #[must_use]
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        if !self.enabled || attempt == 0 {
            return Duration::from_millis(0);
        }

        #[allow(clippy::cast_precision_loss)] // Acceptable for delay calculation
        let base_delay = self.initial_delay.as_millis() as f64;
        #[allow(clippy::cast_possible_wrap)]
        let multiplier = self
            .backoff_multiplier
            .powi(i32::try_from(attempt.saturating_sub(1)).unwrap_or(i32::MAX));
        let mut delay_ms = base_delay * multiplier;

        // Apply maximum delay limit
        #[allow(clippy::cast_precision_loss)] // Acceptable for delay limit calculation
        let max_delay_ms = self.max_delay.as_millis() as f64;
        if delay_ms > max_delay_ms {
            delay_ms = max_delay_ms;
        }

        // Apply jitter if enabled (±25% variation)
        if self.enable_jitter {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            attempt.hash(&mut hasher);
            let hash = hasher.finish();

            // Convert hash to a value between 0.75 and 1.25
            #[allow(clippy::cast_precision_loss)] // Acceptable for jitter calculation
            let jitter = 0.75 + (hash % 500) as f64 / 1000.0;
            delay_ms *= jitter;
        }

        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        Duration::from_millis(delay_ms.round() as u64)
    }

    /// Validate the retry configuration
    ///
    /// # Errors
    ///
    /// Returns error string if:
    /// - `max_retries` exceeds 100
    /// - `initial_delay` is greater than `max_delay`
    /// - `backoff_multiplier` is non-positive or exceeds 10.0
    pub fn validate(&self) -> Result<(), String> {
        if self.max_retries > 100 {
            return Err("Maximum retries cannot exceed 100".to_string());
        }

        if self.initial_delay > self.max_delay {
            return Err("Initial delay cannot be greater than maximum delay".to_string());
        }

        if self.backoff_multiplier <= 0.0 {
            return Err("Backoff multiplier must be positive".to_string());
        }

        if self.backoff_multiplier > 10.0 {
            return Err("Backoff multiplier cannot exceed 10.0".to_string());
        }

        Ok(())
    }
}

/// Canonical embedding configuration
///
/// This is the single source of truth for embedding configuration across memory subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Model name or identifier
    pub registry_key: String,
    /// Expected embedding dimension
    pub dimension: usize,
    /// Normalization strategy for embeddings
    pub normalization: NormalizationStrategy,
    /// Enable caching of embeddings
    pub enable_caching: bool,
    /// Cache size for embeddings
    pub cache_size: usize,
    /// Cache time-to-live
    pub cache_ttl: Duration,
    /// Enable compression of cached embeddings
    pub enable_compression: bool,
}

/// Normalization strategies for embeddings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum NormalizationStrategy {
    /// No normalization
    None = 0,
    /// L2 normalization (unit vectors)
    L2 = 1,
    /// L1 normalization
    L1 = 2,
    /// Min-max normalization
    MinMax = 3,
}

impl Default for EmbeddingConfig {
    /// Create default embedding configuration
    #[inline]
    fn default() -> Self {
        Self {
            registry_key: "text-embedding-3-small".to_string(),
            dimension: 1536,
            normalization: NormalizationStrategy::L2,
            enable_caching: true,
            cache_size: 10_000,
            cache_ttl: Duration::from_secs(3600),
            enable_compression: false,
        }
    }
}

impl EmbeddingConfig {
    /// Create high-performance embedding configuration
    #[inline]
    #[must_use]
    pub fn high_performance() -> Self {
        Self {
            registry_key: "text-embedding-3-large".to_string(),
            dimension: 3072,
            normalization: NormalizationStrategy::L2,
            enable_caching: true,
            cache_size: 50_000,
            cache_ttl: Duration::from_secs(7200),
            enable_compression: true,
        }
    }

    /// Create compact embedding configuration for memory-constrained environments
    #[inline]
    #[must_use]
    pub fn compact() -> Self {
        Self {
            registry_key: "all-MiniLM-L6-v2".to_string(),
            dimension: 384,
            normalization: NormalizationStrategy::L2,
            enable_caching: true,
            cache_size: 1_000,
            cache_ttl: Duration::from_secs(1800),
            enable_compression: true,
        }
    }

    /// Create Stella 1024-dim embedding configuration
    #[must_use]
    pub fn stella_1024() -> Self {
        Self {
            registry_key: "dunzhang/stella_en_1.5B_v5".to_string(),
            dimension: 1024,
            normalization: NormalizationStrategy::L2,
            enable_caching: true,
            cache_size: 2_000,
            cache_ttl: Duration::from_secs(3600),
            enable_compression: true,
        }
    }

    /// Create GTE-Qwen2 1536-dim embedding configuration
    #[must_use]
    pub fn gte_qwen() -> Self {
        Self {
            registry_key: "Alibaba-NLP/gte-Qwen2-1.5B-instruct".to_string(),
            dimension: 1536,
            normalization: NormalizationStrategy::L2,
            enable_caching: true,
            cache_size: 1_000,
            cache_ttl: Duration::from_secs(3600),
            enable_compression: true,
        }
    }

    /// Create `NVEmbed` v2 4096-dim embedding configuration
    #[must_use]
    pub fn nvembed_v2() -> Self {
        Self {
            registry_key: "nvidia/NV-Embed-v2".to_string(),
            dimension: 4096,
            normalization: NormalizationStrategy::L2,
            enable_caching: true,
            cache_size: 500,
            cache_ttl: Duration::from_secs(3600),
            enable_compression: true,
        }
    }

    /// Create Jina-BERT 768-dim embedding configuration
    #[must_use]
    pub fn jina_bert() -> Self {
        Self {
            registry_key: "jinaai/jina-embeddings-v2-base-en".to_string(),
            dimension: 768,
            normalization: NormalizationStrategy::L2,
            enable_caching: true,
            cache_size: 3_000,
            cache_ttl: Duration::from_secs(3600),
            enable_compression: true,
        }
    }

    /// Validate the embedding configuration
    ///
    /// # Errors
    ///
    /// Returns error string if:
    /// - `dimension` is 0 or exceeds 10,000
    /// - `cache_size` is 0 when caching is enabled
    /// - `registry_key` is empty
    pub fn validate(&self) -> Result<(), String> {
        if self.dimension == 0 {
            return Err("Embedding dimension must be greater than 0".to_string());
        }

        if self.dimension > 10_000 {
            return Err("Embedding dimension cannot exceed 10,000".to_string());
        }

        if self.cache_size == 0 && self.enable_caching {
            return Err("Cache size must be greater than 0 when caching is enabled".to_string());
        }

        if self.registry_key.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }

        Ok(())
    }
}
