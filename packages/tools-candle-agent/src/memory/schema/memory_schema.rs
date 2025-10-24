//! Database schema for memory nodes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;
use uuid::Uuid;

use crate::memory::core::primitives::types::MemoryTypeEnum;

/// Database schema for memory nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNodeSchema {
    /// Unique identifier
    pub id: RecordId,
    /// Content of the memory
    pub content: String,
    /// Content hash for fast deduplication and lookup
    pub content_hash: i64,
    /// Type of memory
    pub memory_type: MemoryTypeEnum,
    /// Metadata associated with the memory
    pub metadata: MemoryMetadataSchema,
}

/// Database schema for memory metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadataSchema {
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last accessed time
    pub last_accessed_at: DateTime<Utc>,
    /// Importance score (0.0 to 1.0)
    pub importance: f32,
    /// Vector embedding
    pub embedding: Option<Vec<f32>>,
    /// Custom metadata
    #[serde(default)]
    pub custom: serde_json::Value,
}

/// Public memory type for API access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique identifier
    pub id: String,
    /// Content of the memory
    pub content: String,
    /// Type of memory
    pub memory_type: String,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last updated time
    pub updated_at: DateTime<Utc>,
    /// Last accessed time
    pub last_accessed_at: DateTime<Utc>,
    /// Importance score (0.0 to 1.0)
    pub importance: f32,
    /// Vector embedding
    pub embedding: Option<Vec<f32>>,
    /// Tags
    pub tags: Vec<String>,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl Memory {
    /// Create a new memory instance
    pub fn new(content: String, memory_type: MemoryTypeEnum) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Self {
            id,
            content,
            memory_type: memory_type.to_string(),
            created_at: now,
            updated_at: now,
            last_accessed_at: now,
            importance: 0.5,
            embedding: None,
            tags: Vec::new(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Update the last accessed time
    pub fn touch(&mut self) {
        self.last_accessed_at = Utc::now();
    }

    /// Set the embedding vector
    pub fn set_embedding(&mut self, embedding: Vec<f32>) {
        self.embedding = Some(embedding);
        self.updated_at = Utc::now();
    }

    /// Add metadata key-value pair
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.insert(key, value);
            self.updated_at = Utc::now();
        }
    }

    /// Remove metadata by key
    pub fn remove_metadata(&mut self, key: &str) {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.remove(key);
            self.updated_at = Utc::now();
        }
    }
}
