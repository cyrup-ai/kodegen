//! Schema migration management

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::{Surreal, engine::any::Any};

use crate::memory::migration::{Migration, MigrationError, PendingMigration};

/// Schema migration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Migration version
    pub version: u32,

    /// Migration name
    pub name: String,

    /// Applied timestamp
    pub applied_at: DateTime<Utc>,

    /// Checksum of migration
    pub checksum: String,
}

/// Schema migration tracker
#[derive(Clone)]
pub struct SchemaTracker {
    /// Applied migrations
    applied: HashMap<u32, MigrationRecord>,
}

impl SchemaTracker {
    /// Create a new schema tracker
    pub fn new() -> Self {
        Self {
            applied: HashMap::new(),
        }
    }

    /// Check if a migration is applied
    pub fn is_applied(&self, version: u32) -> bool {
        self.applied.contains_key(&version)
    }

    /// Record a migration as applied
    pub fn record_migration(&mut self, version: u32, name: String, checksum: String) {
        let record = MigrationRecord {
            version,
            name,
            applied_at: crate::domain::memory::cache::get_cached_utc(),
            checksum,
        };
        self.applied.insert(version, record);
    }

    /// Get all applied migrations
    pub fn applied_migrations(&self) -> Vec<&MigrationRecord> {
        let mut migrations: Vec<_> = self.applied.values().collect();
        migrations.sort_by_key(|m| m.version);
        migrations
    }

    /// Get the current version
    pub fn current_version(&self) -> Option<u32> {
        self.applied.keys().max().copied()
    }

    /// Load migration history from database
    pub async fn load_from_db(db: &Surreal<Any>) -> crate::memory::migration::Result<Self> {
        let query = "SELECT * FROM schema_migrations";
        let mut response = db.query(query).await.map_err(|e| {
            crate::memory::migration::MigrationError::DatabaseError(format!(
                "Failed to load migrations: {:?}",
                e
            ))
        })?;

        let records: Vec<MigrationRecord> = response.take(0).unwrap_or_default();

        let mut applied = HashMap::new();
        for record in records {
            applied.insert(record.version, record);
        }

        Ok(Self { applied })
    }

    /// Save migration history to database
    pub async fn save_to_db(&self, db: &Surreal<Any>) -> crate::memory::migration::Result<()> {
        // Clear existing records
        db.query("DELETE FROM schema_migrations")
            .await
            .map_err(|e| {
                crate::memory::migration::MigrationError::DatabaseError(format!(
                    "Failed to clear schema_migrations: {:?}",
                    e
                ))
            })?;

        // Insert all current records using typed API
        for record in self.applied.values() {
            let _: Option<MigrationRecord> = db
                .create(("schema_migrations", format!("v{}", record.version)))
                .content(record.clone())
                .await
                .map_err(|e| {
                    crate::memory::migration::MigrationError::DatabaseError(format!(
                        "Failed to save migration v{}: {:?}",
                        record.version, e
                    ))
                })?;
        }

        Ok(())
    }

    /// Remove a migration from the applied set (used during rollback)
    pub fn remove_migration(&mut self, version: u32) -> Option<MigrationRecord> {
        self.applied.remove(&version)
    }
}

impl Default for SchemaTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in schema migrations
pub struct BuiltinMigrations;

impl BuiltinMigrations {
    /// Get all built-in migrations
    pub fn all() -> Vec<Box<dyn Migration>> {
        vec![
            Box::new(V1InitialSchema),
            Box::new(V2AddVectorIndex),
            Box::new(V3AddRelationshipStrength),
            Box::new(V4QuantumEntanglement),
        ]
    }
}

/// V1: Initial schema
struct V1InitialSchema;

impl Migration for V1InitialSchema {
    fn version(&self) -> u32 {
        1
    }

    fn name(&self) -> &str {
        "initial_schema"
    }

    fn content(&self) -> String {
        [
            "DEFINE TABLE IF NOT EXISTS memory SCHEMALESS",
            "DEFINE TABLE IF NOT EXISTS memory_relationship SCHEMALESS",
            "DEFINE INDEX IF NOT EXISTS memory_type_idx ON TABLE memory COLUMNS memory_type",
        ]
        .join("\n")
    }

