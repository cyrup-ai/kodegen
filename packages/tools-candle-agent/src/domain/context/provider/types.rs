//! Types module for Candle context provider system
//!
//! This module contains all marker types, error enums, event types, data structures,
//! and traits for the zero-allocation context provider system.

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio_stream::Stream;

/// Marker types for `CandleContext`
/// Marker type for file-based Candle context operations. Used in typestate pattern to ensure compile-time safety for file context providers.
#[derive(Debug, Clone)]
pub struct CandleFile;
/// Marker type for multi-file Candle context operations. Enables batch processing of multiple files with zero-allocation streaming patterns.
#[derive(Debug, Clone)]
pub struct CandleFiles;
/// Marker type for directory-based Candle context indexing. Provides recursive directory traversal with configurable depth limits and filtering.
#[derive(Debug, Clone)]
pub struct CandleDirectory;
/// Marker type for GitHub repository Candle context integration. Enables GitHub API integration with rate limiting and authentication.
#[derive(Debug, Clone)]
pub struct CandleGithub;

/// Comprehensive error types for Candle context operations with zero allocations
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum CandleContextError {
    #[error("Context not found: {0}")]
    /// Context resource could not be located. Occurs when file paths, directories, or repository references are invalid or inaccessible.
    ContextNotFound(String),
    #[error("Invalid path: {0}")]
    /// Path validation failed due to invalid characters, encoding, or filesystem constraints. Includes both local and remote path validation.
    InvalidPath(String),
    #[error("IO error: {0}")]
    /// Filesystem I/O operation failed. Wraps underlying `std::io::Error` with context-specific information for debugging.
    IoError(String),
    #[error("Pattern error: {0}")]
    /// Regular expression or glob pattern compilation failed. Occurs during context filtering and search operations.
    PatternError(String),
    #[error("Memory integration error: {0}")]
    /// Memory integration subsystem error. Indicates failure in vector embeddings, storage, or retrieval operations.
    MemoryError(String),
    #[error("Validation error: {0}")]
    /// Input validation failed during context operations. Includes size limits, format validation, and content restrictions.
    ValidationError(String),
    #[error("Performance threshold exceeded: {0}")]
    /// Operation exceeded configured performance thresholds. Used for timeout, memory usage, or processing limits.
    PerformanceThresholdExceeded(String),
    #[error("Provider unavailable: {0}")]
    /// Context provider service is temporarily or permanently unavailable. Includes network, API, and resource availability issues.
    ProviderUnavailable(String),
}

/// Provider-specific error types for Candle
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum CandleProviderError {
    #[error("File provider error: {0}")]
    /// File system provider specific error. Handles file reading, parsing, and metadata extraction failures.
    FileProvider(String),
    #[error("Directory provider error: {0}")]
    /// Directory provider specific error. Manages recursive traversal, filtering, and indexing failures.
    DirectoryProvider(String),
    #[error("GitHub provider error: {0}")]
    /// GitHub API provider specific error. Handles authentication, rate limiting, and repository access issues.
    GithubProvider(String),
    #[error("Embedding provider error: {0}")]
    /// Vector embedding provider error. Manages embedding generation, storage, and retrieval failures.
    EmbeddingProvider(String),
}

/// Validation error types with semantic meaning for Candle
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum CandleValidationError {
    #[error("Invalid input: {0}")]
    /// Generic input validation failure. Used when input doesn't meet format, type, or semantic requirements.
    InvalidInput(String),
    #[error("Path validation failed: {0}")]
    /// Path-specific validation error. Handles invalid characters, length limits, and security restrictions.
    PathValidation(String),
    #[error("Pattern validation failed: {0}")]
    /// Pattern syntax validation failed. Occurs during regex or glob pattern parsing with detailed error information.
    PatternValidation(String),
    #[error("Size limit exceeded: {0}")]
    /// Content size exceeds configured limits. Includes file size, directory depth, and processing constraints.
    SizeLimitExceeded(String),
}

