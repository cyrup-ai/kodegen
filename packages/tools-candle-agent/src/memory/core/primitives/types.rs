// src/memory/memory_type.rs
//! Memory type definitions and traits for the memory system.

use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::str::FromStr;

use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::memory::graph::entity::BaseEntity;
use crate::memory::primitives::metadata::MemoryMetadata;
use crate::memory::utils::Result;
use crate::memory::utils::error::Error;

/// Convert serde_json::Value to surrealdb::Value
fn json_to_surreal_value(json: serde_json::Value) -> surrealdb::Value {
    use surrealdb::value::to_value;

    to_value(json).unwrap_or_default()
}

/// Memory type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MemoryTypeEnum {
    /// Semantic memory (knowledge graph)
    Semantic,
    /// Episodic memory (events and experiences)
    Episodic,
    /// Procedural memory (skills and procedures)
    Procedural,
    /// Working memory (temporary storage)
    Working,
    /// Long-term memory (default)
    #[default]
    LongTerm,
}

impl MemoryTypeEnum {
    /// Convert from string to MemoryTypeEnum
    pub fn from_string(s: &str) -> Result<Self> {
        match s {
            "semantic" => Ok(MemoryTypeEnum::Semantic),
            "episodic" => Ok(MemoryTypeEnum::Episodic),
            "procedural" => Ok(MemoryTypeEnum::Procedural),
            "working" => Ok(MemoryTypeEnum::Working),
            "long_term" => Ok(MemoryTypeEnum::LongTerm),
            _ => Err(Error::InvalidInput(format!("Invalid memory type: {}", s))),
        }
    }
}

impl fmt::Display for MemoryTypeEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryTypeEnum::Semantic => write!(f, "semantic"),
            MemoryTypeEnum::Episodic => write!(f, "episodic"),
            MemoryTypeEnum::Procedural => write!(f, "procedural"),
            MemoryTypeEnum::Working => write!(f, "working"),
            MemoryTypeEnum::LongTerm => write!(f, "long_term"),
        }
    }
}

impl FromStr for MemoryTypeEnum {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_string(s)
    }
}

/// Trait for memory types
pub trait MemoryType: Debug + Send + Sync {
    /// Get the unique identifier of the memory
    fn id(&self) -> &str;

    /// Get the name of the memory
    fn name(&self) -> &str;

    /// Get the description of the memory
    fn description(&self) -> &str;

    /// Get the last updated timestamp
    fn updated_at(&self) -> DateTime<Utc>;

    /// Get the metadata of the memory
    fn metadata(&self) -> &MemoryMetadata;

    /// Get the content of the memory
    fn content(&self) -> &MemoryContent;

    /// Validate the memory
    fn validate(&self) -> Result<()>;

    /// Convert memory to a generic entity representation
    fn to_entity(&self) -> BaseEntity;

    /// Create memory from a generic entity representation
    fn from_entity(entity: BaseEntity) -> Result<Self>
    where
        Self: Sized;
}

/// Memory content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContent {
    /// Text content
    pub text: String,
    /// Embedding vector
    pub embedding: Option<Vec<f32>>,
    /// Image data (base64 encoded)
    pub image_data: Option<String>,
    /// Audio data (base64 encoded)
    pub audio_data: Option<String>,
    /// Video data (base64 encoded)
    pub video_data: Option<String>,
    /// Custom content fields
    pub custom: HashMap<String, Value>,
}

impl MemoryContent {
    /// Create new memory content
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            embedding: None,
            image_data: None,
            audio_data: None,
            video_data: None,
            custom: HashMap::new(),
        }
    }

    /// Set embedding vector
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set image data from bytes
    pub fn with_image(mut self, image_bytes: &[u8]) -> Self {
        self.image_data = Some(base64::engine::general_purpose::STANDARD.encode(image_bytes));
        self
    }

    /// Create memory content from JSON data
    pub fn json(json_value: Value) -> Self {
        Self {
            text: serde_json::to_string(&json_value).unwrap_or_default(),
            embedding: None,
            image_data: None,
            audio_data: None,
            video_data: None,
            custom: HashMap::new(),
        }
    }

    /// Create memory content from text
    pub fn text(text: &str) -> Self {
        Self::new(text)
    }

    /// Convert content to a generic entity representation
    pub fn to_entity(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("content_text".to_string(), self.text.clone().into());
        if let Some(embedding) = &self.embedding {
            map.insert(
                "content_embedding".to_string(),
                serde_json::to_value(embedding).unwrap_or_else(|e| {
                    log::warn!("Failed to serialize embedding: {}. Using null.", e);
                    Value::Null
                }),
            );
        }
        if let Some(image_data) = &self.image_data {
            map.insert("content_image_data".to_string(), image_data.clone().into());
        }
        // ... handle other data types ...
        map.insert(
            "custom".to_string(),
            Value::Object(self.custom.clone().into_iter().collect()),
        );
        map
    }

    /// Create content from a generic entity representation
    pub fn from_entity(attributes: &HashMap<String, Value>) -> Result<Self> {
        let text = attributes
            .get("content_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ConversionError("Missing content_text".to_string()))?
            .to_string();

        let embedding: Option<Vec<f32>> = attributes
            .get("content_embedding")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let image_data: Option<String> = attributes
            .get("content_image_data")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let audio_data: Option<String> = attributes
            .get("content_audio")
            .and_then(|v| v.as_str().map(String::from));

        let video_data: Option<String> = attributes
            .get("content_video")
            .and_then(|v| v.as_str().map(String::from));

        let custom = attributes
            .get("custom")
            .and_then(|v| v.as_object())
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        Ok(Self {
            text,
            embedding,
            image_data,
            audio_data,
            video_data,
            custom,
        })
    }
}

