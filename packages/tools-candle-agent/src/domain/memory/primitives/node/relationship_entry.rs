use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::types::RelationshipType;

/// Lock-free relationship entry for concurrent access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRelationshipEntry {
    /// Target node ID
    pub target_id: Uuid,
    /// Relationship type
    pub relationship_type: RelationshipType,
    /// Relationship strength (0.0 to 1.0)
    pub strength: f32,
    /// Creation timestamp
    pub created_at: SystemTime,
}

impl MemoryRelationshipEntry {
    /// Create new relationship entry
    #[inline]
    #[must_use]
    pub fn new(target_id: Uuid, relationship_type: RelationshipType, strength: f32) -> Self {
        Self {
            target_id,
            relationship_type,
            strength: strength.clamp(0.0, 1.0),
            created_at: SystemTime::now(),
        }
    }
}