/// Candle context events for real-time streaming monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CandleContextEvent {
    /// Provider lifecycle events
    ProviderStarted {
        /// Type of provider that was started (File, Directory, Github, etc.)
        provider_type: String,
        /// Unique identifier for this provider instance
        provider_id: String,
        /// When the provider was started
        timestamp: SystemTime,
    },
    /// Provider shutdown event
    ProviderStopped {
        /// Type of provider that was stopped (File, Directory, Github, etc.)
        provider_type: String,
        /// Unique identifier for this provider instance
        provider_id: String,
        /// When the provider was stopped
        timestamp: SystemTime,
    },

    /// Operation events
    ContextLoadStarted {
        /// Type of context being loaded (file, directory, github, etc.)
        context_type: String,
        /// Source path or identifier for the context
        source: String,
        /// When the context loading operation started
        timestamp: SystemTime,
    },
    ContextLoadCompleted {
        /// Type of context that was loaded (file, directory, github, etc.)
        context_type: String,
        /// Source path or identifier for the context
        source: String,
        /// Number of documents successfully loaded
        documents_loaded: usize,
        /// Duration of the loading operation
        duration: Duration,
        /// When the context loading operation completed
        timestamp: SystemTime,
    },
    ContextLoadFailed {
        /// Type of context that failed to load (file, directory, github, etc.)
        context_type: String,
        /// Source path or identifier for the context
        source: String,
        /// Error message describing the failure
        error: String,
        /// When the context loading operation failed
        timestamp: SystemTime,
    },

    /// Memory integration events
    MemoryCreated {
        /// Unique identifier for the created memory entry
        memory_id: String,
        /// Hash of the content for deduplication and integrity verification
        content_hash: String,
        /// When the memory entry was created
        timestamp: SystemTime,
    },
    MemorySearchCompleted {
        /// Search query that was executed
        query: String,
        /// Number of results returned by the search
        results_count: usize,
        /// Duration of the search operation
        duration: Duration,
        /// When the search operation completed
        timestamp: SystemTime,
    },

    /// Performance events
    PerformanceThresholdBreached {
        /// Name of the performance metric that exceeded threshold
        metric: String,
        /// Configured threshold value that was exceeded
        threshold: f64,
        /// Actual measured value that exceeded the threshold
        actual: f64,
        /// When the threshold breach was detected
        timestamp: SystemTime,
    },

    /// Validation events
    ValidationFailed {
        /// Type of validation that failed (path, pattern, size, etc.)
        validation_type: String,
        /// Error message describing the validation failure
        error: String,
        /// When the validation failure occurred
        timestamp: SystemTime,
    },
}

