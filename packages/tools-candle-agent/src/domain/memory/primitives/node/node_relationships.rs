use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::SystemTime;

use uuid::Uuid;

use super::super::types::{
    MemoryContent, MemoryError, MemoryResult, MemoryTypeEnum, RelationshipType,
};
use super::{MemoryNode, MemoryNodeMetadata, MemoryNodeStats, MemoryRelationshipEntry};

impl MemoryNode {
    /// Add relationship with lock-free skip-list
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if attempting to create a self-relationship
    pub fn add_relationship(
        &self,
        target_id: Uuid,
        relationship_type: RelationshipType,
        strength: f32,
    ) -> MemoryResult<()> {
        if target_id == self.base_memory.id {
            return Err(MemoryError::invalid_content(
                "Cannot create self-relationship",
            ));
        }

        let entry = MemoryRelationshipEntry::new(target_id, relationship_type, strength);
        self.relationships.insert(target_id, entry);
        self.stats
            .relationship_count
            .fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Remove relationship by target ID
    pub fn remove_relationship(&self, target_id: Uuid) -> bool {
        let removed = self.relationships.remove(&target_id).is_some();
        if removed {
            self.stats
                .relationship_count
                .fetch_sub(1, Ordering::Relaxed);
        }
        removed
    }

    /// Get relationship to specific target
    pub fn get_relationship(&self, target_id: Uuid) -> Option<MemoryRelationshipEntry> {
        self.stats.record_read();
        self.relationships
            .get(&target_id)
            .map(|entry| entry.value().clone())
    }

    /// List all relationships
    pub fn list_relationships(&self) -> Vec<(Uuid, MemoryRelationshipEntry)> {
        self.stats.record_read();
        self.relationships
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Get access statistics
    #[inline]
    pub fn stats(&self) -> &MemoryNodeStats {
        &self.stats
    }

    /// Calculate similarity with another node using embeddings
    pub fn calculate_similarity(&self, other: &Self) -> Option<f32> {
        self.stats.record_read();
        other.stats.record_read();

        match (&self.embedding, &other.embedding) {
            (Some(embedding1), Some(embedding2)) => embedding1.cosine_similarity(embedding2),
            _ => None,
        }
    }

    /// Reset node to clean state for pool reuse while preserving allocations
    ///
    /// This method efficiently clears all node data while preserving heap allocations
    /// for better performance in pooling scenarios. Preserves String and Vec capacities.
    ///
    /// # Arguments
    /// * `memory_type` - The memory type to reset the node to
    ///
    /// # Returns
    /// * `MemoryResult<()>` - Ok if reset successful
    ///
    /// # Errors
    /// Returns error if the reset operation fails
    pub async fn reset(&mut self, memory_type: MemoryTypeEnum) -> MemoryResult<()> {
        // 1. Update timestamps to now
        let now = SystemTime::now();
        self.base_memory.created_at = now;
        self.base_memory.updated_at = now;

        // 2. Update memory type
        self.base_memory.memory_type = memory_type;

        // 3. Reset content while preserving String capacity
        match &mut self.base_memory.content {
            MemoryContent::Text(s) => {
                s.clear(); // Preserves capacity
            }
            _ => {
                // Replace with empty Text variant with pre-allocated capacity
                self.base_memory.content = MemoryContent::text(String::with_capacity(1024));
            }
        }

        // 4. Reset embedding vector while preserving capacity
        if let Some(ref mut emb) = self.embedding {
            let dim = emb.dimension;
            emb.data.clear(); // Preserves capacity
            emb.data.resize(dim, 0.0); // Refill with zeros to maintain dimension
        }

        // 5. Clear base_memory metadata HashMap
        {
            let mut meta = self.base_memory.metadata.write().await;
            meta.clear();
        }

        // 6. Replace metadata Arc (cheap allocation, simpler than cloning and clearing)
        self.metadata = Arc::new(MemoryNodeMetadata::new());

        // 7. Clear relationships skiplist
        self.relationships.clear();

        // 8. Reset all atomic statistics counters
        self.stats.access_count.store(0, Ordering::Relaxed);
        self.stats.read_count.store(0, Ordering::Relaxed);
        self.stats.write_count.store(0, Ordering::Relaxed);
        self.stats.relationship_count.store(0, Ordering::Relaxed);
        self.stats.last_access_nanos.store(0, Ordering::Relaxed);

        Ok(())
    }
}
