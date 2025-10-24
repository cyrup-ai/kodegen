//! Memory node builder implementations with zero-allocation, lock-free design
//!
//! All memory node construction logic and builder patterns.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

use cyrup_sugars::{OneOrMany, ZeroOneOrMany};
use crate::domain::memory::primitives::types::{
    CandleBaseMemory as BaseMemory, CandleMemoryContent as MemoryContent, CandleMemoryError as MemoryError, CandleMemoryResult as MemoryResult, CandleMemoryTypeEnum as MemoryTypeEnum, 
};
use crate::domain::memory::primitives::node::CandleMemoryNode as MemoryNode;

/// Ergonomic builder for memory nodes with zero-allocation move semantics
#[derive(Debug, Default)]
pub struct MemoryNodeBuilder {
    id: Option<Uuid>,
    memory_type: Option<MemoryTypeEnum>,
    content: Option<MemoryContent>,
    embedding: ZeroOneOrMany<f32>,
    importance: Option<f32>,
    keywords: OneOrMany<Arc<str>>,
    tags: OneOrMany<Arc<str>>,
    custom_metadata: HashMap<Arc<str>, serde_json::Value>,
}

impl MemoryNodeBuilder {
    /// Create new builder
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set node ID (generates UUID if not set)
    #[must_use]
    #[inline]
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Set memory type
    #[must_use]
    #[inline]
    pub fn with_memory_type(mut self, memory_type: MemoryTypeEnum) -> Self {
        self.memory_type = Some(memory_type);
        self
    }

    /// Set content
    #[must_use]
    #[inline]
    pub fn with_content(mut self, content: MemoryContent) -> Self {
        self.content = Some(content);
        self
    }

    /// Set text content (convenience method)
    #[must_use]
    #[inline]
    pub fn with_text(mut self, text: impl Into<Arc<str>>) -> Self {
        self.content = Some(MemoryContent::text(text));
        self
    }

    /// Set base memory
    #[must_use]
    #[inline]
    pub fn with_base_memory(mut self, base_memory: BaseMemory) -> Self {
        self.id = Some(base_memory.id);
        self.memory_type = Some(base_memory.memory_type);
        self.content = Some(base_memory.content);
        self
    }

    /// Set embedding
    #[must_use]
    #[inline]
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set importance
    #[must_use]
    #[inline]
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = Some(importance);
        self
    }

    /// Set creation time (for compatibility)
    #[must_use]
    #[inline]
    pub fn with_creation_time(self, _time: SystemTime) -> Self {
        // Creation time is handled by BaseMemory constructor
        self
    }

    /// Set last accessed time (for compatibility)
    #[must_use]
    #[inline]
    pub fn with_last_accessed(self, _time: SystemTime) -> Self {
        // Last accessed is tracked by stats automatically
        self
    }

    /// Add keyword
    #[must_use]
    #[inline]
    pub fn with_keyword(mut self, keyword: impl Into<Arc<str>>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    /// Add tag
    #[must_use]
    #[inline]
    pub fn with_tag(mut self, tag: impl Into<Arc<str>>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add custom metadata
    #[must_use]
    #[inline]
    pub fn with_custom_metadata(
        mut self,
        key: impl Into<Arc<str>>,
        value: serde_json::Value,
    ) -> Self {
        self.custom_metadata.insert(key.into(), value);
        self
    }

    /// Build the memory node with validation using pool for zero-allocation
    pub async fn build(self) -> MemoryResult<MemoryNode> {
        let memory_type = self
            .memory_type
            .ok_or_else(|| MemoryError::validation("Memory type is required"))?;

        let content = self.content.unwrap_or_default();

        let id = self.id.unwrap_or_else(Uuid::new_v4);

        // Try to use pooled node for zero-allocation
        let mut node = if let Some(pooled_result) = crate::domain::memory::pool::acquire_pooled_node().await {
            // Propagate pool acquisition errors
            let mut pooled_node = pooled_result?;
            // Initialize the pooled node
            pooled_node.initialize(content.text.to_string(), memory_type).await?;
            // Take ownership from pool
            pooled_node.take().unwrap_or_else(|| MemoryNode::with_id(id, memory_type, content))
        } else {
            // Fallback to direct allocation if pool unavailable
            MemoryNode::with_id(id, memory_type, content)
        };

        // Set embedding if provided
        if let Some(embedding) = self.embedding {
            node.set_embedding(embedding)?;
        }

        // Set importance if provided
        if let Some(importance) = self.importance {
            node.set_importance(importance)?;
        }

        // Add keywords
        for keyword in self.keywords {
            node.add_keyword(keyword);
        }

        // Add tags
        for tag in self.tags {
            node.add_tag(tag);
        }

        // Add custom metadata
        for (key, value) in self.custom_metadata {
            node.set_custom_metadata(key, value);
        }

        Ok(node)
    }
}