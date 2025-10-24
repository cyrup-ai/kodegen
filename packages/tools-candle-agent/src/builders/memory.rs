//! Memory builder implementations with zero-allocation, lock-free design
//!
//! Trait-based memory builders following cyrup architecture patterns.
//! All builders relocated from domain crate for proper architectural separation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use crate::domain::{
    ZeroOneOrMany,
    memory::{
        CandleAlignedEmbedding, CandleBaseMemory, CandleCompatibilityMode, CandleDatabaseConfig,
        CandleEmbeddingConfig, CandleMemoryContent, CandleMemoryError, CandleMemoryMetadata,
        CandleMemoryNode, CandleMemoryNodeMetadata, CandleMemoryResult, CandleMemorySystemConfig,
        CandleMemoryTypeEnum, CandleVectorStoreConfig, CandleVectorStoreType, DatabaseType,
        DistanceMetric, IndexConfig, IndexType, PerformanceConfig, PoolConfig, SimdConfig,
    },
};
use serde_json::Value;
use uuid::Uuid;

/// Memory node builder trait - main entry point for fluent memory node configuration
pub trait MemoryNodeBuilder {
    fn with_id(self, id: Uuid) -> impl MemoryNodeBuilder;
    fn with_memory_type(self, memory_type: MemoryTypeEnum) -> impl MemoryNodeBuilder;
    fn with_content(self, content: MemoryContent) -> impl MemoryNodeBuilder;
    fn with_text(self, text: impl Into<Arc<str>>) -> impl MemoryNodeBuilder;
    fn with_base_memory(self, base_memory: BaseMemory) -> impl MemoryNodeBuilder;
    fn with_embedding(self, embedding: impl Into<ZeroOneOrMany<f32>>) -> impl MemoryNodeBuilder;
    fn with_importance(self, importance: f32) -> impl MemoryNodeBuilder;
    fn with_creation_time(self, time: SystemTime) -> impl MemoryNodeBuilder;
    fn with_last_accessed(self, time: SystemTime) -> impl MemoryNodeBuilder;
    fn with_keyword(self, keyword: impl Into<Arc<str>>) -> impl MemoryNodeBuilder;
    fn with_tag(self, tag: impl Into<Arc<str>>) -> impl MemoryNodeBuilder;
    fn with_custom_metadata(self, key: impl Into<Arc<str>>, value: Value)
    -> impl MemoryNodeBuilder;
    async fn build(self) -> MemoryResult<MemoryNode>;
}

/// Memory system builder trait - main entry point for fluent memory system configuration
pub trait MemorySystemBuilder {
    fn with_database_config(self, config: DatabaseConfig) -> impl MemorySystemBuilder;
    fn with_vector_config(self, config: VectorStoreConfig) -> impl MemorySystemBuilder;
    fn with_cognitive(self, enabled: bool) -> impl MemorySystemBuilder;
    fn with_compatibility_mode(self, mode: CompatibilityMode) -> impl MemorySystemBuilder;
    fn build(self) -> MemoryResult<MemorySystemConfig>;
}

/// Memory entry point for creating memory builders
pub struct Memory;

impl Memory {
    /// Create memory node builder - EXACT syntax: Memory::node()
    #[inline]
    pub fn node() -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            id: ZeroOneOrMany::None,
            memory_type: ZeroOneOrMany::None,
            content: ZeroOneOrMany::None,
            embedding: ZeroOneOrMany::None,
            importance: ZeroOneOrMany::None,
            creation_time: ZeroOneOrMany::None,
            last_accessed: ZeroOneOrMany::None,
            keywords: ZeroOneOrMany::None,
            tags: ZeroOneOrMany::None,
            custom_metadata: ZeroOneOrMany::None,
        }
    }

    /// Create memory system builder - EXACT syntax: Memory::system()
    #[inline]
    pub fn system() -> impl MemorySystemBuilder {
        MemorySystemBuilderImpl {
            database_config: ZeroOneOrMany::Zero,
            vector_config: ZeroOneOrMany::Zero,
            llm_config: ZeroOneOrMany::Zero,
            enable_cognitive: ZeroOneOrMany::Zero,
            compatibility_mode: ZeroOneOrMany::Zero,
        }
    }

    /// Create optimized memory system - EXACT syntax: Memory::optimized()
    #[inline]
    pub fn optimized() -> MemoryResult<MemorySystemConfig> {
        MemorySystemConfig::optimized()
    }

    /// Create minimal memory system - EXACT syntax: Memory::minimal()
    #[inline]
    pub fn minimal() -> MemoryResult<MemorySystemConfig> {
        MemorySystemConfig::minimal()
    }

    /// Create semantic search optimized system - EXACT syntax: Memory::for_semantic_search()
    #[inline]
    pub fn for_semantic_search() -> MemoryResult<MemorySystemConfig> {
        MemorySystemConfig::for_semantic_search()
    }

    /// Create realtime chat optimized system - EXACT syntax: Memory::for_realtime_chat()
    #[inline]
    pub fn for_realtime_chat() -> MemoryResult<MemorySystemConfig> {
        MemorySystemConfig::for_realtime_chat()
    }

    /// Create large scale optimized system - EXACT syntax: Memory::for_large_scale()
    #[inline]
    pub fn for_large_scale() -> MemoryResult<MemorySystemConfig> {
        MemorySystemConfig::for_large_scale()
    }
}