impl MessageChunk for CandleContextEvent {
    fn bad_chunk(error: String) -> Self {
        CandleContextEvent::ValidationFailed {
            validation_type: "system_error".to_string(),
            error,
            timestamp: SystemTime::now(),
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            CandleContextEvent::ContextLoadFailed { error, .. }
            | CandleContextEvent::ValidationFailed { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl Default for CandleContextEvent {
    fn default() -> Self {
        CandleContextEvent::ValidationFailed {
            validation_type: "default".to_string(),
            error: "default event".to_string(),
            timestamp: SystemTime::now(),
        }
    }
}

/// Candle memory node representation with owned strings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleMemoryNode {
    /// Unique identifier for this memory node
    pub id: String,
    /// Content stored in this memory node
    pub content: String,
    /// Metadata key-value pairs associated with this node
    pub metadata: HashMap<String, String>,
    /// Optional vector embedding for similarity search
    pub embedding: Option<Vec<f32>>,
    /// When this memory node was created or last updated
    pub timestamp: SystemTime,
}

/// Immutable file context with owned strings and atomic tracking for Candle
#[derive(Debug, Clone)]
pub struct CandleImmutableFileContext {
    /// File path as owned string
    pub path: String,
    /// Content hash for deduplication
    pub content_hash: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// Last modified timestamp
    pub modified: SystemTime,
    /// Memory integration layer
    pub memory_integration: Option<CandleMemoryIntegration>,
}

/// Immutable files context with owned strings for Candle
#[derive(Debug, Clone)]
pub struct CandleImmutableFilesContext {
    /// File paths as owned strings
    pub paths: Vec<String>,
    /// Glob pattern as owned string
    pub pattern: String,
    /// Total files count
    pub total_files: usize,
    /// Memory integration layer
    pub memory_integration: Option<CandleMemoryIntegration>,
}

/// Immutable directory context with owned strings for Candle
#[derive(Debug, Clone)]
pub struct CandleImmutableDirectoryContext {
    /// Directory path as owned string
    pub path: String,
    /// Recursive traversal flag
    pub recursive: bool,
    /// File extensions filter
    pub extensions: Vec<String>,
    /// Maximum depth for traversal
    pub max_depth: Option<usize>,
    /// Memory integration layer
    pub memory_integration: Option<CandleMemoryIntegration>,
}

/// Immutable GitHub context with owned strings for Candle
#[derive(Debug, Clone)]
pub struct CandleImmutableGithubContext {
    /// Repository URL as owned string
    pub repository_url: String,
    /// Branch name as owned string
    pub branch: String,
    /// File pattern as owned string
    pub pattern: String,
    /// Authentication token (if needed)
    pub auth_token: Option<String>,
    /// Memory integration layer
    pub memory_integration: Option<CandleMemoryIntegration>,
}

/// Candle memory integration layer with atomic operations
#[derive(Debug)]
pub struct CandleMemoryIntegration {
    /// Memory manager identifier
    pub manager_id: String,
    /// Embedding model identifier
    pub embedding_model: String,
    /// Vector dimension
    pub vector_dimension: usize,
    /// Performance tracking
    pub memory_requests: AtomicU64,
    pub successful_operations: AtomicU64,
    pub failed_operations: AtomicU64,
    pub total_processing_time_nanos: AtomicU64,
}

impl Clone for CandleMemoryIntegration {
    fn clone(&self) -> Self {
        Self {
            manager_id: self.manager_id.clone(),
            embedding_model: self.embedding_model.clone(),
            vector_dimension: self.vector_dimension,
            memory_requests: AtomicU64::new(
                self.memory_requests
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            successful_operations: AtomicU64::new(
                self.successful_operations
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            failed_operations: AtomicU64::new(
                self.failed_operations
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            total_processing_time_nanos: AtomicU64::new(
                self.total_processing_time_nanos
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

impl CandleMemoryIntegration {
    /// Create new memory integration with owned strings
    #[inline]
    #[must_use]
    pub fn new(manager_id: String, embedding_model: String, vector_dimension: usize) -> Self {
        Self {
            manager_id,
            embedding_model,
            vector_dimension,
            memory_requests: AtomicU64::new(0),
            successful_operations: AtomicU64::new(0),
            failed_operations: AtomicU64::new(0),
            total_processing_time_nanos: AtomicU64::new(0),
        }
    }

    /// Record successful operation
    #[inline]
    pub fn record_success(&self, duration_nanos: u64) {
        self.successful_operations.fetch_add(1, Ordering::Relaxed);
        self.total_processing_time_nanos
            .fetch_add(duration_nanos, Ordering::Relaxed);
    }

    /// Record failed operation
    #[inline]
    pub fn record_failure(&self) {
        self.failed_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get success rate (0.0 to 1.0)
    #[inline]
    #[allow(clippy::cast_precision_loss)] // Acceptable for rate calculations
    pub fn success_rate(&self) -> f64 {
        let successful = self.successful_operations.load(Ordering::Relaxed);
        let failed = self.failed_operations.load(Ordering::Relaxed);
        let total = successful + failed;
        if total == 0 {
            1.0
        } else {
            successful as f64 / total as f64
        }
    }

    /// Get average processing time in nanoseconds
    #[inline]
    pub fn average_processing_time_nanos(&self) -> u64 {
        let total_time = self.total_processing_time_nanos.load(Ordering::Relaxed);
        let successful = self.successful_operations.load(Ordering::Relaxed);
        if successful == 0 {
            0
        } else {
            total_time / successful
        }
    }
}

/// Immutable embedding model with streaming operations for Candle
pub trait CandleImmutableEmbeddingModel: Send + Sync + 'static {
    /// Generate embeddings for text with streaming results - returns unwrapped values
    fn embed(
        &self,
        text: &str,
        context: Option<String>,
    ) -> Pin<Box<dyn Stream<Item = Vec<f32>> + Send>>;

    /// Get model information
    fn model_info(&self) -> CandleEmbeddingModelInfo;

    /// Validate input text
    ///
    /// # Errors
    ///
    /// Returns `CandleValidationError` if input text fails validation
    fn validate_input(&self, text: &str) -> Result<(), CandleValidationError>;
}

/// Embedding model information with owned strings for Candle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleEmbeddingModelInfo {
    pub name: String,
    pub version: String,
    pub vector_dimension: usize,
    pub max_input_length: usize,
    pub supported_languages: Vec<String>,
}

/// Immutable memory manager with streaming operations for Candle
pub trait CandleImmutableMemoryManager: Send + Sync + 'static {
    /// Create memory with streaming confirmation - returns unwrapped values
    fn create_memory(&self, node: CandleMemoryNode) -> Pin<Box<dyn Stream<Item = ()> + Send>>;

    /// Search by vector with streaming results - returns unwrapped values
    fn search_by_vector(
        &self,
        vector: Vec<f32>,
        limit: usize,
    ) -> Pin<Box<dyn Stream<Item = CandleMemoryNode> + Send>>;

    /// Search by text with streaming results - returns unwrapped values
    fn search_by_text(
        &self,
        query: &str,
        limit: usize,
    ) -> Pin<Box<dyn Stream<Item = CandleMemoryNode> + Send>>;

    /// Update memory with streaming confirmation - returns unwrapped values
    fn update_memory(
        &self,
        memory_id: &str,
        node: CandleMemoryNode,
    ) -> Pin<Box<dyn Stream<Item = ()> + Send>>;

    /// Delete memory with streaming confirmation - returns unwrapped values
    fn delete_memory(&self, memory_id: &str) -> Pin<Box<dyn Stream<Item = ()> + Send>>;

    /// Get memory manager information
    fn manager_info(&self) -> CandleMemoryManagerInfo;
}

/// Memory manager information with owned strings for Candle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleMemoryManagerInfo {
    pub name: String,
    pub version: String,
    pub storage_type: String,
    pub max_memory_nodes: Option<usize>,
    pub supported_operations: Vec<String>,
}
