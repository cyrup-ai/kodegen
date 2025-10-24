//! Migration module for mem0-rs
//!
//! This module provides functionality for data migration, import/export,
//! and schema evolution for the memory system.

pub mod converter;
pub mod exporter;
pub mod importer;
pub mod schema_migrations;
pub mod validator;

// Re-export main types
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub use converter::*;
pub use exporter::*;
pub use importer::*;
pub use schema_migrations::*;
use sha2::{Digest, Sha256};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::sync::oneshot;
pub use validator::*;

/// Result type for migration operations
pub type Result<T> = std::result::Result<T, MigrationError>;

/// Migration error types
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Schema mismatch: expected {expected}, found {found}")]
    SchemaMismatch { expected: String, found: String },

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

/// Migration direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationDirection {
    Up,
    Down,
}

/// A pending migration operation that can be awaited
pub struct PendingMigration {
    rx: oneshot::Receiver<Result<()>>,
}

impl PendingMigration {
    pub fn new(rx: oneshot::Receiver<Result<()>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingMigration {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => Poll::Ready(Err(MigrationError::DatabaseError(
                "Migration task failed".to_string(),
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Migration trait
pub trait Migration: Send + Sync {
    /// Get the migration version
    fn version(&self) -> u32;

    /// Get the migration name
    fn name(&self) -> &str;

    /// Return canonical content representation for checksum calculation
    /// This should include all SQL statements in deterministic order
    fn content(&self) -> String;

    /// Apply the migration
    fn up(&self, db: Arc<Surreal<Any>>) -> PendingMigration;

    /// Rollback the migration
    fn down(&self, db: Arc<Surreal<Any>>) -> PendingMigration;
}

/// Migration manager with SurrealDB integration
pub struct MigrationManager {
    db: Arc<Surreal<Any>>,
    migrations: Vec<Box<dyn Migration>>,
    tracker: SchemaTracker,
}

impl MigrationManager {
    /// Create a new migration manager with database connection
    pub async fn new(db: Arc<Surreal<Any>>) -> Result<Self> {
        // Ensure schema_migrations table exists
        db.query("DEFINE TABLE IF NOT EXISTS schema_migrations SCHEMALESS")
            .await
            .map_err(|e| MigrationError::DatabaseError(format!("{:?}", e)))?;

        // Load existing migration records using SchemaTracker's persistence
        let tracker = SchemaTracker::load_from_db(&db).await?;

        Ok(Self {
            db,
            migrations: Vec::new(),
            tracker,
        })
    }

    /// Add a migration
    pub fn add_migration(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(migration);
    }

    /// Run pending migrations
    pub async fn migrate(&mut self) -> Result<()> {
        // Sort migrations by version
        self.migrations.sort_by_key(|m| m.version());

        for migration in &self.migrations {
            let version = migration.version();

            // Calculate SHA256 checksum from version, name, and content
            let content_to_hash = format!(
                "version:{}\nname:{}\ncontent:{}",
                version,
                migration.name(),
                migration.content()
            );
            let digest = Sha256::digest(content_to_hash.as_bytes());
            let checksum = digest
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();

            // If migration was previously applied, verify checksum hasn't changed
            if self.tracker.is_applied(version) {
                if let Some(existing_record) = self
                    .tracker
                    .applied_migrations()
                    .iter()
                    .find(|r| r.version == version)
                    && existing_record.checksum != checksum
                {
                    return Err(MigrationError::ValidationFailed(format!(
                        "Migration v{} ({}) checksum mismatch: expected {}, found {}. Migration code may have been tampered with.",
                        version,
                        migration.name(),
                        existing_record.checksum,
                        checksum
                    )));
                }
                log::debug!(
                    "Migration v{} ({}) already applied, skipping",
                    version,
                    migration.name()
                );
                continue;
            }

            log::info!("Applying migration v{}: {}", version, migration.name());

            // Execute migration (await the PendingMigration)
            migration.up(Arc::clone(&self.db)).await?;

            // Record in database
            let record = MigrationRecord {
                version,
                name: migration.name().to_string(),
                applied_at: crate::domain::memory::cache::get_cached_utc(),
                checksum: checksum.clone(),
            };

            let _: Option<MigrationRecord> = self
                .db
                .create(("schema_migrations", format!("v{}", version)))
                .content(record.clone())
                .await
                .map_err(|e| MigrationError::DatabaseError(format!("{:?}", e)))?;

            // Update tracker
            self.tracker
                .record_migration(version, migration.name().to_string(), checksum);

            log::info!(
                "Migration v{} ({}) applied successfully",
                version,
                migration.name()
            );
        }

        Ok(())
    }

    /// Rollback to a specific version
    pub async fn rollback_to(&mut self, target_version: u32) -> Result<()> {
        // Sort migrations in reverse order
        self.migrations
            .sort_by_key(|m| std::cmp::Reverse(m.version()));

        for migration in &self.migrations {
            let version = migration.version();

            if version <= target_version {
                break;
            }

            if !self.tracker.is_applied(version) {
                continue;
            }

            log::info!("Rolling back migration v{}: {}", version, migration.name());

            migration.down(Arc::clone(&self.db)).await?;

            // Remove from database
            self.db
                .delete::<Option<MigrationRecord>>(("schema_migrations", format!("v{}", version)))
                .await
                .map_err(|e| MigrationError::DatabaseError(format!("{:?}", e)))?;

            // FIX: Update tracker to reflect rollback
            self.tracker.remove_migration(version);
        }

        Ok(())
    }
}
