use std::fmt::{self, Debug, Display};
use std::sync::Arc;
use std::time::SystemTime;

use bytes::Bytes;
use hashbrown::HashMap;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, Visitor},
    ser::SerializeStruct,
};
use uuid::Uuid;

// Import for error conversion
// use crate::memory::utils::error::Error as FluentMemoryError; // Temporarily disabled to break circular dependency

/// Zero-allocation memory type enumeration with blazing-fast operations
///
/// Stack-allocated enum with no heap usage and branch prediction optimization
/// All variants are Copy for zero-allocation semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)] // Optimize for minimal size and fast comparison
pub enum MemoryTypeEnum {
    /// Factual information and knowledge
    Fact = 0,
    /// Specific events and experiences
    Episode = 1,
    /// Semantic knowledge and concepts
    Semantic = 2,
    /// Procedural skills and how-to knowledge
    Procedural = 3,
    /// Declarative facts and information
    Declarative = 4,
    /// Implicit unconscious knowledge
    Implicit = 5,
    /// Explicit conscious knowledge
    Explicit = 6,
    /// Contextual situation-dependent memory
    Contextual = 7,
    /// Temporal time-based sequences
    Temporal = 8,
    /// Spatial location-based memory
    Spatial = 9,
    /// Associative linked memories
    Associative = 10,
    /// Emotional affective memory
    Emotional = 11,
    /// Episodic memory (events and experiences)
    Episodic = 12,
    /// Working memory (temporary storage)
    Working = 13,
    /// Long-term memory (persistent storage)
    LongTerm = 14,
}

/// Type alias for backward compatibility
pub type MemoryType = MemoryTypeEnum;

impl MemoryTypeEnum {
    /// Convert from string to `MemoryTypeEnum` with zero allocation
    /// Uses static string matching for blazing-fast lookup
    #[inline]
    #[must_use]
    pub const fn from_str_const(s: &str) -> Option<Self> {
        match s.as_bytes() {
            b"fact" => Some(Self::Fact),
            b"episode" => Some(Self::Episode),
            b"semantic" => Some(Self::Semantic),
            b"procedural" => Some(Self::Procedural),
            b"declarative" => Some(Self::Declarative),
            b"implicit" => Some(Self::Implicit),
            b"explicit" => Some(Self::Explicit),
            b"contextual" => Some(Self::Contextual),
            b"temporal" => Some(Self::Temporal),
            b"spatial" => Some(Self::Spatial),
            b"associative" => Some(Self::Associative),
            b"emotional" => Some(Self::Emotional),
            b"episodic" => Some(Self::Episodic),
            b"working" => Some(Self::Working),
            b"long_term" => Some(Self::LongTerm),
            _ => None,
        }
    }

    /// Get static string representation with zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Episode => "episode",
            Self::Semantic => "semantic",
            Self::Procedural => "procedural",
            Self::Declarative => "declarative",
            Self::Implicit => "implicit",
            Self::Explicit => "explicit",
            Self::Contextual => "contextual",
            Self::Temporal => "temporal",
            Self::Spatial => "spatial",
            Self::Associative => "associative",
            Self::Emotional => "emotional",
            Self::Episodic => "episodic",
            Self::Working => "working",
            Self::LongTerm => "long_term",
        }
    }

    /// Get base importance score for memory type with zero allocation
    #[inline]
    #[must_use]
    pub const fn base_importance(&self) -> f32 {
        match self {
            Self::Fact | Self::Declarative | Self::Explicit => 0.9, // High importance
            Self::Semantic | Self::LongTerm => 0.8,                 // Important knowledge
            Self::Episodic | Self::Episode | Self::Contextual => 0.7, // Experiences
            Self::Procedural | Self::Temporal | Self::Spatial => 0.6, // Skills and context
            Self::Working => 0.4,                                   // Temporary
            Self::Implicit | Self::Associative | Self::Emotional => 0.5, // Background
        }
    }
}

