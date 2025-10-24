// src/memory/episodic.rs
//! Episodic memory implementation for the memory system.
//!
//! Episodic memory stores sequences of events with temporal information,
//! allowing for time-based queries and context-aware retrieval.

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use chrono::{DateTime, Utc};
use crossbeam_skiplist::SkipMap;
use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::pin::Pin;
use surrealdb::Value;
use tokio::sync::RwLock;
use tokio_stream::Stream;

use crate::memory::graph::entity::{BaseEntity, Entity};
use crate::memory::primitives::metadata::MemoryMetadata;
use crate::memory::primitives::node::MemoryNode;
use crate::memory::primitives::types::{BaseMemory, MemoryContent, MemoryType, MemoryTypeEnum};
use crate::memory::repository::MemoryRepository;
use crate::memory::utils::Result;
use crate::memory::utils::error::Error;

/// Wrapper for Result<EpisodicMemory> that implements MessageChunk
#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodicMemoryChunk {
    pub success: bool,
    pub error_message: Option<String>,
}

impl EpisodicMemoryChunk {
    pub fn new(result: Result<EpisodicMemory>) -> Self {
        match result {
            Ok(_) => Self {
                success: true,
                error_message: None,
            },
            Err(e) => Self {
                success: false,
                error_message: Some(format!("{:?}", e)),
            },
        }
    }
}