/// Hidden implementation of MemoryNodeBuilder - never seen publicly
struct MemoryNodeBuilderImpl {
    id: ZeroOneOrMany<Uuid>,
    memory_type: ZeroOneOrMany<MemoryTypeEnum>,
    content: ZeroOneOrMany<MemoryContent>,
    embedding: ZeroOneOrMany<f32>,
    importance: ZeroOneOrMany<f32>,
    creation_time: ZeroOneOrMany<SystemTime>,
    last_accessed: ZeroOneOrMany<SystemTime>,
    keywords: ZeroOneOrMany<Arc<str>>,
    tags: ZeroOneOrMany<Arc<str>>,
    custom_metadata: ZeroOneOrMany<(Arc<str>, Value)>,
}

/// Hidden implementation of MemorySystemBuilder - never seen publicly
struct MemorySystemBuilderImpl {
    database_config: ZeroOneOrMany<DatabaseConfig>,
    vector_config: ZeroOneOrMany<VectorStoreConfig>,
    enable_cognitive: ZeroOneOrMany<bool>,
    compatibility_mode: ZeroOneOrMany<CompatibilityMode>,
}

impl MemoryNodeBuilder for MemoryNodeBuilderImpl {
    #[inline]
    fn with_id(self, id: Uuid) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            id: self.id.with_pushed(id),
            ..self
        }
    }

    #[inline]
    fn with_memory_type(self, memory_type: MemoryTypeEnum) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            memory_type: self.memory_type.with_pushed(memory_type),
            ..self
        }
    }

    #[inline]
    fn with_content(self, content: MemoryContent) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            content: self.content.with_pushed(content),
            ..self
        }
    }

    #[inline]
    fn with_text(self, text: impl Into<Arc<str>>) -> impl MemoryNodeBuilder {
        let content = MemoryContent::text(text);
        MemoryNodeBuilderImpl {
            content: self.content.with_pushed(content),
            ..self
        }
    }

    #[inline]
    fn with_base_memory(self, base_memory: BaseMemory) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            id: self.id.with_pushed(base_memory.id),
            memory_type: self.memory_type.with_pushed(base_memory.memory_type),
            content: self.content.with_pushed(base_memory.content),
            creation_time: self.creation_time.with_pushed(base_memory.created_at),
            ..self
        }
    }

    #[inline]
    fn with_embedding(self, embedding: impl Into<ZeroOneOrMany<f32>>) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            embedding: embedding.into(),
            ..self
        }
    }

    #[inline]
    fn with_importance(self, importance: f32) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            importance: self.importance.with_pushed(importance),
            ..self
        }
    }

    #[inline]
    fn with_creation_time(self, time: SystemTime) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            creation_time: self.creation_time.with_pushed(time),
            ..self
        }
    }

    #[inline]
    fn with_last_accessed(self, time: SystemTime) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            last_accessed: self.last_accessed.with_pushed(time),
            ..self
        }
    }

    #[inline]
    fn with_keyword(self, keyword: impl Into<Arc<str>>) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            keywords: self.keywords.with_pushed(keyword.into()),
            ..self
        }
    }

    #[inline]
    fn with_tag(self, tag: impl Into<Arc<str>>) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            tags: self.tags.with_pushed(tag.into()),
            ..self
        }
    }

    #[inline]
    fn with_custom_metadata(
        self,
        key: impl Into<Arc<str>>,
        value: Value,
    ) -> impl MemoryNodeBuilder {
        MemoryNodeBuilderImpl {
            custom_metadata: self.custom_metadata.with_pushed((key.into(), value)),
            ..self
        }
    }

    #[inline]
    async fn build(self) -> MemoryResult<MemoryNode> {
        // Extract required fields with proper error handling
        let memory_type = self
            .memory_type
            .into_iter()
            .next()
            .ok_or_else(|| MemoryError::validation("Memory type is required"))?;

        let content = self.content.into_iter().next().unwrap_or_default();

        let id = self.id.into_iter().next().unwrap_or_else(Uuid::new_v4);

        // Create base memory node with performance optimization
        let now = SystemTime::now();
        let creation_time = self.creation_time.into_iter().next().unwrap_or(now);

        let mut node = MemoryNode::with_id(id, memory_type, content);

        // Apply embedding with validation
        if !self.embedding.is_none() {
            let embedding_data: Vec<f32> = self.embedding.into_iter().collect();
            if !embedding_data.is_empty() {
                node.set_embedding(embedding_data)
                    .map_err(|e| MemoryError::validation(&format!("Invalid embedding: {}", e)))?;
            }
        }

        // Apply importance with validation
        if let Some(importance) = self.importance.into_iter().next() {
            node.set_importance(importance)
                .map_err(|e| MemoryError::validation(&format!("Invalid importance: {}", e)))?;
        }

        // Apply keywords efficiently
        for keyword in self.keywords.into_iter() {
            node.add_keyword(keyword);
        }

        // Apply tags efficiently
        for tag in self.tags.into_iter() {
            node.add_tag(tag);
        }

        // Apply custom metadata efficiently
        for (key, value) in self.custom_metadata.into_iter() {
            node.set_custom_metadata(key, value);
        }

        Ok(node)
    }
}