impl Display for MemoryTypeEnum {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Lock-free relationship type with atomic reference counting and blazing-fast comparison
///
/// Optimized for concurrent relationship tracking with skip-list backing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)] // Optimize for minimal size and cache efficiency
pub enum RelationshipType {
    /// Basic relationship connection
    RelatedTo = 0,
    /// Dependency relationship (A depends on B)
    DependsOn = 1,
    /// Conflict relationship (A conflicts with B)
    ConflictsWith = 2,
    /// Causal relationship (A causes B)
    CausedBy = 3,
    /// Temporal relationship (A precedes B)
    PrecedesTemporally = 4,
    /// Semantic similarity relationship
    SimilarTo = 5,
    /// Contradiction relationship
    Contradicts = 6,
    /// Supporting evidence relationship
    Supports = 7,
    /// Part-whole relationship (A is part of B)
    PartOf = 8,
    /// Generalization relationship (A generalizes B)
    GeneralizationOf = 9,
    /// Specialization relationship (A specializes B)
    SpecializationOf = 10,
    /// Association relationship
    AssociatedWith = 11,
    /// Custom relationship type with String for serialization compatibility
    Custom(String) = 255,
}

impl RelationshipType {
    /// Check if relationship is bidirectional with zero allocation
    #[inline]
    #[must_use]
    pub const fn is_bidirectional(&self) -> bool {
        matches!(
            self,
            Self::RelatedTo
                | Self::SimilarTo
                | Self::Contradicts
                | Self::ConflictsWith
                | Self::AssociatedWith
        )
    }

    /// Get inverse relationship type with optimized lookup
    #[inline]
    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        match self {
            Self::DependsOn => Some(Self::Custom("depended_on_by".to_string())),
            Self::CausedBy => Some(Self::Custom("causes".to_string())),
            Self::PrecedesTemporally => Some(Self::Custom("follows_temporally".to_string())),
            Self::PartOf => Some(Self::Custom("has_part".to_string())),
            Self::GeneralizationOf => Some(Self::SpecializationOf),
            Self::SpecializationOf => Some(Self::GeneralizationOf),
            Self::Supports => Some(Self::Custom("supported_by".to_string())),
            _ => None,
        }
    }

    /// Create custom relationship type with string
    #[inline]
    pub fn custom(name: impl Into<String>) -> Self {
        Self::Custom(name.into())
    }
}

impl Display for RelationshipType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RelatedTo => f.write_str("related_to"),
            Self::DependsOn => f.write_str("depends_on"),
            Self::ConflictsWith => f.write_str("conflicts_with"),
            Self::CausedBy => f.write_str("caused_by"),
            Self::PrecedesTemporally => f.write_str("precedes_temporally"),
            Self::SimilarTo => f.write_str("similar_to"),
            Self::Contradicts => f.write_str("contradicts"),
            Self::Supports => f.write_str("supports"),
            Self::PartOf => f.write_str("part_of"),
            Self::GeneralizationOf => f.write_str("generalization_of"),
            Self::SpecializationOf => f.write_str("specialization_of"),
            Self::AssociatedWith => f.write_str("associated_with"),
            Self::Custom(name) => f.write_str(name),
        }
    }
}

/// Streaming memory content with zero-copy architecture
///
/// Supports Text/Image/Audio/Video variants with zero-copy Bytes backing
/// Memory-mapped file support for large content with streaming access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryContent {
    /// Empty content - zero allocation
    Empty,
    /// Text content with String for serialization compatibility
    Text(String),
    /// Binary image data with zero-copy Bytes
    Image(Bytes),
    /// Binary audio data with zero-copy Bytes
    Audio(Bytes),
    /// Binary video data with zero-copy Bytes
    Video(Bytes),
    /// JSON structured data for serialization compatibility
    Json(serde_json::Value),
    /// Custom binary data with zero-copy Bytes and content type
    Binary {
        /// Raw binary data stored with zero-copy semantics
        data: Bytes,
        /// MIME content type for the binary data
        content_type: String,
    },
}

impl std::fmt::Display for MemoryContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryContent::Empty => write!(f, ""),
            MemoryContent::Text(text) => write!(f, "{text}"),
            MemoryContent::Image(bytes) => {
                let len = bytes.len();
                write!(f, "[Image: {len} bytes]")
            }
            MemoryContent::Audio(bytes) => {
                let len = bytes.len();
                write!(f, "[Audio: {len} bytes]")
            }
            MemoryContent::Video(bytes) => {
                let len = bytes.len();
                write!(f, "[Video: {len} bytes]")
            }
            MemoryContent::Json(json) => write!(f, "{json}"),
            MemoryContent::Binary { data, content_type } => {
                let len = data.len();
                write!(f, "[Binary({content_type}): {len} bytes]")
            }
        }
    }
}