impl Default for MemoryContent {
    fn default() -> Self {
        Self::new("") // Use existing new() method
    }
}

/// Base memory struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMemory {
    /// Unique identifier
    pub id: String,
    /// Name of the memory
    pub name: String,
    /// Description of the memory
    pub description: String,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Metadata
    pub metadata: MemoryMetadata,
    /// Content
    pub content: MemoryContent,
}

impl BaseMemory {
    /// Create a new base memory
    pub fn new(
        id: &str,
        name: &str,
        description: &str,
        memory_type: MemoryTypeEnum,
        content: MemoryContent,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            updated_at: chrono::Utc::now(),
            metadata: MemoryMetadata::with_type(memory_type),
            content,
        }
    }
}

impl MemoryType for BaseMemory {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn metadata(&self) -> &MemoryMetadata {
        &self.metadata
    }

    fn content(&self) -> &MemoryContent {
        &self.content
    }

    fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(Error::InvalidInput("Memory ID cannot be empty".to_string()));
        }
        if self.name.is_empty() {
            return Err(Error::InvalidInput(
                "Memory name cannot be empty".to_string(),
            ));
        }
        if self.content.text.is_empty() {
            return Err(Error::InvalidInput(
                "Memory content text cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn to_entity(&self) -> BaseEntity {
        use crate::memory::graph::entity::BaseEntity;
        let mut entity = BaseEntity::new(
            self.id.clone(),
            format!("memory_{}", self.metadata.category),
        );

        /// Helper function to safely serialize values to JSON, with fallback handling
        fn serialize_field<T: Serialize>(value: &T, field_name: &str) -> serde_json::Value {
            match serde_json::to_value(value) {
                Ok(json_value) => json_value,
                Err(e) => {
                    log::warn!(
                        "Failed to serialize field '{}': {}. Using null.",
                        field_name,
                        e
                    );
                    serde_json::Value::Null
                }
            }
        }

        // Add basic fields as attributes
        entity = entity.with_attribute("name", json_to_surreal_value(self.name.clone().into()));
        entity = entity.with_attribute(
            "description",
            json_to_surreal_value(self.description.clone().into()),
        );
        entity = entity.with_attribute(
            "updated_at",
            json_to_surreal_value(serialize_field(&self.updated_at, "updated_at")),
        );

        // Add metadata fields as attributes
        entity = entity.with_attribute(
            "user_id",
            json_to_surreal_value(serialize_field(&self.metadata.user_id, "user_id")),
        );
        entity = entity.with_attribute(
            "agent_id",
            json_to_surreal_value(serialize_field(&self.metadata.agent_id, "agent_id")),
        );
        entity = entity.with_attribute(
            "context",
            json_to_surreal_value(self.metadata.context.clone().into()),
        );
        entity = entity.with_attribute(
            "keywords",
            json_to_surreal_value(serialize_field(&self.metadata.keywords, "keywords")),
        );
        entity = entity.with_attribute(
            "tags",
            json_to_surreal_value(serialize_field(&self.metadata.tags, "tags")),
        );
        entity = entity.with_attribute(
            "category",
            json_to_surreal_value(self.metadata.category.clone().into()),
        );
        entity = entity.with_attribute(
            "importance",
            json_to_surreal_value(self.metadata.importance.into()),
        );
        entity = entity.with_attribute(
            "source",
            json_to_surreal_value(serialize_field(&self.metadata.source, "source")),
        );
        entity = entity.with_attribute(
            "created_at",
            json_to_surreal_value(serialize_field(&self.metadata.created_at, "created_at")),
        );
        entity = entity.with_attribute(
            "last_accessed_at",
            json_to_surreal_value(serialize_field(
                &self.metadata.last_accessed_at,
                "last_accessed_at",
            )),
        );
        entity = entity.with_attribute(
            "embedding",
            json_to_surreal_value(serialize_field(&self.metadata.embedding, "embedding")),
        );
        entity = entity.with_attribute(
            "custom",
            json_to_surreal_value(self.metadata.custom.clone()),
        );

        // Add content as attributes
        let content_attrs = self.content.to_entity();
        for (key, value) in content_attrs {
            entity = entity.with_attribute(&key, json_to_surreal_value(value));
        }

        entity
    }

    fn from_entity(entity: BaseEntity) -> Result<Self>
    where
        Self: Sized,
    {
        use crate::memory::graph::entity::Entity;

        let id = entity.id().to_string();

        // Extract memory type from entity type
        let entity_type = entity.entity_type();
        let _memory_type = if let Some(stripped) = entity_type.strip_prefix("memory_") {
            MemoryTypeEnum::from_string(stripped)?
        } else {
            return Err(Error::ConversionError(format!(
                "Invalid entity type for memory: {}",
                entity_type
            )));
        };

        let get_attr = |key: &str, attributes: &HashMap<String, Value>| -> Result<Value> {
            attributes.get(key).cloned().ok_or_else(|| {
                Error::ConversionError(format!("Missing attribute '{}' in entity", key))
            })
        };

        let attributes: HashMap<String, Value> = entity
            .attributes()
            .iter()
            .map(|(k, v)| {
                // Convert surrealdb::Value to serde_json::Value via serialization
                let json_value = serde_json::to_value(v)
                    .unwrap_or_else(|_| serde_json::Value::String(format!("{:?}", v)));
                (k.clone(), json_value)
            })
            .collect();

        let name = get_attr("name", &attributes)?
            .as_str()
            .unwrap_or_default()
            .to_string();
        let description = get_attr("description", &attributes)?
            .as_str()
            .unwrap_or_default()
            .to_string();
        let updated_at = get_attr("updated_at", &attributes)?
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or_else(|| Error::ConversionError("Invalid format for updated_at".to_string()))?;

        // Create metadata manually from attributes
        let mut metadata = MemoryMetadata::new();

        if let Some(user_id) = get_attr("user_id", &attributes)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            metadata.user_id = Some(user_id);
        }
        if let Some(agent_id) = get_attr("agent_id", &attributes)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            metadata.agent_id = Some(agent_id);
        }
        if let Some(context) = get_attr("context", &attributes)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            metadata.context = context;
        }
        if let Some(category) = get_attr("category", &attributes)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            metadata.category = category;
        }
        if let Some(importance) = get_attr("importance", &attributes)
            .ok()
            .and_then(|v| v.as_f64())
        {
            metadata.importance = importance as f32;
        }
        if let Some(source) = get_attr("source", &attributes)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            metadata.source = Some(source);
        }
        if let Some(created_str) = get_attr("created_at", &attributes)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            && let Ok(parsed) = DateTime::parse_from_rfc3339(&created_str)
        {
            metadata.created_at = parsed.with_timezone(&Utc);
        }
        if let Some(keywords_val) = get_attr("keywords", &attributes).ok()
            && let Ok(keywords) = serde_json::from_value::<Vec<String>>(keywords_val)
        {
            metadata.keywords = keywords;
        }
        if let Some(tags_val) = get_attr("tags", &attributes).ok()
            && let Ok(tags) = serde_json::from_value::<Vec<String>>(tags_val)
        {
            metadata.tags = tags;
        }
        if let Ok(custom_val) = get_attr("custom", &attributes) {
            metadata.custom = custom_val;
        }

        let content = MemoryContent::from_entity(&attributes)?;

        Ok(Self {
            id,
            name,
            description,
            updated_at,
            metadata,
            content,
        })
    }
}