impl MemorySystemBuilder for MemorySystemBuilderImpl {
    #[inline]
    fn with_database_config(self, config: DatabaseConfig) -> impl MemorySystemBuilder {
        MemorySystemBuilderImpl {
            database_config: self.database_config.with_pushed(config),
            ..self
        }
    }

    #[inline]
    fn with_vector_config(self, config: VectorStoreConfig) -> impl MemorySystemBuilder {
        MemorySystemBuilderImpl {
            vector_config: self.vector_config.with_pushed(config),
            ..self
        }
    }

    #[inline]
    fn with_cognitive(self, enabled: bool) -> impl MemorySystemBuilder {
        MemorySystemBuilderImpl {
            enable_cognitive: self.enable_cognitive.with_pushed(enabled),
            ..self
        }
    }

    #[inline]
    fn with_compatibility_mode(self, mode: CompatibilityMode) -> impl MemorySystemBuilder {
        MemorySystemBuilderImpl {
            compatibility_mode: self.compatibility_mode.with_pushed(mode),
            ..self
        }
    }

    #[inline]
    fn build(self) -> MemoryResult<MemorySystemConfig> {
        let config = MemorySystemConfig {
            database: self.database_config.into_iter().next().unwrap_or_default(),
            vector_store: self.vector_config.into_iter().next().unwrap_or_default(),
            enable_cognitive: self.enable_cognitive.into_iter().next().unwrap_or(true),
            compatibility_mode: self
                .compatibility_mode
                .into_iter()
                .next()
                .unwrap_or_default(),
        };

        // Validate configuration
        config.validate()?;
        Ok(config)
    }
}

/// Extension trait for MemoryNode to add builder functionality
pub trait MemoryNodeBuilderExt {
    /// Get memory node builder - EXACT syntax: memory_node.builder()
    fn builder() -> impl MemoryNodeBuilder;
}

impl MemoryNodeBuilderExt for MemoryNode {
    #[inline]
    fn builder() -> impl MemoryNodeBuilder {
        Memory::node()
    }
}

/// Extension trait for MemorySystemConfig to add builder functionality
pub trait MemorySystemBuilderExt {
    /// Get memory system builder - EXACT syntax: MemorySystemConfig::builder()
    fn builder() -> impl MemorySystemBuilder;
}

