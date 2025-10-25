// src/schema/relationship_schema.rs
//! Relationship schema definition.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::RecordId;

use crate::memory::utils;

/// Relationship schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Relationship ID
    pub id: RecordId,
    /// Source memory ID
    pub source_id: String,
    /// Target memory ID
    pub target_id: String,
    /// Relationship type
    pub relationship_type: String,
    /// Relationship metadata
    pub metadata: Value,
    /// Creation timestamp (milliseconds since epoch)
    pub created_at: u64,
    /// Last update timestamp (milliseconds since epoch)
    pub updated_at: u64,
    /// Relationship strength (0.0 to 1.0)
    #[serde(default)]
    pub strength: f32,
    /// Additional fields
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub additional_fields: HashMap<String, Value>,
}

impl Relationship {
    /// Create a new relationship
    pub fn new(
        source_id: impl Into<String>,
        target_id: impl Into<String>,
        relationship_type: impl Into<String>,
    ) -> Self {
        let now = utils::current_timestamp_ms();
        let id = utils::generate_id();

        Self {
            id: RecordId::from(("memory_relationship", id.as_str())),
            source_id: source_id.into(),
            target_id: target_id.into(),
            relationship_type: relationship_type.into(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: now,
            updated_at: now,
            strength: 1.0,
            additional_fields: HashMap::new(),
        }
    }

    /// Create a new relationship with ID
    pub fn new_with_id(
        id: impl Into<String>,
        source_id: impl Into<String>,
        target_id: impl Into<String>,
        relationship_type: impl Into<String>,
    ) -> Self {
        let now = utils::current_timestamp_ms();
        let id_str = id.into();

        Self {
            id: RecordId::from(("memory_relationship", id_str.as_str())),
            source_id: source_id.into(),
            target_id: target_id.into(),
            relationship_type: relationship_type.into(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: now,
            updated_at: now,
            strength: 1.0,
            additional_fields: HashMap::new(),
        }
    }

    /// Set metadata
    #[must_use]
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set strength
    #[must_use]
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: Value) {
        self.metadata = metadata;
        self.updated_at = utils::current_timestamp_ms();
    }

    /// Update strength
    pub fn update_strength(&mut self, strength: f32) {
        self.strength = strength.clamp(0.0, 1.0);
        self.updated_at = utils::current_timestamp_ms();
    }

    /// Get creation timestamp as ISO 8601 string
    pub fn created_at_iso8601(&self) -> String {
        utils::timestamp_to_iso8601(self.created_at)
    }

    /// Get update timestamp as ISO 8601 string
    pub fn updated_at_iso8601(&self) -> String {
        utils::timestamp_to_iso8601(self.updated_at)
    }

    /// Get metadata value
    pub fn get_metadata_value<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        if let serde_json::Value::Object(ref map) = self.metadata {
            map.get(key)
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        } else {
            None
        }
    }

    /// Set metadata value
    pub fn set_metadata_value<T: serde::Serialize>(
        &mut self,
        key: &str,
        value: T,
    ) -> Result<(), serde_json::Error> {
        let value = serde_json::to_value(value)?;

        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.insert(key.to_string(), value);
        } else {
            // If metadata is not an object, create a new object
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), value);
            self.metadata = serde_json::Value::Object(map);
        }

        self.updated_at = utils::current_timestamp_ms();
        Ok(())
    }

    /// Remove metadata value
    pub fn remove_metadata_value(&mut self, key: &str) {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.remove(key);
        }

        self.updated_at = utils::current_timestamp_ms();
    }

    /// Get additional field value
    pub fn get_additional_field<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.additional_fields
            .get(key)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// Set additional field value
    pub fn set_additional_field<T: serde::Serialize>(
        &mut self,
        key: &str,
        value: T,
    ) -> Result<(), serde_json::Error> {
        let value = serde_json::to_value(value)?;
        self.additional_fields.insert(key.to_string(), value);
        self.updated_at = utils::current_timestamp_ms();
        Ok(())
    }

    /// Remove additional field
    pub fn remove_additional_field(&mut self, key: &str) {
        self.additional_fields.remove(key);
        self.updated_at = utils::current_timestamp_ms();
    }

    /// Check if this relationship is between the specified memories
    pub fn is_between(&self, memory_id1: &str, memory_id2: &str) -> bool {
        (self.source_id == memory_id1 && self.target_id == memory_id2)
            || (self.source_id == memory_id2 && self.target_id == memory_id1)
    }

    /// Check if this relationship involves the specified memory
    pub fn involves(&self, memory_id: &str) -> bool {
        self.source_id == memory_id || self.target_id == memory_id
    }

    /// Get the other memory ID in the relationship
    pub fn get_other_memory_id(&self, memory_id: &str) -> Option<&str> {
        if self.source_id == memory_id {
            Some(&self.target_id)
        } else if self.target_id == memory_id {
            Some(&self.source_id)
        } else {
            None
        }
    }
}

