//! SurrealDB memory manager implementation.
//!
//! This module provides the core SurrealDBMemoryManager struct with
//! initialization, database utilities, migrations, and export/import functionality.

use std::sync::Arc;

use surrealdb::Surreal;
use surrealdb::engine::any::Any;

use crate::capability::registry::TextEmbeddingModel;
use crate::memory::migration::{
    BuiltinMigrations, DataExporter, DataImporter, ExportFormat, ImportFormat, MigrationManager,
};
use crate::memory::primitives::{MemoryNode, MemoryRelationship};
use crate::memory::schema::memory_schema::MemoryNodeSchema;
use crate::memory::utils::error::Error;
use std::path::Path;

use super::Result;
use super::types::ExportData;

/// SurrealDB-backed memory manager implementation
#[derive(Debug)]
pub struct SurrealDBMemoryManager {
    pub(in crate::memory::core) db: Surreal<Any>,
    pub(super) embedding_model: Option<TextEmbeddingModel>,
}

impl SurrealDBMemoryManager {
    /// Create a new SurrealDBMemoryManager with an existing database connection
    pub fn new(db: Surreal<Any>) -> Self {
        Self {
            db,
            embedding_model: None,
        }
    }

    /// Create a new manager with an embedding model for auto-embedding generation
    pub fn with_embedding_model(db: Surreal<Any>, embedding_model: TextEmbeddingModel) -> Self {
        Self {
            db,
            embedding_model: Some(embedding_model),
        }
    }

    /// Alternative constructor using Arc<TextEmbeddingModel>
    pub fn with_embeddings(db: Surreal<Any>, embedding_model: Arc<TextEmbeddingModel>) -> Self {
        Self {
            db,
            embedding_model: Some((*embedding_model).clone()),
        }
    }

    /// Get a reference to the underlying database connection
    pub fn database(&self) -> &Surreal<Any> {
        &self.db
    }

    /// Initialize the database schema and indexes
    ///
    /// This method sets up:
    /// - Memory table with content and metadata fields
    /// - Relationship table with source/target references
    /// - Quantum signature table for cognitive states
    /// - MTREE index for vector similarity search
    /// - Entanglement graph edges
    pub async fn initialize(&self) -> Result<()> {
        // Define the memory table schema
        self.db
            .query(
                "
                DEFINE TABLE IF NOT EXISTS memory SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS content ON memory TYPE string;
                DEFINE FIELD IF NOT EXISTS content_hash ON memory TYPE int;
                DEFINE FIELD IF NOT EXISTS memory_type ON memory TYPE string;
                DEFINE FIELD IF NOT EXISTS created_at ON memory TYPE datetime;
                DEFINE FIELD IF NOT EXISTS updated_at ON memory TYPE datetime;
                DEFINE FIELD IF NOT EXISTS metadata ON memory FLEXIBLE TYPE object;
                DEFINE FIELD IF NOT EXISTS metadata.created_at ON memory TYPE int;
                DEFINE FIELD IF NOT EXISTS metadata.last_accessed_at ON memory TYPE int;
                DEFINE FIELD IF NOT EXISTS metadata.importance ON memory TYPE float;
                DEFINE FIELD IF NOT EXISTS metadata.embedding ON memory FLEXIBLE TYPE option<array<float>>;
                DEFINE FIELD IF NOT EXISTS metadata.custom ON memory FLEXIBLE TYPE option<object>;
                ",
            )
            .await
            .map_err(|e| Error::Database(format!("Failed to define memory table: {:?}", e)))?;

        // Define the relationship table
        self.db
            .query(
                "
                DEFINE TABLE IF NOT EXISTS relationship SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS source_id ON relationship TYPE string;
                DEFINE FIELD IF NOT EXISTS target_id ON relationship TYPE string;
                DEFINE FIELD IF NOT EXISTS relationship_type ON relationship TYPE string;
                DEFINE FIELD IF NOT EXISTS created_at ON relationship TYPE int;
                DEFINE FIELD IF NOT EXISTS updated_at ON relationship TYPE int;
                DEFINE FIELD IF NOT EXISTS strength ON relationship TYPE option<float>;
                DEFINE FIELD IF NOT EXISTS metadata ON relationship FLEXIBLE TYPE option<object>;
                ",
            )
            .await
            .map_err(|e| {
                Error::Database(format!("Failed to define relationship table: {:?}", e))
            })?;

        // Define quantum signature table
        self.db
            .query(
                "
                DEFINE TABLE IF NOT EXISTS quantum_signature SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS memory_id ON quantum_signature TYPE string;
                DEFINE FIELD IF NOT EXISTS coherence ON quantum_signature TYPE float;
                DEFINE FIELD IF NOT EXISTS phase ON quantum_signature TYPE float;
                DEFINE FIELD IF NOT EXISTS amplitude ON quantum_signature TYPE float;
                DEFINE FIELD IF NOT EXISTS entanglement_count ON quantum_signature TYPE int;
                DEFINE FIELD IF NOT EXISTS last_measured_at ON quantum_signature TYPE int;
                ",
            )
            .await
            .map_err(|e| {
                Error::Database(format!("Failed to define quantum_signature table: {:?}", e))
            })?;

        // Define MTREE index for vector similarity search
        self.db
            .query(
                "
                DEFINE INDEX IF NOT EXISTS memory_embedding_mtree ON memory 
                FIELDS metadata.embedding 
                MTREE DIMENSION 1024 
                DIST COSINE 
                TYPE F32;
                ",
            )
            .await
            .map_err(|e| Error::Database(format!("Failed to define MTREE index: {:?}", e)))?;

        // Define entanglement edges (graph relations)
        self.db
            .query(
                "
                DEFINE TABLE IF NOT EXISTS entangled SCHEMAFULL TYPE RELATION FROM memory TO memory;
                DEFINE FIELD IF NOT EXISTS entanglement_type ON entangled TYPE string;
                DEFINE FIELD IF NOT EXISTS strength ON entangled TYPE float;
                DEFINE FIELD IF NOT EXISTS created_at ON entangled TYPE int;
                ",
            )
            .await
            .map_err(|e| {
                Error::Database(format!("Failed to define entanglement edges: {:?}", e))
            })?;

        // Define causal relation edges (separate from general entanglement)
        self.db
            .query(
                "
                DEFINE TABLE IF NOT EXISTS caused SCHEMAFULL TYPE RELATION FROM memory TO memory;
                DEFINE FIELD IF NOT EXISTS strength ON caused TYPE float;
                DEFINE FIELD IF NOT EXISTS temporal_distance ON caused TYPE int;
                DEFINE FIELD IF NOT EXISTS created_at ON caused TYPE int;
                ",
            )
            .await
            .map_err(|e| Error::Database(format!("Failed to define causal edges: {:?}", e)))?;

        Ok(())
    }

