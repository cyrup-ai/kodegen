//! Type definitions for SurrealDB memory manager.
//!
//! This module contains data structures used for creating and managing
//! memory nodes and relationships in the SurrealDB backend.

use serde::{Deserialize, Serialize};

use crate::memory::primitives::MemoryNode;
use crate::memory::primitives::MemoryRelationship;
use crate::memory::primitives::types::MemoryTypeEnum;
use crate::memory::schema::memory_schema::MemoryMetadataSchema;

/// Content structure for creating/updating memory nodes (without ID)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MemoryNodeCreateContent {
    pub content: String,
    pub content_hash: i64,
    pub memory_type: MemoryTypeEnum,
    pub metadata: MemoryMetadataSchema,
}

impl From<&MemoryNode> for MemoryNodeCreateContent {
    fn from(memory: &MemoryNode) -> Self {
        Self {
            content: memory.content.text.clone(),
            content_hash: memory.content_hash,
            memory_type: memory.memory_type,
            metadata: MemoryMetadataSchema {
                created_at: memory.metadata.created_at,
                last_accessed_at: memory
                    .metadata
                    .last_accessed_at
                    .unwrap_or(memory.metadata.created_at),
                importance: memory.metadata.importance,
                embedding: memory.metadata.embedding.clone(),
                custom: memory.metadata.custom.clone(),
            },
        }
    }
}

/// Content structure for creating relationships (without ID)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RelationshipCreateContent {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub metadata: serde_json::Value,
    pub created_at: u64,
    pub updated_at: u64,
    pub strength: f32,
}

impl From<&MemoryRelationship> for RelationshipCreateContent {
    fn from(relationship: &MemoryRelationship) -> Self {
        // Preserve timestamps if present, generate if absent
        let now = crate::memory::utils::current_timestamp_ms();

        Self {
            source_id: relationship.source_id.clone(),
            target_id: relationship.target_id.clone(),
            relationship_type: relationship.relationship_type.clone(),
            metadata: relationship
                .metadata
                .clone()
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
            created_at: relationship.created_at.unwrap_or(now),
            updated_at: relationship.updated_at.unwrap_or(now),
            strength: relationship.strength.unwrap_or(1.0),
        }
    }
}

/// Export data structure containing memories and relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub memories: Vec<MemoryNode>,
    pub relationships: Vec<MemoryRelationship>,
}