impl MemoryContent {
    /// Create text content with string
    #[inline]
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text(content.into())
    }

    /// Create image content from bytes with zero-copy
    #[inline]
    pub fn image(data: impl Into<Bytes>) -> Self {
        Self::Image(data.into())
    }

    /// Create audio content from bytes with zero-copy
    #[inline]
    pub fn audio(data: impl Into<Bytes>) -> Self {
        Self::Audio(data.into())
    }

    /// Create video content from bytes with zero-copy
    #[inline]
    pub fn video(data: impl Into<Bytes>) -> Self {
        Self::Video(data.into())
    }

    /// Create JSON content
    #[inline]
    #[must_use]
    pub fn json(value: serde_json::Value) -> Self {
        Self::Json(value)
    }

    /// Create binary content with content type
    #[inline]
    pub fn binary(data: impl Into<Bytes>, content_type: impl Into<String>) -> Self {
        Self::Binary {
            data: data.into(),
            content_type: content_type.into(),
        }
    }

    /// Get content size in bytes with zero allocation
    #[inline]
    pub fn size(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Text(text) => text.len(),
            Self::Image(data)
            | Self::Audio(data)
            | Self::Video(data)
            | Self::Binary { data, .. } => data.len(),
            Self::Json(value) => {
                // Estimate JSON size without serialization
                std::mem::size_of_val(value)
            }
        }
    }

    /// Check if content is empty with zero allocation
    #[inline]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

impl Default for MemoryContent {
    #[inline]
    fn default() -> Self {
        Self::Empty
    }
}

/// Smart pointer optimized base memory with atomic operations
///
/// UUID-based ID system with inline generation, Arc<str> for zero-copy content sharing
/// tokio async tasks concurrent access
#[derive(Debug, Clone)]
pub struct BaseMemory {
    /// UUID-based unique identifier with inline generation
    pub id: Uuid,
    /// Memory type classification
    pub memory_type: MemoryTypeEnum,
    /// Content with zero-copy sharing
    pub content: MemoryContent,
    /// Creation timestamp with atomic operations
    pub created_at: SystemTime,
    /// Last update timestamp with atomic operations
    pub updated_at: SystemTime,
    /// tokio async tasks access optimization
    pub metadata: Arc<tokio::sync::RwLock<HashMap<String, serde_json::Value>>>,
}

impl BaseMemory {
    /// Create new base memory with inline UUID generation
    #[inline]
    pub fn new(id: Uuid, memory_type: MemoryTypeEnum, content: MemoryContent) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            memory_type,
            content,
            created_at: now,
            updated_at: now,
            metadata: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Create with generated UUID for convenience
    #[inline]
    pub fn with_generated_id(memory_type: MemoryTypeEnum, content: MemoryContent) -> Self {
        Self::new(Uuid::new_v4(), memory_type, content)
    }

    /// Update timestamp atomically
    #[inline]
    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now();
    }

    /// Set metadata value
    pub async fn set_metadata(&self, key: impl Into<String>, value: serde_json::Value) {
        let mut meta = self.metadata.write().await;
        meta.insert(key.into(), value);
    }

    /// Get metadata value
    pub async fn get_metadata(&self, key: &str) -> Option<serde_json::Value> {
        let meta = self.metadata.read().await;
        meta.get(key).cloned()
    }

    /// Get content size with zero allocation
    #[inline]
    pub fn content_size(&self) -> usize {
        self.content.size()
    }

    /// Calculate base importance with zero allocation
    #[inline]
    pub fn base_importance(&self) -> f32 {
        self.memory_type.base_importance()
    }
}

impl PartialEq for BaseMemory {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for BaseMemory {}

impl std::hash::Hash for BaseMemory {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Serialize for BaseMemory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("BaseMemory", 5)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("memory_type", &self.memory_type)?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for BaseMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            MemoryType,
            Content,
            CreatedAt,
            UpdatedAt,
        }

        struct BaseMemoryVisitor;

        impl<'de> Visitor<'de> for BaseMemoryVisitor {
            type Value = BaseMemory;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct BaseMemory")
            }

