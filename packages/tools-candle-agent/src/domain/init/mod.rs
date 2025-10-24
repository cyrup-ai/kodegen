//! Domain initialization and configuration

pub mod globals;

use std::sync::{Arc, LazyLock};
use tokio::sync::{Mutex, mpsc};

use crate::domain::memory::MemoryConfig;
use crate::memory::core::manager::surreal::SurrealDBMemoryManager;
use surrealdb::engine::any;

use crate::domain::core::DomainInitError;

/// Type alias for memory manager connection pool channel pair
type MemoryManagerPool = (
    mpsc::UnboundedSender<Arc<SurrealDBMemoryManager>>,
    Arc<Mutex<mpsc::UnboundedReceiver<Arc<SurrealDBMemoryManager>>>>,
);

/// Initialize memory service with default configuration
///
/// Returns a properly configured memory manager service (not streamed)
/// Following the universal pattern: managers are services, not data to be streamed
///
/// # Errors
///
/// Returns `DomainInitError` if:
/// - Database connection cannot be established
/// - Database initialization fails
/// - Memory schema initialization fails
pub async fn initialize_memory_service() -> Result<SurrealDBMemoryManager, DomainInitError> {
    let config = get_default_memory_config();
    initialize_memory_service_with_config(config).await
}

/// Initialize memory service with custom configuration
///
/// # Arguments
/// * `config` - Memory configuration with database settings
///
/// # Returns
/// Properly initialized `SurrealDBMemoryManager` ready for use
///
/// # Errors
///
/// Returns `DomainInitError` if:
/// - Database connection to `SurrealDB` fails
/// - Namespace or database selection fails
/// - Memory schema and index initialization fails
///
/// # Performance
/// Production-quality `SurrealDB` connection with cognitive memory features
pub async fn initialize_memory_service_with_config(
    config: MemoryConfig,
) -> Result<SurrealDBMemoryManager, DomainInitError> {
    // Connect to SurrealDB using the real SDK
    let db = any::connect(&config.database.connection_string)
        .await
        .map_err(|e| DomainInitError::DatabaseConnectionFailed(e.to_string()))?;

    // Use namespace and database
    db.use_ns(&config.database.namespace)
        .use_db(&config.database.database)
        .await
        .map_err(|e| DomainInitError::DatabaseInitializationFailed(e.to_string()))?;

    // Create the real memory manager with SurrealDB connection and default embeddings from registry
    let manager = SurrealDBMemoryManager::new(db);

    // Initialize the memory schema and indexes
    manager
        .initialize()
        .await
        .map_err(|e| DomainInitError::MemoryInitializationFailed(e.to_string()))?;

    Ok(manager)
}

/// Get default memory configuration
///
/// # Returns
/// Default configuration for local `SurrealDB` with cognitive features
#[must_use]
pub fn get_default_memory_config() -> MemoryConfig {
    MemoryConfig::default()
}

/// Memory service connection pool for efficient resource management
///
/// Uses Arc<SurrealDBMemoryManager> for shared ownership across threads
static MEMORY_SERVICE_POOL: LazyLock<MemoryManagerPool> = LazyLock::new(|| {
    let (sender, receiver) = mpsc::unbounded_channel();
    (sender, Arc::new(Mutex::new(receiver)))
});

/// Get a memory manager from the connection pool
///
/// # Returns
/// Shared reference to memory manager service, or None if pool is empty
pub async fn get_from_pool() -> Option<Arc<SurrealDBMemoryManager>> {
    let (_sender, receiver) = &*MEMORY_SERVICE_POOL;
    let mut rx = receiver.lock().await;
    rx.recv().await
}

/// Return a memory manager to the connection pool
///
/// # Arguments
/// * `memory` - Memory manager service to return to pool
pub fn return_to_pool(memory: Arc<SurrealDBMemoryManager>) {
    let (sender, _receiver) = &*MEMORY_SERVICE_POOL;
    let _ = sender.send(memory);
}

/// Get the current connection pool size
///
/// # Returns
/// Number of available managers in the pool (note: tokio mpsc doesn't expose queue length)
#[must_use]
pub fn pool_size() -> usize {
    0 // tokio mpsc channels don't expose queue length
}

/// Initialize the connection pool with pre-configured managers
///
/// # Arguments
/// * `pool_size` - Number of managers to pre-allocate
/// * `config` - Configuration for each manager
///
/// # Errors
///
/// Returns `DomainInitError` if any manager in the pool fails to initialize
///
/// # Performance
/// Async initialization with proper error handling
pub async fn initialize_pool(
    pool_size: usize,
    config: MemoryConfig,
) -> Result<(), DomainInitError> {
    for _ in 0..pool_size {
        let manager = initialize_memory_service_with_config(config.clone()).await?;
        return_to_pool(Arc::new(manager));
    }
    Ok(())
}
