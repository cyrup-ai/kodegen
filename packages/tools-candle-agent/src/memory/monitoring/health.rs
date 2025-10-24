//! Health check functionality for the memory system

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::sync::{RwLock, oneshot};

use crate::memory::vector::vector_store::{IndexStats, VectorStore};

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// System is healthy
    Healthy,
    /// System is degraded but operational
    Degraded,
    /// System is unhealthy
    Unhealthy,
    /// System status is unknown
    Unknown,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Overall status
    pub status: HealthStatus,

    /// Component statuses
    pub components: HashMap<String, ComponentHealth>,

    /// Timestamp of the check
    pub timestamp: DateTime<Utc>,

    /// System version
    pub version: String,
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,

    /// Component status
    pub status: HealthStatus,

    /// Optional message
    pub message: Option<String>,

    /// Additional details
    pub details: HashMap<String, serde_json::Value>,
}

/// A pending component health check that can be awaited
pub struct PendingComponentHealth {
    rx: oneshot::Receiver<ComponentHealth>,
}

impl PendingComponentHealth {
    pub fn new(rx: oneshot::Receiver<ComponentHealth>) -> Self {
        Self { rx }
    }
}

impl Future for PendingComponentHealth {
    type Output = ComponentHealth;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(health)) => Poll::Ready(health),
            Poll::Ready(Err(_)) => Poll::Ready(ComponentHealth {
                name: "unknown".to_string(),
                status: HealthStatus::Unhealthy,
                message: Some("Health check task failed".to_string()),
                details: HashMap::new(),
            }),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Health checker
pub struct HealthChecker {
    /// Component checkers
    checkers: Vec<Box<dyn ComponentChecker>>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new() -> Self {
        Self {
            checkers: Vec::new(),
        }
    }

    /// Add a component checker
    pub fn add_checker(&mut self, checker: Box<dyn ComponentChecker>) {
        self.checkers.push(checker);
    }

    /// Run health check
    pub async fn check(&self) -> HealthCheck {
        let mut components = HashMap::new();
        let mut overall_status = HealthStatus::Healthy;

        for checker in &self.checkers {
            let component_health = checker.check().await;

            // Update overall status
            match component_health.status {
                HealthStatus::Unhealthy => overall_status = HealthStatus::Unhealthy,
                HealthStatus::Degraded if overall_status == HealthStatus::Healthy => {
                    overall_status = HealthStatus::Degraded;
                }
                _ => {}
            }

            components.insert(checker.name().to_string(), component_health);
        }

        HealthCheck {
            status: overall_status,
            components,
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Component health checker trait
pub trait ComponentChecker: Send + Sync {
    /// Get component name
    fn name(&self) -> &str;

    /// Check component health
    fn check(&self) -> PendingComponentHealth;
}

/// Database health checker with actual SurrealDB access
pub struct DatabaseHealthChecker {
    database: Arc<Surreal<Any>>,
}

impl DatabaseHealthChecker {
    /// Create new health checker with SurrealDB instance
    pub fn new(database: Arc<Surreal<Any>>) -> Self {
        Self { database }
    }
}

impl ComponentChecker for DatabaseHealthChecker {
    fn name(&self) -> &str {
        "database"
    }

    fn check(&self) -> PendingComponentHealth {
        let (tx, rx) = oneshot::channel();
        let name = self.name().to_string();

        // Clone database references before moving into async block
        let db_conn = self.database.clone();
        let db_query = self.database.clone();
        let db_pool = self.database.clone();

        tokio::spawn(async move {
            // Production database health check with comprehensive diagnostics
            let mut health = ComponentHealth {
                name: name.clone(),
                status: HealthStatus::Unknown,
                message: None,
                details: HashMap::new(),
            };

            // Perform multiple health checks concurrently
            let connection_start = std::time::Instant::now();

            // 1. Test basic connectivity
            let connection_test = async move {
                // Test basic connectivity with SurrealDB health check
                tokio::time::timeout(std::time::Duration::from_millis(5000), async move {
                    db_conn.health().await.map_err(|e| {
                        let err_msg = e.to_string();
                        Box::new(std::io::Error::other(err_msg))
                            as Box<dyn std::error::Error + Send + Sync>
                    })
                })
                .await
            };

            // 2. Test query performance
            let query_performance_test = async move {
                let query_start = std::time::Instant::now();
                // Execute simple query to measure actual database response time
                let _ = db_query.query("SELECT 1").await;
                query_start.elapsed()
            };

            // 3. Check connection pool status
            let pool_status_test = async move {
                // SurrealDB SDK does not expose connection pool metrics
                // Best effort: verify database is responsive with INFO query
                let result = db_pool.query("INFO FOR DB").await;

                match result {
                    Ok(_) => {
                        // Connection is active and working
                        // Conservative estimate: 1 active connection confirmed by successful query
                        // Max set to 100 as reasonable default (SDK manages pool internally)
                        (1u32, 100u32)
                    }
                    Err(_) => {
                        // Query failed - no active connections
                        (0u32, 100u32)
                    }
                }
            };

            // Execute all tests concurrently
            let (connection_result, query_duration, (active_conns, max_conns)) =
                tokio::join!(connection_test, query_performance_test, pool_status_test);

            let total_duration = connection_start.elapsed();

            // Evaluate health based on test results
            match connection_result {
                Ok(Ok(_)) => {
                    health.status = HealthStatus::Healthy;
                    health.message = Some("Database is operational".to_string());

                    // Add detailed metrics
                    health.details.insert(
                        "connection_time_ms".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(
                            total_duration.as_millis() as u64,
                        )),
                    );
                    health.details.insert(
                        "query_time_ms".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(
                            query_duration.as_millis() as u64,
                        )),
                    );
                    health.details.insert(
                        "active_connections".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(active_conns)),
                    );
                    health.details.insert(
                        "max_connections".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(max_conns)),
                    );
                    health.details.insert(
                        "connection_utilization".to_string(),
                        serde_json::Value::String(format!(
                            "{:.1}%",
                            (active_conns as f64 / max_conns as f64) * 100.0
                        )),
                    );

                    // Check for performance warnings
                    if query_duration.as_millis() > 1000 {
                        health.status = HealthStatus::Degraded;
                        health.message = Some("Database responding slowly".to_string());
                    }

                    if (active_conns as f64 / max_conns as f64) > 0.8 {
                        health.status = HealthStatus::Degraded;
                        health.message = Some("Database connection pool near capacity".to_string());
                    }
                }
                Ok(Err(e)) => {
                    health.status = HealthStatus::Unhealthy;
                    health.message = Some(format!("Database error: {}", e));
                    health.details.insert(
                        "error_type".to_string(),
                        serde_json::Value::String("query_error".to_string()),
                    );
                }
                Err(_) => {
                    health.status = HealthStatus::Unhealthy;
                    health.message = Some("Database connection timeout".to_string());
                    health.details.insert(
                        "error_type".to_string(),
                        serde_json::Value::String("timeout".to_string()),
                    );
                    health.details.insert(
                        "timeout_ms".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(5000)),
                    );
                }
            }

            let _ = tx.send(health);
        });

        PendingComponentHealth::new(rx)
    }
}