    fn up(&self, db: Arc<Surreal<Any>>) -> PendingMigration {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // Create memory table (schemaless for flexibility)
            let result = db
                .query("DEFINE TABLE IF NOT EXISTS memory SCHEMALESS")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!("Failed to create memory table: {:?}", e))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Create relationship table
            let result = db
                .query("DEFINE TABLE IF NOT EXISTS memory_relationship SCHEMALESS")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!(
                        "Failed to create relationship table: {:?}",
                        e
                    ))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Create index on memory_type for efficient querying
            let result = db
                .query("DEFINE INDEX IF NOT EXISTS memory_type_idx ON TABLE memory COLUMNS memory_type")
                .await
                .map_err(|e| MigrationError::DatabaseError(format!("Failed to create memory_type index: {:?}", e)));

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            let _ = tx.send(Ok(()));
        });

        PendingMigration::new(rx)
    }

    fn down(&self, db: Arc<Surreal<Any>>) -> PendingMigration {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // Remove memory_type index
            let result = db
                .query("REMOVE INDEX IF EXISTS memory_type_idx ON TABLE memory")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!(
                        "Failed to remove memory_type index: {:?}",
                        e
                    ))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Drop relationship table
            let result = db
                .query("REMOVE TABLE IF EXISTS memory_relationship")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!(
                        "Failed to drop relationship table: {:?}",
                        e
                    ))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Drop memory table
            let result = db
                .query("REMOVE TABLE IF EXISTS memory")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!("Failed to drop memory table: {:?}", e))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            let _ = tx.send(Ok(()));
        });

        PendingMigration::new(rx)
    }
}

/// V2: Add vector index
struct V2AddVectorIndex;

impl Migration for V2AddVectorIndex {
    fn version(&self) -> u32 {
        2
    }

    fn name(&self) -> &str {
        "add_vector_index"
    }

    fn content(&self) -> String {
        "DEFINE INDEX IF NOT EXISTS memory_embedding_idx ON TABLE memory COLUMNS metadata.embedding MTREE DIMENSION 384 DIST COSINE TYPE F32".to_string()
    }

    fn up(&self, db: Arc<Surreal<Any>>) -> PendingMigration {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // Create vector similarity index on memory table
            // This enables efficient vector search using SurrealDB's vector::similarity functions
            let result = db
                .query("DEFINE INDEX IF NOT EXISTS memory_embedding_idx ON TABLE memory COLUMNS metadata.embedding MTREE DIMENSION 384 DIST COSINE TYPE F32")
                .await
                .map_err(|e| MigrationError::DatabaseError(format!("Failed to create vector index: {:?}", e)));

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            let _ = tx.send(Ok(()));
        });

        PendingMigration::new(rx)
    }

    fn down(&self, db: Arc<Surreal<Any>>) -> PendingMigration {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // Remove vector index
            let result = db
                .query("REMOVE INDEX IF EXISTS memory_embedding_idx ON TABLE memory")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!("Failed to remove vector index: {:?}", e))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            let _ = tx.send(Ok(()));
        });

        PendingMigration::new(rx)
    }
}

/// V3: Add relationship strength
struct V3AddRelationshipStrength;

impl Migration for V3AddRelationshipStrength {
    fn version(&self) -> u32 {
        3
    }

    fn name(&self) -> &str {
        "add_relationship_strength"
    }

    fn content(&self) -> String {
        [
            "DEFINE INDEX IF NOT EXISTS relationship_strength_idx ON TABLE memory_relationship COLUMNS strength",
            "UPDATE memory_relationship SET strength = 0.5 WHERE strength IS NULL",
        ].join("\n")
    }

    fn up(&self, db: Arc<Surreal<Any>>) -> PendingMigration {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // Add strength field to relationship table and create index
            // SurrealDB's schemaless design means we just need to add the index
            // The field will be added dynamically when records include it
            let result = db
                .query("DEFINE INDEX IF NOT EXISTS relationship_strength_idx ON TABLE memory_relationship COLUMNS strength")
                .await
                .map_err(|e| MigrationError::DatabaseError(format!("Failed to create strength index: {:?}", e)));

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Optionally: Update existing relationships to have default strength
            let result = db
                .query("UPDATE memory_relationship SET strength = 0.5 WHERE strength IS NULL")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!(
                        "Failed to set default strength: {:?}",
                        e
                    ))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            let _ = tx.send(Ok(()));
        });

        PendingMigration::new(rx)
    }

    fn down(&self, db: Arc<Surreal<Any>>) -> PendingMigration {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // Remove strength index
            let result = db
                .query(
                    "REMOVE INDEX IF EXISTS relationship_strength_idx ON TABLE memory_relationship",
                )
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!(
                        "Failed to remove strength index: {:?}",
                        e
                    ))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Note: In SurrealDB schemaless tables, we don't need to drop the field
            // It will just be ignored if not indexed. If you want to remove it:
            // UPDATE memory_relationship UNSET strength
            // (Optional, commented out for safety)

            let _ = tx.send(Ok(()));
        });

        PendingMigration::new(rx)
    }
}

