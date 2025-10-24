use std::sync::Arc;

use super::super::types::{MemoryError, MemoryResult};
use super::{AlignedEmbedding, MemoryNode};

impl MemoryNode {
    /// Set embedding with SIMD alignment
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if embedding vector is empty
    pub fn set_embedding(&mut self, embedding: Vec<f32>) -> MemoryResult<()> {
        if embedding.is_empty() {
            return Err(MemoryError::invalid_content("Embedding cannot be empty"));
        }

        self.stats.record_write();
        self.embedding = Some(AlignedEmbedding::new(embedding));
        Ok(())
    }

    /// Set importance with validation
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if importance value is not between 0.0 and 1.0
    pub fn set_importance(&mut self, importance: f32) -> MemoryResult<()> {
        if !(0.0..=1.0).contains(&importance) {
            return Err(MemoryError::invalid_content(
                "Importance must be between 0.0 and 1.0",
            ));
        }

        self.stats.record_write();

        // Update metadata atomically by cloning and replacing
        let mut new_metadata = (*self.metadata).clone();
        new_metadata.importance = importance;
        new_metadata.version += 1;

        self.metadata = Arc::new(new_metadata);
        Ok(())
    }

    /// Add keyword to metadata
    pub fn add_keyword(&mut self, keyword: impl Into<Arc<str>>) {
        self.stats.record_write();

        let mut new_metadata = (*self.metadata).clone();
        new_metadata.add_keyword(keyword);

        self.metadata = Arc::new(new_metadata);
    }

    /// Add tag to metadata
    pub fn add_tag(&mut self, tag: impl Into<Arc<str>>) {
        self.stats.record_write();

        let mut new_metadata = (*self.metadata).clone();
        new_metadata.add_tag(tag);

        self.metadata = Arc::new(new_metadata);
    }

    /// Set custom metadata value
    pub fn set_custom_metadata(&mut self, key: impl Into<Arc<str>>, value: serde_json::Value) {
        self.stats.record_write();

        let mut new_metadata = (*self.metadata).clone();
        new_metadata.set_custom(key, value);

        self.metadata = Arc::new(new_metadata);
    }
}
