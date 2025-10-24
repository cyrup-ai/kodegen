use std::sync::Arc;
use std::time::SystemTime;

use crossbeam_skiplist::SkipMap;
use uuid::Uuid;

use super::super::types::{BaseMemory, MemoryContent, MemoryTypeEnum};
use super::{AlignedEmbedding, MemoryNode, MemoryNodeMetadata, MemoryNodeStats};

impl MemoryNode {
    /// Create new memory node with generated UUID
    #[inline]
    pub fn new(memory_type: MemoryTypeEnum, content: MemoryContent) -> Self {
        let id = Uuid::new_v4();
        let base_memory = BaseMemory::new(id, memory_type, content);

        Self {
            base_memory,
            embedding: None,
            metadata: Arc::new(MemoryNodeMetadata::new()),
            relationships: Arc::new(SkipMap::new()),
            stats: Arc::new(MemoryNodeStats::new()),
        }
    }

    /// Create memory node with specific UUID
    #[inline]
    pub fn with_id(id: Uuid, memory_type: MemoryTypeEnum, content: MemoryContent) -> Self {
        let base_memory = BaseMemory::new(id, memory_type, content);

        Self {
            base_memory,
            embedding: None,
            metadata: Arc::new(MemoryNodeMetadata::new()),
            relationships: Arc::new(SkipMap::new()),
            stats: Arc::new(MemoryNodeStats::new()),
        }
    }

    /// Get node ID
    #[inline]
    pub fn id(&self) -> Uuid {
        self.stats.record_read();
        self.base_memory.id
    }

    /// Get base memory reference
    #[inline]
    pub fn base_memory(&self) -> &BaseMemory {
        self.stats.record_read();
        &self.base_memory
    }

    /// Get memory type
    #[inline]
    pub fn memory_type(&self) -> MemoryTypeEnum {
        self.stats.record_read();
        self.base_memory.memory_type
    }

    /// Get content reference
    #[inline]
    pub fn content(&self) -> &MemoryContent {
        self.stats.record_read();
        &self.base_memory.content
    }

    /// Get embedding reference
    #[inline]
    pub fn embedding(&self) -> Option<&AlignedEmbedding> {
        self.stats.record_read();
        self.embedding.as_ref()
    }

    /// Get creation time
    #[inline]
    pub fn creation_time(&self) -> SystemTime {
        self.stats.record_read();
        self.base_memory.created_at
    }

    /// Get last accessed time from stats
    #[inline]
    pub fn last_accessed(&self) -> SystemTime {
        self.stats
            .last_access_time()
            .unwrap_or(self.base_memory.created_at)
    }

    /// Get importance from metadata
    #[inline]
    pub fn importance(&self) -> f32 {
        self.stats.record_read();
        self.metadata.importance
    }
}