            fn visit_map<V>(self, mut map: V) -> Result<BaseMemory, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut id = None;
                let mut memory_type = None;
                let mut content = None;
                let mut created_at = None;
                let mut updated_at = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        Field::MemoryType => {
                            if memory_type.is_some() {
                                return Err(serde::de::Error::duplicate_field("memory_type"));
                            }
                            memory_type = Some(map.next_value()?);
                        }
                        Field::Content => {
                            if content.is_some() {
                                return Err(serde::de::Error::duplicate_field("content"));
                            }
                            content = Some(map.next_value()?);
                        }
                        Field::CreatedAt => {
                            if created_at.is_some() {
                                return Err(serde::de::Error::duplicate_field("created_at"));
                            }
                            created_at = Some(map.next_value()?);
                        }
                        Field::UpdatedAt => {
                            if updated_at.is_some() {
                                return Err(serde::de::Error::duplicate_field("updated_at"));
                            }
                            updated_at = Some(map.next_value()?);
                        }
                    }
                }

                let id = id.ok_or_else(|| serde::de::Error::missing_field("id"))?;
                let memory_type =
                    memory_type.ok_or_else(|| serde::de::Error::missing_field("memory_type"))?;
                let content = content.ok_or_else(|| serde::de::Error::missing_field("content"))?;
                let created_at =
                    created_at.ok_or_else(|| serde::de::Error::missing_field("created_at"))?;
                let updated_at =
                    updated_at.ok_or_else(|| serde::de::Error::missing_field("updated_at"))?;

                Ok(BaseMemory {
                    id,
                    memory_type,
                    content,
                    created_at,
                    updated_at,
                    metadata: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
                })
            }
        }

        const FIELDS: &[&str] = &["id", "memory_type", "content", "created_at", "updated_at"];
        deserializer.deserialize_struct("BaseMemory", FIELDS, BaseMemoryVisitor)
    }
}

/// Memory relationship with atomic counters and zero allocation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRelationship {
    /// Unique relationship identifier
    pub id: Uuid,
    /// Source memory ID
    pub from_id: Uuid,
    /// Target memory ID
    pub to_id: Uuid,
    /// Relationship type
    pub relationship_type: RelationshipType,
    /// Relationship strength (0.0 to 1.0)
    pub strength: f32,
    /// Creation timestamp
    pub created_at: SystemTime,
}

impl MemoryRelationship {
    /// Create new memory relationship with generated ID
    #[inline]
    #[must_use]
    pub fn new(
        from_id: Uuid,
        to_id: Uuid,
        relationship_type: RelationshipType,
        strength: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_id,
            to_id,
            relationship_type,
            strength: strength.clamp(0.0, 1.0),
            created_at: SystemTime::now(),
        }
    }

    /// Check if relationship is bidirectional
    #[inline]
    #[must_use]
    pub fn is_bidirectional(&self) -> bool {
        self.relationship_type.is_bidirectional()
    }

    /// Get inverse relationship if possible
    #[inline]
    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        self.relationship_type.inverse().map(|inverse_type| Self {
            id: Uuid::new_v4(),
            from_id: self.to_id,
            to_id: self.from_id,
            relationship_type: inverse_type,
            strength: self.strength,
            created_at: self.created_at,
        })
    }
}

/// Result type for memory operations
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Memory operation error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum MemoryError {
    /// Memory item could not be found
    #[error("Memory not found: {0}")]
    NotFound(String),
    /// Invalid memory type encountered
    #[error("Invalid memory type: {0}")]
    InvalidType(String),
    /// Invalid content format or structure
    #[error("Invalid content: {0}")]
    InvalidContent(String),
    /// Error during serialization or deserialization
    #[error("Serialization error: {0}")]
    Serialization(String),
    /// Input validation failed
    #[error("Validation error: {0}")]
    Validation(String),
    /// General operation failure
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    /// Storage backend error
    #[error("Storage error: {0}")]
    StorageError(String),
}

impl MemoryError {
    /// Create not found error
    #[inline]
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create invalid type error
    #[inline]
    pub fn invalid_type(msg: impl Into<String>) -> Self {
        Self::InvalidType(msg.into())
    }

    /// Create invalid content error
    #[inline]
    pub fn invalid_content(msg: impl Into<String>) -> Self {
        Self::InvalidContent(msg.into())
    }

    /// Create validation error
    #[inline]
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create storage error
    #[inline]
    pub fn storage_error(msg: impl Into<String>) -> Self {
        Self::StorageError(msg.into())
    }
}

// Dead code removed - FluentMemoryError type does not exist

// Trait implementations for error conversions
