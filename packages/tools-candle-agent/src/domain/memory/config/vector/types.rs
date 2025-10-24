use serde::{Deserialize, Serialize};

/// Vector store types with performance characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum VectorStoreType {
    /// `SurrealDB` vector store - Multi-model database with vector support
    SurrealDB = 0,
    /// In-memory vector store - Fastest for small datasets
    Memory = 1,
    /// FAISS vector store - Facebook AI Similarity Search
    FAISS = 2,
    /// HNSW vector store - Hierarchical Navigable Small World
    HNSW = 3,
    /// Annoy vector store - Approximate Nearest Neighbors Oh Yeah
    Annoy = 4,
    /// Milvus vector store - Purpose-built vector database
    Milvus = 5,
    /// Pinecone vector store - Managed vector database
    Pinecone = 6,
}

impl VectorStoreType {
    /// Get recommended index type for vector store
    #[inline]
    #[must_use]
    pub const fn recommended_index_type(&self) -> IndexType {
        match self {
            Self::FAISS | Self::Milvus => IndexType::IVFPQ,
            Self::HNSW => IndexType::HNSW,
            Self::Annoy => IndexType::Annoy,
            Self::SurrealDB | Self::Memory | Self::Pinecone => IndexType::FlatIP,
        }
    }

    /// Check if store supports batch operations
    #[inline]
    #[must_use]
    pub const fn supports_batch_operations(&self) -> bool {
        match self {
            Self::SurrealDB | Self::FAISS | Self::HNSW | Self::Milvus | Self::Pinecone => true,
            Self::Memory | Self::Annoy => false,
        }
    }

    /// Get optimal batch size for store type
    #[inline]
    #[must_use]
    pub const fn optimal_batch_size(&self) -> usize {
        match self {
            Self::SurrealDB | Self::HNSW => 1000,
            Self::Memory | Self::Pinecone => 100,
            Self::FAISS => 10000,
            Self::Annoy => 1,
            Self::Milvus => 2000,
        }
    }

    /// Check if store supports real-time updates
    #[inline]
    #[must_use]
    pub const fn supports_realtime_updates(&self) -> bool {
        match self {
            Self::SurrealDB | Self::Memory | Self::Milvus | Self::Pinecone => true,
            Self::FAISS | Self::HNSW | Self::Annoy => false,
        }
    }
}

impl std::fmt::Display for VectorStoreType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SurrealDB => write!(f, "surrealdb"),
            Self::Memory => write!(f, "memory"),
            Self::FAISS => write!(f, "faiss"),
            Self::HNSW => write!(f, "hnsw"),
            Self::Annoy => write!(f, "annoy"),
            Self::Milvus => write!(f, "milvus"),
            Self::Pinecone => write!(f, "pinecone"),
        }
    }
}

/// Distance metrics with SIMD optimization support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DistanceMetric {
    /// Cosine similarity - Most common for semantic search
    Cosine = 0,
    /// Euclidean distance - L2 norm
    Euclidean = 1,
    /// Manhattan distance - L1 norm
    Manhattan = 2,
    /// Dot product - Inner product similarity
    DotProduct = 3,
    /// Hamming distance - For binary vectors
    Hamming = 4,
    /// Jaccard similarity - For sparse vectors
    Jaccard = 5,
}

impl DistanceMetric {
    /// Check if metric supports SIMD optimization
    #[inline]
    #[must_use]
    pub const fn supports_simd(&self) -> bool {
        match self {
            Self::Cosine | Self::Euclidean | Self::DotProduct => true,
            Self::Manhattan | Self::Hamming | Self::Jaccard => false,
        }
    }

    /// Get recommended SIMD instruction set for metric
    #[inline]
    #[must_use]
    pub const fn recommended_simd_set(&self) -> super::simd::SimdInstructionSet {
        use super::simd::SimdInstructionSet;
        match self {
            Self::Cosine | Self::DotProduct => SimdInstructionSet::AVX2,
            Self::Euclidean => SimdInstructionSet::AVX512,
            _ => SimdInstructionSet::None,
        }
    }

    /// Check if metric is symmetric
    #[inline]
    #[must_use]
    pub const fn is_symmetric(&self) -> bool {
        match self {
            Self::Cosine | Self::Euclidean | Self::Manhattan | Self::Hamming | Self::Jaccard => {
                true
            }
            Self::DotProduct => false,
        }
    }

    /// Get value range for metric
    #[inline]
    #[must_use]
    pub const fn value_range(&self) -> (f32, f32) {
        match self {
            Self::Cosine | Self::Jaccard => (-1.0, 1.0),
            Self::Euclidean | Self::Manhattan | Self::Hamming => (0.0, f32::INFINITY),
            Self::DotProduct => (f32::NEG_INFINITY, f32::INFINITY),
        }
    }
}

impl std::fmt::Display for DistanceMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cosine => write!(f, "cosine"),
            Self::Euclidean => write!(f, "euclidean"),
            Self::Manhattan => write!(f, "manhattan"),
            Self::DotProduct => write!(f, "dot_product"),
            Self::Hamming => write!(f, "hamming"),
            Self::Jaccard => write!(f, "jaccard"),
        }
    }
}

/// Index types for vector stores
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum IndexType {
    /// Flat index with inner product
    FlatIP = 0,
    /// Flat index with L2 distance
    FlatL2 = 1,
    /// Inverted file with product quantization
    IVFPQ = 2,
    /// Hierarchical Navigable Small World
    HNSW = 3,
    /// Approximate nearest neighbors optimized for memory
    Annoy = 4,
    /// Locality sensitive hashing
    LSH = 5,
    /// Scalar quantization
    SQ = 6,
}

impl IndexType {
    /// Check if index type supports exact search
    #[inline]
    #[must_use]
    pub const fn supports_exact_search(&self) -> bool {
        match self {
            Self::FlatIP | Self::FlatL2 => true,
            Self::IVFPQ | Self::HNSW | Self::Annoy | Self::LSH | Self::SQ => false,
        }
    }

    /// Get memory usage multiplier compared to raw vectors
    #[inline]
    #[must_use]
    pub const fn memory_multiplier(&self) -> f32 {
        match self {
            Self::FlatIP | Self::FlatL2 => 1.0,
            Self::IVFPQ => 0.1,
            Self::HNSW => 1.5,
            Self::Annoy => 2.0,
            Self::LSH => 1.2,
            Self::SQ => 0.25,
        }
    }

    /// Get build time complexity relative to vector count
    #[inline]
    #[must_use]
    pub const fn build_complexity(&self) -> &'static str {
        match self {
            Self::FlatIP | Self::FlatL2 => "O(1)",
            Self::IVFPQ | Self::LSH | Self::SQ => "O(n)",
            Self::HNSW | Self::Annoy => "O(n log n)",
        }
    }
}

/// Memory allocation strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum AllocationStrategy {
    /// Standard system allocator
    System = 0,
    /// Memory pool allocator for frequent allocations
    Pool = 1,
    /// Arena allocator for batch operations
    Arena = 2,
    /// NUMA-aware allocator
    NUMA = 3,
}
