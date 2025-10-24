use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::types::{AllocationStrategy, VectorStoreType};

/// Connection configuration for external vector stores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConnectionConfig {
    /// Connection URL
    pub url: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Connection timeout
    pub timeout: Duration,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Enable TLS/SSL
    pub enable_tls: bool,
    /// Custom headers for requests
    pub headers: Option<Arc<serde_json::Value>>,
}

impl VectorConnectionConfig {
    /// Create new connection configuration
    #[inline]
    pub fn new(url: impl Into<Arc<str>>) -> Self {
        Self {
            url: url.into().to_string(),
            api_key: None,
            timeout: Duration::from_secs(30),
            max_connections: 10,
            idle_timeout: Duration::from_secs(300),
            enable_tls: true,
            headers: None,
        }
    }

    /// Set API key
    #[must_use]
    #[inline]
    pub fn with_api_key(mut self, api_key: impl Into<Arc<str>>) -> Self {
        self.api_key = Some(api_key.into().to_string());
        self
    }

    /// Set timeout
    #[must_use]
    #[inline]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Performance configuration for vector operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of threads for parallel operations
    pub num_threads: usize,
    /// Batch size for bulk operations
    pub batch_size: usize,
    /// Enable prefetching for sequential access
    pub enable_prefetch: bool,
    /// Cache size for frequently accessed vectors
    pub cache_size: usize,
    /// Enable compression for storage
    pub enable_compression: bool,
    /// Quantization precision for reduced memory usage
    pub quantization_bits: Option<u8>,
}

impl PerformanceConfig {
    /// Create optimized performance configuration
    #[inline]
    #[must_use]
    pub fn optimized(store_type: VectorStoreType) -> Self {
        Self {
            num_threads: num_cpus::get(),
            batch_size: store_type.optimal_batch_size(),
            enable_prefetch: true,
            cache_size: 10000,
            enable_compression: matches!(
                store_type,
                VectorStoreType::FAISS | VectorStoreType::Milvus
            ),
            quantization_bits: None,
        }
    }

    /// Create minimal performance configuration for testing
    #[inline]
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            num_threads: 1,
            batch_size: 10,
            enable_prefetch: false,
            cache_size: 100,
            enable_compression: false,
            quantization_bits: None,
        }
    }
}

impl Default for PerformanceConfig {
    #[inline]
    fn default() -> Self {
        Self::optimized(VectorStoreType::Memory)
    }
}

/// Memory configuration for vector operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Memory allocation strategy
    pub allocation_strategy: AllocationStrategy,
    /// Enable memory mapping for large indices
    pub enable_mmap: bool,
    /// Memory usage tracking
    pub track_usage: bool,
    /// Atomic memory usage counter
    #[serde(skip)]
    pub current_usage: Arc<AtomicUsize>,
}

impl MemoryConfig {
    /// Create new memory configuration
    #[inline]
    #[must_use]
    pub fn new(max_memory_bytes: usize) -> Self {
        Self {
            max_memory_bytes,
            allocation_strategy: AllocationStrategy::System,
            enable_mmap: max_memory_bytes > 1024 * 1024 * 1024, // Enable for >1GB
            track_usage: true,
            current_usage: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Record memory allocation
    #[inline]
    #[must_use]
    pub fn allocate(&self, bytes: usize) -> bool {
        if self.track_usage {
            let current = self.current_usage.fetch_add(bytes, Ordering::Relaxed);
            current + bytes <= self.max_memory_bytes
        } else {
            true
        }
    }

    /// Record memory deallocation
    #[inline]
    pub fn deallocate(&self, bytes: usize) {
        if self.track_usage {
            self.current_usage.fetch_sub(bytes, Ordering::Relaxed);
        }
    }

    /// Get current memory usage
    #[inline]
    #[must_use]
    pub fn current_usage(&self) -> usize {
        if self.track_usage {
            self.current_usage.load(Ordering::Relaxed)
        } else {
            0
        }
    }

    /// Get available memory
    #[inline]
    #[must_use]
    pub fn available_memory(&self) -> usize {
        self.max_memory_bytes.saturating_sub(self.current_usage())
    }

    /// Check if allocation would exceed limit
    #[inline]
    #[must_use]
    pub fn would_exceed_limit(&self, bytes: usize) -> bool {
        if self.track_usage {
            self.current_usage() + bytes > self.max_memory_bytes
        } else {
            false
        }
    }
}

impl Default for MemoryConfig {
    #[inline]
    fn default() -> Self {
        // Default to 1GB memory limit
        Self::new(1024 * 1024 * 1024)
    }
}