    /// Execute a raw SurrealQL query
    ///
    /// Useful for custom queries and administrative operations.
    pub async fn execute_query(&self, query: &str) -> Result<serde_json::Value> {
        let mut response = self
            .db
            .query(query)
            .await
            .map_err(|e| Error::Database(format!("{:?}", e)))?;

        let result: Vec<serde_json::Value> = response
            .take(0)
            .map_err(|e| Error::Database(format!("{:?}", e)))?;

        Ok(serde_json::Value::Array(result))
    }

    /// Health check for database connection
    pub async fn health_check(&self) -> Result<bool> {
        match self.db.health().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Run all pending migrations
    pub async fn run_migrations(&self) -> Result<()> {
        let db_arc = Arc::new(self.db.clone());
        let mut migration_mgr = MigrationManager::new(db_arc)
            .await
            .map_err(|e| Error::Database(format!("Migration manager creation failed: {:?}", e)))?;

        // Add all built-in migrations to the manager
        for migration in BuiltinMigrations::all() {
            migration_mgr.add_migration(migration);
        }

        // Execute all pending migrations
        migration_mgr
            .migrate()
            .await
            .map_err(|e| Error::Database(format!("Migration failed: {:?}", e)))?;

        Ok(())
    }

    /// Export all memories and relationships to a file
    pub async fn export_memories(&self, path: &Path, format: ExportFormat) -> Result<()> {
        // Fetch all memories
        let query = "SELECT * FROM memory";
        let mut response = self
            .db
            .query(query)
            .await
            .map_err(|e| Error::Database(format!("Export query failed: {:?}", e)))?;

        let memory_schemas: Vec<MemoryNodeSchema> = response
            .take(0)
            .map_err(|e| Error::Database(format!("Failed to parse memories: {:?}", e)))?;

        let memories: Vec<MemoryNode> = memory_schemas.into_iter().map(Self::from_schema).collect();

        // Fetch all relationships
        let query = "SELECT * FROM relationship";
        let mut response = self
            .db
            .query(query)
            .await
            .map_err(|e| Error::Database(format!("Export query failed: {:?}", e)))?;

        let relationships: Vec<MemoryRelationship> = response
            .take(0)
            .map_err(|e| Error::Database(format!("Failed to parse relationships: {:?}", e)))?;

        // Create export data structure
        let export_data = ExportData {
            memories,
            relationships,
        };

        // Use DataExporter to write to file
        let exporter = DataExporter::new(format);
        exporter
            .export_to_file(&[export_data], path)
            .await
            .map_err(|e| Error::Other(format!("Export failed: {:?}", e)))
    }

    /// Import memories and relationships from a file
    pub async fn import_memories(&self, path: &Path, format: ImportFormat) -> Result<()> {
        // Use DataImporter for format-aware import
        let importer = DataImporter::new();

        // Import based on format
        let import_data_vec: Vec<ExportData> = match format {
            ImportFormat::Json => importer
                .import_json(path)
                .await
                .map_err(|e| Error::Other(format!("JSON import failed: {:?}", e)))?,
            ImportFormat::Csv => importer
                .import_csv(path)
                .await
                .map_err(|e| Error::Other(format!("CSV import failed: {:?}", e)))?,
        };

        let import_data = import_data_vec
            .into_iter()
            .next()
            .ok_or_else(|| Error::Other("No data in import file".to_string()))?;

        // Validation: Check for duplicate memory IDs
        let mut memory_ids = std::collections::HashSet::new();
        for memory in &import_data.memories {
            if !memory_ids.insert(memory.id.clone()) {
                return Err(Error::Other(format!(
                    "Duplicate memory ID found: {}",
                    memory.id
                )));
            }
        }

        // Validation: Verify relationship references exist
        for relationship in &import_data.relationships {
            if !memory_ids.contains(&relationship.source_id) {
                return Err(Error::Other(format!(
                    "Relationship references non-existent source_id: {}",
                    relationship.source_id
                )));
            }
            if !memory_ids.contains(&relationship.target_id) {
                return Err(Error::Other(format!(
                    "Relationship references non-existent target_id: {}",
                    relationship.target_id
                )));
            }
        }

        // Insert memories
        for memory in import_data.memories {
            // Use CREATE to insert with explicit ID
            let content = super::types::MemoryNodeCreateContent::from(&memory);

            let query = "
                CREATE memory CONTENT {
                    id: $id,
                    content: $content,
                    content_hash: $content_hash,
                    memory_type: $memory_type,
                    created_at: $created_at,
                    updated_at: $updated_at,
                    metadata: $metadata
                }
            ";

            self.db
                .query(query)
                .bind(("id", memory.id.clone()))
                .bind(("content", content.content))
                .bind(("content_hash", content.content_hash))
                .bind(("memory_type", format!("{:?}", content.memory_type)))
                .bind(("created_at", memory.created_at))
                .bind(("updated_at", memory.updated_at))
                .bind(("metadata", content.metadata))
                .await
                .map_err(|e| Error::Database(format!("Failed to import memory: {:?}", e)))?;
        }

        // Insert relationships
        for relationship in import_data.relationships {
            let content = super::types::RelationshipCreateContent::from(&relationship);

            let query = "
                CREATE relationship CONTENT {
                    id: $id,
                    source_id: $source_id,
                    target_id: $target_id,
                    relationship_type: $relationship_type,
                    created_at: $created_at,
                    updated_at: $updated_at,
                    strength: $strength,
                    metadata: $metadata
                }
            ";

            self.db
                .query(query)
                .bind(("id", relationship.id))
                .bind(("source_id", content.source_id))
                .bind(("target_id", content.target_id))
                .bind(("relationship_type", content.relationship_type))
                .bind(("created_at", content.created_at))
                .bind(("updated_at", content.updated_at))
                .bind(("strength", content.strength))
                .bind(("metadata", content.metadata))
                .await
                .map_err(|e| Error::Database(format!("Failed to import relationship: {:?}", e)))?;
        }

        Ok(())
    }

    /// Convert SurrealDB schema to domain MemoryNode
    pub(super) fn from_schema(schema: MemoryNodeSchema) -> MemoryNode {
        use crate::memory::core::primitives::metadata::MemoryMetadata;
        use crate::memory::core::primitives::types::MemoryContent;
        use crate::memory::monitoring::operations::OperationStatus;

        let id_str = format!("memory:{}", schema.id);

        let mut metadata = MemoryMetadata::with_memory_type(schema.memory_type);
        metadata.created_at = schema.metadata.created_at;
        metadata.last_accessed_at = Some(schema.metadata.last_accessed_at);
        metadata.importance = schema.metadata.importance;
        metadata.embedding = schema.metadata.embedding.clone();
        metadata.custom = schema.metadata.custom.clone();

        MemoryNode {
            id: id_str,
            content: MemoryContent::new(&schema.content),
            content_hash: schema.content_hash,
            memory_type: schema.memory_type,
            created_at: schema.metadata.created_at,
            updated_at: schema.metadata.last_accessed_at,
            embedding: schema.metadata.embedding,
            evaluation_status: OperationStatus::Success,
            metadata,
            relevance_score: None,
        }
    }
}