impl MessageChunk for EpisodicMemoryChunk {
    fn bad_chunk(error: String) -> Self {
        Self {
            success: false,
            error_message: Some(error),
        }
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

impl Default for EpisodicMemoryChunk {
    fn default() -> Self {
        Self::bad_chunk("Default EpisodicMemoryChunk".to_string())
    }
}

/// Context for an episodic memory event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicContext {
    /// Unique identifier for the context
    pub id: String,

    /// Type of context (e.g., "location", "person", "object")
    pub context_type: String,

    /// Value of the context
    pub value: String,

    /// Additional metadata for the context
    pub metadata: HashMap<String, Value>,
}

impl EpisodicContext {
    /// Create a new episodic context
    pub fn new(id: &str, context_type: &str, value: &str) -> Self {
        Self {
            id: id.to_string(),
            context_type: context_type.to_string(),
            value: value.to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the context
    #[must_use]
    pub fn with_metadata(mut self, key: &str, value: Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    /// Convert the context to a SurrealDB value
    pub fn to_value(&self) -> Result<surrealdb::Value> {
        let json_obj = json!({
            "id": self.id,
            "context_type": self.context_type,
            "value": self.value,
            "metadata": self.metadata
        });
        Ok(surrealdb::value::to_value(json_obj).unwrap_or_default())
    }
}

/// Represents a single event in episodic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicEvent {
    /// Unique identifier for the event
    pub id: String,

    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,

    /// Content of the event
    pub content: MemoryContent,

    /// Content hash for fast deduplication and lookup
    pub content_hash: i64,

    /// Context associated with the event
    pub context: Vec<EpisodicContext>,

    /// Additional metadata for the event
    pub metadata: HashMap<String, Value>,
}

impl EpisodicEvent {
    /// Create a new episodic event
    pub fn new(id: &str, content: MemoryContent) -> Self {
        // Calculate content hash for fast deduplication
        let content_hash = crate::domain::memory::serialization::content_hash(&content.text);

        Self {
            id: id.to_string(),
            timestamp: Utc::now(),
            content,
            content_hash,
            context: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a context to the event
    #[must_use]
    pub fn with_context(mut self, context: EpisodicContext) -> Self {
        self.context.push(context);
        self
    }

    /// Add metadata to the event
    #[must_use]
    pub fn with_metadata(mut self, key: &str, value: Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    /// Create a memory record from this event for efficient serialization
    pub fn to_memory_record(
        &self,
        output_text: &str,
    ) -> crate::domain::memory::serialization::MemoryRecord {
        crate::domain::memory::serialization::MemoryRecord::new(
            &self.content.text,
            output_text,
            self.timestamp.timestamp() as u64,
        )
    }

    /// Serialize event to binary format using zero-allocation buffer
    pub fn serialize_to_buffer(&self, output_text: &str) -> Vec<u8> {
        crate::domain::memory::serialization::with_serialization_buffer(|buffer| {
            let record = self.to_memory_record(output_text);
            record.serialize_to_buffer(buffer);
            buffer.as_slice().to_vec()
        })
    }
}

/// Represents an episodic memory, which is a collection of events
#[derive(Debug, Clone)]
pub struct EpisodicMemory {
    /// Base memory properties
    pub base: BaseMemory,

    /// Collection of events, indexed by timestamp for fast temporal queries
    pub events: Arc<ArcSwap<SkipMap<DateTime<Utc>, EpisodicEvent>>>,
}

impl MemoryType for EpisodicMemory {
    fn id(&self) -> &str {
        &self.base.id
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn description(&self) -> &str {
        &self.base.description
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.base.updated_at
    }

    fn metadata(&self) -> &MemoryMetadata {
        &self.base.metadata
    }

    fn content(&self) -> &MemoryContent {
        &self.base.content
    }

    fn validate(&self) -> Result<()> {
        if self.base.id.is_empty() {
            return Err(Error::InvalidInput("ID cannot be empty".to_string()));
        }
        if self.base.name.is_empty() {
            return Err(Error::InvalidInput("Name cannot be empty".to_string()));
        }
        Ok(())
    }

    fn to_entity(&self) -> BaseEntity {
        BaseEntity::new(self.base.id.clone(), "episodic_memory".to_string())
            .with_attribute(
                "name",
                surrealdb::value::to_value(self.base.name.clone()).unwrap_or_default(),
            )
            .with_attribute(
                "description",
                surrealdb::value::to_value(self.base.description.clone()).unwrap_or_default(),
            )
    }

    fn from_entity(entity: BaseEntity) -> Result<Self> {
        use surrealdb::value::from_value;

        let name = entity
            .get_attribute("name")
            .and_then(|v| from_value::<String>(v.clone()).ok())
            .unwrap_or_else(|| "Unnamed".to_string());

        let description = entity
            .get_attribute("description")
            .and_then(|v| from_value::<String>(v.clone()).ok())
            .unwrap_or_default();

        Ok(Self::new(entity.id(), &name, &description))
    }
}

impl EpisodicMemory {
    /// Create a new episodic memory
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        let mut metadata = MemoryMetadata::with_type(MemoryTypeEnum::Episodic);
        metadata.custom = json!({"version": "1.0"});

        Self {
            base: BaseMemory {
                id: id.to_string(),
                name: name.to_string(),
                description: description.to_string(),
                updated_at: Utc::now(),
                metadata,
                content: MemoryContent::new(""),
            },
            events: Arc::new(ArcSwap::new(Arc::new(SkipMap::new()))),
        }
    }

    /// Create from a BaseMemory
    pub fn from_memory(memory: &BaseMemory) -> Result<Self> {
        // Deserialize events from the memory content text field
        let events_map: HashMap<DateTime<Utc>, EpisodicEvent> = if !memory.content.text.is_empty() {
            serde_json::from_str(&memory.content.text)?
        } else if let Some(custom_data) = memory.content.custom.get("events") {
            serde_json::from_value(custom_data.clone())?
        } else {
            return Err(Error::InvalidInput(
                "No episodic events data found in memory content".to_string(),
            ));
        };

        // Convert HashMap to SkipMap
        let events = SkipMap::new();
        for (timestamp, event) in events_map {
            events.insert(timestamp, event);
        }

        Ok(Self {
            base: memory.clone(),
            events: Arc::new(ArcSwap::new(Arc::new(events))),
        })
    }

    /// Convert to BaseMemory
    pub fn to_memory(&self) -> Result<BaseMemory> {
        let mut memory = self.base.clone();
        let events_guard = self.events.load();
        let events_map: HashMap<_, _> = events_guard
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect();
        memory.content = MemoryContent::new(&serde_json::to_string(&events_map)?);
        Ok(memory)
    }

    /// Get the memory type
    pub fn memory_type(&self) -> MemoryTypeEnum {
        MemoryTypeEnum::Episodic
    }

    /// Add an event to the episodic memory
    pub fn add_event(&self, event: EpisodicEvent) {
        let new_events = self.events.load().clone();
        new_events.insert(event.timestamp, event);
        self.events.store(new_events);
        // Note: Would update timestamp here but base is immutable in this context
    }

    /// Retrieve events within a specific time range
    pub fn get_events_in_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Vec<EpisodicEvent> {
        self.events
            .load()
            .range(start_time..=end_time)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Find the last N events before a given time
    pub fn get_last_n_events(&self, n: usize, before_time: DateTime<Utc>) -> Vec<EpisodicEvent> {
        self.events
            .load()
            .range(..=before_time)
            .rev()
            .take(n)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Create a new episodic memory and store it in the repository
    pub fn create(
        memory_repo: Arc<RwLock<MemoryRepository>>,
        id: &str,
        name: &str,
        description: &str,
    ) -> Pin<Box<dyn Stream<Item = EpisodicMemoryChunk> + Send>> {
        // Convert to owned strings BEFORE moving into closure
        let id_owned = id.to_string();
        let name_owned = name.to_string();
        let description_owned = description.to_string();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let id_string = id_owned;
            let name_string = name_owned;
            let description_string = description_owned;

            tokio::spawn(async move {
                let result = {
                    let episodic =
                        EpisodicMemory::new(&id_string, &name_string, &description_string);

                    // Convert to MemoryNode for storage
                    let mut metadata = MemoryMetadata::new();
                    metadata.created_at = episodic.base.metadata.created_at;

                    let content = match serde_json::to_string(&episodic.base.content) {
                        Ok(content_str) => MemoryContent::text(&content_str),
                        Err(_) => {
                            let _ = tx.send(EpisodicMemoryChunk::new(Err(
                                crate::memory::utils::error::Error::SerializationError(
                                    "Failed to serialize episodic memory content".to_string(),
                                ),
                            )));
                            return;
                        }
                    };

                    // Calculate content hash for deduplication
                    let content_hash =
                        crate::domain::memory::serialization::content_hash(&content.text);

                    let memory_node = MemoryNode {
                        id: episodic.base.id.clone(),
                        content,
                        content_hash,
                        memory_type: MemoryTypeEnum::Episodic,
                        created_at: episodic.base.metadata.created_at,
                        updated_at: episodic.base.updated_at,
                        embedding: None,
                        evaluation_status:
                            crate::memory::monitoring::operations::OperationStatus::Pending,
                        metadata,
                        relevance_score: None,
                    };

                    // Persist memory to repository using write lock
                    let created_memory = match memory_repo
                        .write()
                        .await
                        .create(&episodic.base.id, &memory_node)
                    {
                        Ok(memory) => memory,
                        Err(e) => {
                            let _ = tx.send(EpisodicMemoryChunk::new(Err(e)));
                            return;
                        }
                    };
                    {
                        // Convert created MemoryNode to BaseMemory
                        let mut metadata = MemoryMetadata::with_type(MemoryTypeEnum::Episodic);
                        metadata.created_at = created_memory.created_at;
                        // MemoryMetadata doesn't have updated_at field - that's on BaseMemory

                        let base_memory = BaseMemory {
                            id: created_memory.id.clone(),
                            name: name_string.clone(),
                            description: description_string.clone(),
                            updated_at: created_memory.updated_at,
                            metadata,
                            content: created_memory.content.clone(),
                        };
                        EpisodicMemory::from_memory(&base_memory)
                    }
                };
                let _ = tx.send(EpisodicMemoryChunk::new(result));
            });
        }))
    }
}
