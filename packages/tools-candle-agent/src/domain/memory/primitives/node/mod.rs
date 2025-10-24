// Module declarations
pub mod embedding;
pub mod metadata;
pub mod node_core;
pub mod node_relationships;
pub mod node_setters;
pub mod relationship_entry;
pub mod serde_impls;
pub mod stats;
pub mod trait_impls;

// Re-exports for convenience
pub use embedding::AlignedEmbedding;
pub use metadata::MemoryNodeMetadata;
pub use relationship_entry::MemoryRelationshipEntry;
pub use stats::MemoryNodeStats;

// Standard library imports
use std::sync::Arc;

// External crate imports
use crossbeam_skiplist::SkipMap;
use uuid::Uuid;

// Internal imports
use super::types::BaseMemory;

/// High-performance memory node with concurrent design
///
/// Features:
/// - UUID-based node identification with inline generation
/// - SIMD-aligned embedding vectors for AVX2/NEON optimization
/// - `AtomicU64` for concurrent access statistics and version tracking
///   tokio async tasks-skiplist
#[derive(Debug, Clone)]
pub struct MemoryNode {
    /// Base memory with core data
    pub base_memory: BaseMemory,

    /// SIMD-aligned embedding vector for AVX2/NEON optimization
    pub embedding: Option<AlignedEmbedding>,

    /// Cache-padded metadata to prevent false sharing
    pub metadata: Arc<MemoryNodeMetadata>,

    /// Lock-free relationship tracking with skip-list
    pub relationships: Arc<SkipMap<Uuid, MemoryRelationshipEntry>>,

    /// Atomic access statistics for concurrent monitoring
    pub stats: Arc<MemoryNodeStats>,
}