/// V4: Create dedicated quantum entanglement RELATION table
struct V4QuantumEntanglement;

impl Migration for V4QuantumEntanglement {
    fn version(&self) -> u32 {
        4
    }

    fn name(&self) -> &str {
        "quantum_entanglement_relation"
    }

    fn content(&self) -> String {
        [
            "DEFINE TABLE IF NOT EXISTS entangled TYPE RELATION IN memory OUT memory ENFORCED SCHEMALESS",
            "DEFINE FIELD strength ON entangled TYPE number ASSERT $value >= 0.0 AND $value <= 1.0",
            "DEFINE FIELD bond_type ON entangled TYPE string",
            "DEFINE FIELD created_at ON entangled TYPE datetime DEFAULT time::now()",
            "DEFINE INDEX IF NOT EXISTS entangled_strength_idx ON entangled COLUMNS strength",
            "DEFINE INDEX IF NOT EXISTS entangled_type_idx ON entangled COLUMNS bond_type",
        ].join(";\n")
    }

    fn up(&self, db: Arc<Surreal<Any>>) -> PendingMigration {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // Step 1: Create RELATION table with enforced graph constraints
            let result = db
                .query("DEFINE TABLE IF NOT EXISTS entangled TYPE RELATION IN memory OUT memory ENFORCED SCHEMALESS")
                .await
                .map_err(|e| MigrationError::DatabaseError(
                    format!("Failed to create entangled RELATION table: {:?}", e)
                ));

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Step 2: Define strength field with 0.0-1.0 validation
            let result = db
                .query("DEFINE FIELD strength ON entangled TYPE number ASSERT $value >= 0.0 AND $value <= 1.0")
                .await
                .map_err(|e| MigrationError::DatabaseError(
                    format!("Failed to define strength field: {:?}", e)
                ));

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Step 3: Define bond_type field (string for flexibility)
            let result = db
                .query("DEFINE FIELD bond_type ON entangled TYPE string")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!(
                        "Failed to define bond_type field: {:?}",
                        e
                    ))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Step 4: Define created_at with auto-timestamp
            let result = db
                .query("DEFINE FIELD created_at ON entangled TYPE datetime DEFAULT time::now()")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!(
                        "Failed to define created_at field: {:?}",
                        e
                    ))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Step 5: Create strength index for range queries
            let result = db
                .query("DEFINE INDEX IF NOT EXISTS entangled_strength_idx ON entangled COLUMNS strength")
                .await
                .map_err(|e| MigrationError::DatabaseError(
                    format!("Failed to create strength index: {:?}", e)
                ));

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            // Step 6: Create bond_type index for type filtering
            let result = db
                .query(
                    "DEFINE INDEX IF NOT EXISTS entangled_type_idx ON entangled COLUMNS bond_type",
                )
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!(
                        "Failed to create bond_type index: {:?}",
                        e
                    ))
                });

            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }

            log::info!("V4 migration complete: entangled RELATION table created with indexes");
            let _ = tx.send(Ok(()));
        });

        PendingMigration::new(rx)
    }

    fn down(&self, db: Arc<Surreal<Any>>) -> PendingMigration {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // Remove indexes first (reverse order)
            let _ = db
                .query("REMOVE INDEX IF EXISTS entangled_type_idx ON entangled")
                .await;

            let _ = db
                .query("REMOVE INDEX IF EXISTS entangled_strength_idx ON entangled")
                .await;

            // Remove RELATION table (fields cascade automatically)
            let result = db
                .query("REMOVE TABLE IF EXISTS entangled")
                .await
                .map_err(|e| {
                    MigrationError::DatabaseError(format!(
                        "Failed to drop entangled table: {:?}",
                        e
                    ))
                });

            log::info!("V4 migration rolled back: entangled RELATION table removed");
            let _ = tx.send(result.map(|_| ()));
        });

        PendingMigration::new(rx)
    }
}