/// Types of relationships between memories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// Causal relationship (A causes B)
    CausedBy,
    /// Temporal relationship (A happens before B)
    PrecedesTemporally,
    /// Semantic similarity
    SimilarTo,
    /// Contradiction relationship
    Contradicts,
    /// Supporting evidence
    Supports,
    /// Part-whole relationship
    PartOf,
    /// Generalization relationship
    GeneralizationOf,
    /// Specialization relationship
    SpecializationOf,
    /// Association relationship
    AssociatedWith,
    /// Custom relationship type
    Custom(String),
}

impl RelationshipType {
    /// Check if this relationship type is bidirectional
    pub fn is_bidirectional(&self) -> bool {
        matches!(
            self,
            Self::SimilarTo | Self::Contradicts | Self::AssociatedWith
        )
    }

    /// Get the inverse relationship type
    pub fn inverse(&self) -> Option<Self> {
        match self {
            Self::CausedBy => Some(Self::Custom("causes".to_string())),
            Self::PrecedesTemporally => Some(Self::Custom("follows_temporally".to_string())),
            Self::PartOf => Some(Self::Custom("has_part".to_string())),
            Self::GeneralizationOf => Some(Self::SpecializationOf),
            Self::SpecializationOf => Some(Self::GeneralizationOf),
            Self::Supports => Some(Self::Custom("supported_by".to_string())),
            _ => None,
        }
    }
}

impl From<&str> for RelationshipType {
    fn from(s: &str) -> Self {
        match s {
            "caused_by" => Self::CausedBy,
            "precedes_temporally" => Self::PrecedesTemporally,
            "similar_to" => Self::SimilarTo,
            "contradicts" => Self::Contradicts,
            "supports" => Self::Supports,
            "part_of" => Self::PartOf,
            "generalization_of" => Self::GeneralizationOf,
            "specialization_of" => Self::SpecializationOf,
            "associated_with" => Self::AssociatedWith,
            _ => Self::Custom(s.to_string()),
        }
    }
}