impl MemorySystemBuilderExt for MemorySystemConfig {
    #[inline]
    fn builder() -> impl MemorySystemBuilder {
        Memory::system()
    }
}

/// Convenience function for creating memory nodes - EXACT syntax: memory_node()
#[inline]
pub fn memory_node() -> impl MemoryNodeBuilder {
    Memory::node()
}

/// Convenience function for creating memory systems - EXACT syntax: memory_system()
#[inline]
pub fn memory_system() -> impl MemorySystemBuilder {
    Memory::system()
}

/// Advanced memory node builder with specialized configurations
pub struct AdvancedMemoryBuilder;

impl AdvancedMemoryBuilder {
    /// Create high-performance memory node - EXACT syntax: AdvancedMemoryBuilder::high_performance()
    #[inline]
    pub fn high_performance() -> impl MemoryNodeBuilder {
        Memory::node()
            .with_importance(0.9)
            .with_memory_type(MemoryTypeEnum::LongTerm)
    }

    /// Create semantic search optimized node - EXACT syntax: AdvancedMemoryBuilder::semantic_search()
    #[inline]
    pub fn semantic_search() -> impl MemoryNodeBuilder {
        Memory::node()
            .with_importance(0.8)
            .with_memory_type(MemoryTypeEnum::Semantic)
    }

    /// Create episodic memory node - EXACT syntax: AdvancedMemoryBuilder::episodic()
    #[inline]
    pub fn episodic() -> impl MemoryNodeBuilder {
        Memory::node()
            .with_importance(0.7)
            .with_memory_type(MemoryTypeEnum::Episodic)
    }

    /// Create working memory node - EXACT syntax: AdvancedMemoryBuilder::working()
    #[inline]
    pub fn working() -> impl MemoryNodeBuilder {
        Memory::node()
            .with_importance(0.6)
            .with_memory_type(MemoryTypeEnum::Working)
    }
}

/// Advanced memory system builder with specialized configurations
pub struct AdvancedMemorySystemBuilder;

impl AdvancedMemorySystemBuilder {
    /// Create production-ready memory system - EXACT syntax: AdvancedMemorySystemBuilder::production()
    #[inline]
    pub fn production() -> MemoryResult<impl MemorySystemBuilder> {
        let database_config = DatabaseConfig::new(
            DatabaseType::PostgreSQL,
            "postgresql://localhost:5432/cyrup_prod",
            "production",
            "memory_prod",
        )?;

        let vector_config = VectorStoreConfig::new(
            VectorStoreType::FAISS,
            EmbeddingConfig::high_performance(),
            1536,
        )?
        .with_distance_metric(DistanceMetric::Cosine)
        .with_performance_config(PerformanceConfig::optimized(VectorStoreType::FAISS));

        Ok(Memory::system()
            .with_database_config(database_config)
            .with_vector_config(vector_config)
            .with_cognitive(true)
            .with_compatibility_mode(CompatibilityMode::Hybrid))
    }

    /// Create development memory system - EXACT syntax: AdvancedMemorySystemBuilder::development()
    #[inline]
    pub fn development() -> MemoryResult<impl MemorySystemBuilder> {
        let database_config =
            DatabaseConfig::new(DatabaseType::Memory, "memory", "development", "memory_dev")?;

        let vector_config =
            VectorStoreConfig::new(VectorStoreType::Memory, EmbeddingConfig::default(), 768)?;

        Ok(Memory::system()
            .with_database_config(database_config)
            .with_vector_config(vector_config)
            .with_cognitive(false)
            .with_compatibility_mode(CompatibilityMode::Flexible))
    }

    /// Create testing memory system - EXACT syntax: AdvancedMemorySystemBuilder::testing()
    #[inline]
    pub fn testing() -> MemoryResult<impl MemorySystemBuilder> {
        let database_config =
            DatabaseConfig::new(DatabaseType::Memory, "memory", "test", "memory_test")?
                .with_pool_config(PoolConfig::minimal());

        let vector_config =
            VectorStoreConfig::new(VectorStoreType::Memory, EmbeddingConfig::default(), 384)?
                .with_performance_config(PerformanceConfig::minimal());

        Ok(Memory::system()
            .with_database_config(database_config)
            .with_vector_config(vector_config)
            .with_cognitive(false)
            .with_compatibility_mode(CompatibilityMode::Strict))
    }
}