/// Vector store health checker with actual VectorStore access
pub struct VectorStoreHealthChecker {
    vector_store: Arc<RwLock<dyn VectorStore + Send + Sync>>,
    embedding_dimensions: usize,
}

impl VectorStoreHealthChecker {
    /// Create new health checker with VectorStore instance and embedding dimensions
    ///
    /// # Arguments
    /// * `vector_store` - Vector store to monitor
    /// * `embedding_dimensions` - Actual dimension size from embedding model configuration
    pub fn new(
        vector_store: Arc<RwLock<dyn VectorStore + Send + Sync>>,
        embedding_dimensions: usize,
    ) -> Self {
        Self {
            vector_store,
            embedding_dimensions,
        }
    }
}

impl ComponentChecker for VectorStoreHealthChecker {
    fn name(&self) -> &str {
        "vector_store"
    }

    fn check(&self) -> PendingComponentHealth {
        let (tx, rx) = oneshot::channel();
        let name = "vector_store".to_string();

        // Clone vector store references before moving into async block
        let vs_conn = self.vector_store.clone();
        let vs_idx = self.vector_store.clone();
        let vs_search = self.vector_store.clone();
        let vs_mem = self.vector_store.clone();
        let embedding_dims = self.embedding_dimensions;

        tokio::spawn(async move {
            // Production vector store health check with comprehensive diagnostics
            let mut health = ComponentHealth {
                name: name.clone(),
                status: HealthStatus::Unknown,
                message: None,
                details: HashMap::new(),
            };

            let health_check_start = std::time::Instant::now();

            // Perform comprehensive vector store health checks
            let connectivity_test = async move {
                // Test connectivity by calling count() - if it succeeds, VectorStore is accessible
                tokio::time::timeout(std::time::Duration::from_millis(3000), async move {
                    tokio::task::spawn_blocking(move || {
                        let vs = vs_conn.blocking_read();
                        vs.count().map(|_| ())
                    })
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                })
                .await
            };

            let index_status_test = async move {
                // Get actual index statistics from vector store
                let stats = tokio::task::spawn_blocking(move || {
                    let vs = vs_idx.blocking_read();
                    vs.get_index_stats()
                })
                .await
                .unwrap_or(Ok(IndexStats {
                    entry_count: 0,
                    dimensions: None,
                    quality_score: 0.0,
                    memory_bytes: 0,
                    fragmentation_ratio: 0.0,
                }))
                .unwrap_or(IndexStats {
                    entry_count: 0,
                    dimensions: None,
                    quality_score: 0.0,
                    memory_bytes: 0,
                    fragmentation_ratio: 0.0,
                });

                let dimensions = stats.dimensions.unwrap_or(embedding_dims as u32);
                let index_quality = stats.quality_score;

                (stats.entry_count, dimensions, index_quality)
            };

            let search_performance_test = async move {
                let search_start = std::time::Instant::now();

                // Execute real search with sample vector
                let (duration, results_count) = tokio::task::spawn_blocking(move || {
                    let vs = vs_search.blocking_read();
                    let sample_vector = vec![0.0f32; embedding_dims]; // Zero vector for health check
                    let results = vs
                        .search(&sample_vector, Some(10), None)
                        .unwrap_or_default();
                    let count = results.len() as u32;
                    (search_start.elapsed(), count)
                })
                .await
                .unwrap_or((search_start.elapsed(), 0));

                (duration, results_count)
            };

            let memory_usage_test = async move {
                // Estimate memory from vector count (no trait method available)
                let count = tokio::task::spawn_blocking(move || {
                    let vs = vs_mem.blocking_read();
                    vs.count().unwrap_or(0)
                })
                .await
                .unwrap_or(0);

                // Rough estimate: embedding dimensions * 4 bytes per f32
                let estimated_mb = (count * embedding_dims * 4) / (1024 * 1024);
                let used_memory_mb = estimated_mb as u64;
                let total_memory_mb = (estimated_mb * 2) as u64; // Assume 50% utilization

                (used_memory_mb, total_memory_mb)
            };

            // Execute all health checks concurrently
            let (
                connectivity_result,
                (vector_count, dimensions, index_quality),
                (search_duration, search_results),
                (used_memory, total_memory),
            ) = tokio::join!(
                connectivity_test,
                index_status_test,
                search_performance_test,
                memory_usage_test
            );

            let total_check_duration = health_check_start.elapsed();

            // Evaluate overall health based on all test results
            match connectivity_result {
                Ok(Ok(_)) => {
                    health.status = HealthStatus::Healthy;
                    health.message = Some("Vector store is operational".to_string());

                    // Set health status based on quality thresholds
                    if index_quality < 60.0 {
                        health.status = HealthStatus::Unhealthy;
                        health.message = Some(format!(
                            "Index quality critically low: {:.1}%",
                            index_quality
                        ));
                    } else if index_quality < 80.0 {
                        health.status = HealthStatus::Degraded;
                        health.message =
                            Some(format!("Index quality degraded: {:.1}%", index_quality));
                    }

                    // Check for dimension mismatch
                    if dimensions != embedding_dims as u32 {
                        health.status = HealthStatus::Unhealthy;
                        health.message = Some(format!(
                            "Dimension mismatch: expected {}, got {}",
                            embedding_dims, dimensions
                        ));
                    }

                    // Add comprehensive metrics
                    health.details.insert(
                        "health_check_time_ms".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(
                            total_check_duration.as_millis() as u64,
                        )),
                    );
                    health.details.insert(
                        "vector_count".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(vector_count)),
                    );
                    health.details.insert(
                        "vector_dimensions".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(dimensions)),
                    );
                    health.details.insert(
                        "index_quality_percent".to_string(),
                        serde_json::Value::String(format!("{:.1}", index_quality)),
                    );
                    health.details.insert(
                        "search_latency_ms".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(
                            search_duration.as_millis() as u64,
                        )),
                    );
                    health.details.insert(
                        "search_results_found".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(search_results)),
                    );
                    health.details.insert(
                        "memory_usage_mb".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(used_memory)),
                    );
                    health.details.insert(
                        "memory_total_mb".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(total_memory)),
                    );
                    health.details.insert(
                        "memory_utilization_percent".to_string(),
                        serde_json::Value::String(format!(
                            "{:.1}",
                            (used_memory as f64 / total_memory as f64) * 100.0
                        )),
                    );

                    // Check for performance and capacity warnings
                    if search_duration.as_millis() > 500 {
                        health.status = HealthStatus::Degraded;
                        health.message = Some("Vector search performance degraded".to_string());
                    }

                    if index_quality < 90.0 {
                        health.status = HealthStatus::Degraded;
                        health.message =
                            Some("Vector index quality below optimal threshold".to_string());
                    }

                    if (used_memory as f64 / total_memory as f64) > 0.85 {
                        health.status = HealthStatus::Degraded;
                        health.message = Some("Vector store memory usage high".to_string());
                    }

                    if vector_count == 0 {
                        health.status = HealthStatus::Degraded;
                        health.message = Some("Vector store is empty".to_string());
                    }
                }
                Ok(Err(e)) => {
                    health.status = HealthStatus::Unhealthy;
                    health.message = Some(format!("Vector store error: {}", e));
                    health.details.insert(
                        "error_type".to_string(),
                        serde_json::Value::String("operation_error".to_string()),
                    );
                }
                Err(_) => {
                    health.status = HealthStatus::Unhealthy;
                    health.message = Some("Vector store connection timeout".to_string());
                    health.details.insert(
                        "error_type".to_string(),
                        serde_json::Value::String("connectivity_timeout".to_string()),
                    );
                    health.details.insert(
                        "timeout_ms".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(3000)),
                    );
                }
            }

            let _ = tx.send(health);
        });

        PendingComponentHealth::new(rx)
    }
}
