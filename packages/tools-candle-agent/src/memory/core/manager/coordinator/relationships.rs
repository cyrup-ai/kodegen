//! Memory relationship management

use futures_util::StreamExt;

use crate::memory::MemoryRelationship;
use crate::memory::core::manager::surreal::trait_def::MemoryManager;
use crate::memory::utils::{Error, Result};

use super::lifecycle::MemoryCoordinator;

impl MemoryCoordinator {
    /// Add a relationship between memories using SurrealDB's native capabilities
    pub async fn add_relationship(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<MemoryRelationship> {
        let mut relationship = MemoryRelationship::new(
            source_id.to_string(),
            target_id.to_string(),
            relationship_type,
        );

        if let Some(metadata) = metadata {
            relationship = relationship.with_metadata(metadata);
        }

        // Store relationship in SurrealDB
        let stored_relationship = self
            .surreal_manager
            .create_relationship(relationship)
            .await?;

        Ok(stored_relationship)
    }

    /// Get relationships for a memory using SurrealDB's native capabilities
    pub async fn get_relationships(&self, memory_id: &str) -> Result<Vec<MemoryRelationship>> {
        // Use SurrealDB's native relationship retrieval directly
        let relationship_stream = self.surreal_manager.get_relationships(memory_id);

        // Collect results using StreamExt::collect()
        let relationships: Vec<_> = relationship_stream.collect().await;

        // Convert to MemoryRelationships with proper error handling
        let mut result_relationships = Vec::new();
        for relationship_result in relationships {
            match relationship_result {
                Ok(relationship) => result_relationships.push(relationship),
                Err(e) => {
                    return Err(Error::Internal(format!(
                        "Failed to retrieve relationships: {}",
                        e
                    )));
                }
            }
        }

        Ok(result_relationships)
    }
}
