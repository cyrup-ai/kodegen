use std::time::SystemTime;
use cyrup_sugars::ZeroOneOrMany;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::pin::Pin;
use tokio_stream::Stream;

#[derive(Debug)]
pub enum VectorStoreError {
    NotFound,
    ConnectionError(String),
    InvalidQuery(String)}

impl std::fmt::Display for VectorStoreError {
    #[cold]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VectorStoreError::NotFound => write!(f, "Vector store item not found"),
            VectorStoreError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            VectorStoreError::InvalidQuery(msg) => write!(f, "Invalid query: {}", msg)}
    }
}

impl std::error::Error for VectorStoreError {}

pub type Error = VectorStoreError;

#[derive(Debug)]
pub enum MemoryError {
    NotFound,
    StorageError(String),
    ValidationError(String),
    NetworkError(String)}

impl std::fmt::Display for MemoryError {
    #[cold]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::NotFound => write!(f, "Memory not found"),
            MemoryError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            MemoryError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            MemoryError::NetworkError(msg) => write!(f, "Network error: {}", msg)}
    }
}

impl std::error::Error for MemoryError {}

impl From<MemoryError> for VectorStoreError {
    #[cold]
    fn from(error: MemoryError) -> Self {
        match error {
            MemoryError::NotFound => VectorStoreError::NotFound,
            MemoryError::StorageError(msg) => VectorStoreError::ConnectionError(msg),
            MemoryError::ValidationError(msg) => VectorStoreError::InvalidQuery(msg),
            MemoryError::NetworkError(msg) => VectorStoreError::ConnectionError(msg)}
    }
}

#[derive(Debug, Clone)]
pub enum MemoryType {
    ShortTerm,
    LongTerm,
    Semantic,
    Episodic}

#[derive(Debug, Clone, Copy)]
pub enum ImportanceContext {
    UserInput,
    SystemResponse,
    SuccessfulExecution,
    ErrorCondition,
    BackgroundProcess,
    CriticalOperation}

impl MemoryType {
    /// Calculate base importance for memory type with zero allocation
    #[inline]
    pub const fn base_importance(&self) -> f32 {
        match self {
            MemoryType::ShortTerm => 0.3,    // Temporary, less important
            MemoryType::LongTerm => 0.8,     // Persistent, more important
            MemoryType::Semantic => 0.9,     // Knowledge, very important
            MemoryType::Episodic => 0.6,     // Experiences, moderately important
        }
    }
}

impl ImportanceContext {
    /// Calculate context modifier with zero allocation
    #[inline]
    pub const fn modifier(&self) -> f32 {
        match self {
            ImportanceContext::UserInput => 0.2,           // User-driven, important
            ImportanceContext::SystemResponse => 0.0,      // Neutral
            ImportanceContext::SuccessfulExecution => 0.1, // Positive outcome
            ImportanceContext::ErrorCondition => -0.2,     // Negative outcome
            ImportanceContext::BackgroundProcess => -0.1,  // Less important
            ImportanceContext::CriticalOperation => 0.3,   // Very important
        }
    }
}

/// Global atomic counter for memory node IDs - zero allocation, blazing-fast
static MEMORY_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate next memory ID with zero allocation and blazing-fast performance
#[inline]
#[must_use]
pub fn next_memory_id() -> u64 {
    MEMORY_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Calculate memory importance with zero allocation and blazing-fast performance
#[inline]
#[must_use]
pub fn calculate_importance(
    memory_type: &MemoryType,
    context: ImportanceContext,
    content_length: usize,
) -> f32 {
    let base = memory_type.base_importance();
    let context_mod = context.modifier();
    
    // Content length modifier: longer content gets slight boost, capped at 0.1
    let length_mod = if content_length > 1000 {
        0.1
    } else if content_length > 100 {
        0.05
    } else {
        0.0
    };
    
    // Clamp final importance between 0.0 and 1.0
    (base + context_mod + length_mod).clamp(0.0, 1.0)
}

#[derive(Debug, Clone)]
pub struct MemoryNode {
    pub id: u64,
    pub content: String,
    pub memory_type: MemoryType,
    pub metadata: MemoryMetadata,
    pub embedding: Option<Vec<f32>>}

#[derive(Debug, Clone)]
pub struct MemoryMetadata {
    pub importance: f32,
    pub last_accessed: SystemTime,
    pub creation_time: SystemTime}

#[derive(Debug, Clone)]
pub struct MemoryRelationship {
    pub id: u64,
    pub from_id: u64,
    pub to_id: u64,
    pub relationship_type: String}

pub trait VectorStoreIndexDyn: Send + Sync {
    fn top_n(
        &self,
        query: &str,
        n: usize,
    ) -> Pin<Box<dyn Stream<Item = ZeroOneOrMany<(f64, String, Value)>> + Send>>;
    fn top_n_ids(
        &self,
        query: &str,
        n: usize,
    ) -> Pin<Box<dyn Stream<Item = ZeroOneOrMany<(f64, String)>> + Send>>;
}

pub struct VectorStoreIndex {
    pub backend: Box<dyn VectorStoreIndexDyn>}

impl VectorStoreIndex {
    // Direct creation from backend
    pub fn with_backend<B: VectorStoreIndexDyn + 'static>(backend: B) -> Self {
        VectorStoreIndex {
            backend: Box::new(backend)}
    }

    // VectorQueryBuilder moved to cyrup/src/builders/memory.rs
}