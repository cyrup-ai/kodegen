//! Core entity types and trait definitions
//!
//! This module provides the Entity trait and BaseEntity implementation
//! for mapping domain objects to graph nodes.

use std::collections::HashMap;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use surrealdb::Value;

use crate::memory::graph::graph_db::{GraphError, Node, Result};

/// Type alias for entity validation functions
pub type EntityValidatorFn = Box<dyn Fn(&dyn Entity) -> Result<()> + Send + Sync>;

/// Entity trait for domain objects
pub trait Entity: Send + Sync + Debug {
    /// Get the entity ID
    fn id(&self) -> &str;

    /// Get the entity type
    fn entity_type(&self) -> &str;

    /// Get an attribute value
    fn get_attribute(&self, name: &str) -> Option<&Value>;

    /// Set an attribute value
    fn set_attribute(&mut self, name: &str, value: Value);

    /// Get all attributes
    fn attributes(&self) -> &HashMap<String, Value>;

    /// Validate the entity
    fn validate(&self) -> Result<()>;

    /// Convert to a graph node
    fn to_node(&self) -> Node;

    /// Create from a graph node
    fn from_node(node: Node) -> Result<Self>
    where
        Self: Sized;
}

/// Base entity implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEntity {
    id: String,
    entity_type: String,
    attributes: HashMap<String, Value>,
}

impl BaseEntity {
    /// Create a new base entity
    pub fn new(id: String, entity_type: String) -> Self {
        Self {
            id,
            entity_type,
            attributes: HashMap::new(),
        }
    }

    /// Create with pre-allocated capacity for attributes
    pub fn with_capacity(id: String, entity_type: String, capacity: usize) -> Self {
        Self {
            id,
            entity_type,
            attributes: HashMap::with_capacity(capacity),
        }
    }

    /// Get memory usage in bytes (approximate)
    pub fn memory_usage(&self) -> usize {
        self.id.len() + self.entity_type.len() + (self.attributes.len() * 64) // Approximate
    }

    /// Bulk set attributes with validation
    pub fn set_attributes_bulk(&mut self, attributes: HashMap<String, Value>) -> Result<()> {
        // Validate all attributes first
        for (key, value) in &attributes {
            self.validate_attribute(key, value)?;
        }

        // If all valid, set them
        self.attributes.extend(attributes);
        Ok(())
    }

    /// Validate a single attribute (can be overridden)
    pub fn validate_attribute(&self, _key: &str, _value: &Value) -> Result<()> {
        // Default validation - accept all
        Ok(())
    }

    /// Remove multiple attributes
    pub fn remove_attributes(&mut self, keys: &[&str]) {
        for key in keys {
            self.attributes.remove(*key);
        }
    }

    /// Check if entity has all required attributes
    pub fn has_required_attributes(&self, required: &[&str]) -> bool {
        required
            .iter()
            .all(|key| self.attributes.contains_key(*key))
    }

    /// Builder pattern method to set an attribute and return self
    #[must_use]
    pub fn with_attribute(mut self, name: &str, value: Value) -> Self {
        self.attributes.insert(name.to_string(), value);
        self
    }
}

impl Entity for BaseEntity {
    fn id(&self) -> &str {
        &self.id
    }

    fn entity_type(&self) -> &str {
        &self.entity_type
    }

    fn get_attribute(&self, name: &str) -> Option<&Value> {
        self.attributes.get(name)
    }

    fn set_attribute(&mut self, name: &str, value: Value) {
        self.attributes.insert(name.to_string(), value);
    }

    fn attributes(&self) -> &HashMap<String, Value> {
        &self.attributes
    }

    fn validate(&self) -> Result<()> {
        // Basic validation - ensure ID and type are not empty
        if self.id.is_empty() {
            return Err(GraphError::ValidationError(
                "Entity ID cannot be empty".to_string(),
            ));
        }

        if self.entity_type.is_empty() {
            return Err(GraphError::ValidationError(
                "Entity type cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    fn to_node(&self) -> Node {
        // Convert surrealdb::Value attributes to serde_json::Value for Node compatibility
        let mut properties = std::collections::HashMap::new();
        for (key, value) in &self.attributes {
            // Convert surrealdb::Value to serde_json::Value via serialization
            let json_value = serde_json::to_value(value)
                .unwrap_or_else(|_| serde_json::Value::String(format!("{:?}", value)));
            properties.insert(key.clone(), json_value);
        }

        properties.insert("id".to_string(), serde_json::Value::String(self.id.clone()));
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(self.entity_type.clone()),
        );

        Node {
            id: self.id.clone(),
            properties,
            labels: vec![self.entity_type.clone()],
        }
    }

    fn from_node(node: Node) -> Result<Self> {
        let id = node
            .properties
            .get("id")
            .and_then(|v| {
                // Try to deserialize as String from SurrealDB Value
                v.as_str().map(|s| s.to_string())
            })
            .ok_or_else(|| GraphError::ValidationError("Node missing ID".to_string()))?;

        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| {
                // Try to deserialize as String from SurrealDB Value
                v.as_str().map(|s| s.to_string())
            })
            .ok_or_else(|| GraphError::ValidationError("Node missing entity_type".to_string()))?;

        // Convert serde_json::Value to surrealdb::Value for attributes
        let mut attributes = HashMap::new();
        for (key, value) in node.properties {
            if key != "id" && key != "entity_type" {
                // Convert serde_json::Value to surrealdb::Value using proper conversion
                let surreal_value = surrealdb::value::to_value(value).unwrap_or_default();

                attributes.insert(key, surreal_value);
            }
        }

        Ok(Self {
            id,
            entity_type,
            attributes,
        })
    }
}
