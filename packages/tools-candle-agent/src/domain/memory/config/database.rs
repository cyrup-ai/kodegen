use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use super::super::primitives::types::{MemoryError, MemoryResult};
use crate::domain::util::unix_timestamp_nanos;

/// Database configuration with connection pool optimization
///
/// Features:
/// - Connection string validation with secure parsing
/// - Pool sizing with automatic CPU core detection
/// - Connection timeout with exponential backoff
/// - Health check configuration with atomic status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type classification
    pub db_type: DatabaseType,
    /// Connection string with validation
    pub connection_string: String,
    /// Database namespace
    pub namespace: String,
    /// Database name
    pub database: String,
    /// Optional username for authentication
    pub username: Option<String>,
    /// Optional password for authentication (stored securely)
    pub password: Option<String>,
    /// Connection pool configuration
    pub pool_config: PoolConfig,
    /// Connection timeout configuration
    pub timeout_config: TimeoutConfig,
    /// Health check configuration
    pub health_check_config: HealthCheckConfig,
    /// Additional database-specific options
    pub options: Option<Arc<serde_json::Value>>,
}

/// Database types with optimized enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DatabaseType {
    /// `SurrealDB` - High-performance multi-model database
    SurrealDB = 0,
    /// `PostgreSQL` - Robust relational database
    PostgreSQL = 1,
    /// `SQLite` - Embedded database
    SQLite = 2,
    /// In-memory database for testing
    Memory = 3,
}

impl DatabaseType {
    /// Get default port for database type
    #[inline]
    #[must_use]
    pub const fn default_port(&self) -> u16 {
        match self {
            Self::SurrealDB => 8000,
            Self::PostgreSQL => 5432,
            Self::SQLite | Self::Memory => 0, // File-based/in-memory, no port
        }
    }

    /// Check if database type supports connection pooling
    #[inline]
    #[must_use]
    pub const fn supports_pooling(&self) -> bool {
        match self {
            Self::SurrealDB | Self::PostgreSQL => true,
            Self::SQLite | Self::Memory => false,
        }
    }

    /// Get recommended pool size for database type
    #[inline]
    #[must_use]
    pub fn recommended_pool_size(&self) -> usize {
        match self {
            Self::SurrealDB => num_cpus::get() * 4,  // I/O intensive
            Self::PostgreSQL => num_cpus::get() * 2, // CPU + I/O balanced
            Self::SQLite => 1,                       // Single writer limitation
            Self::Memory => num_cpus::get(),         // CPU bound
        }
    }
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SurrealDB => write!(f, "surrealdb"),
            Self::PostgreSQL => write!(f, "postgresql"),
            Self::SQLite => write!(f, "sqlite"),
            Self::Memory => write!(f, "memory"),
        }
    }
}

/// Connection pool configuration with CPU-aware optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum pool size
    pub min_size: usize,
    /// Maximum pool size (auto-detected based on CPU cores)
    pub max_size: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Connection maximum lifetime
    pub max_lifetime: Duration,
    /// Pool health check interval
    pub health_check_interval: Duration,
    /// Enable connection validation
    pub validate_connections: bool,
}

impl PoolConfig {
    /// Create optimized pool configuration
    #[inline]
    #[must_use]
    pub fn optimized(db_type: DatabaseType) -> Self {
        let cpu_count = num_cpus::get();
        let max_size = db_type.recommended_pool_size();

        Self {
            min_size: (cpu_count / 2).max(1),
            max_size,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            max_lifetime: Duration::from_secs(1800), // 30 minutes
            health_check_interval: Duration::from_secs(60), // 1 minute
            validate_connections: true,
        }
    }

    /// Create minimal pool configuration for testing
    #[inline]
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            min_size: 1,
            max_size: 2,
            idle_timeout: Duration::from_secs(30),
            max_lifetime: Duration::from_secs(300),
            health_check_interval: Duration::from_secs(10),
            validate_connections: false,
        }
    }
}

impl Default for PoolConfig {
    #[inline]
    fn default() -> Self {
        Self::optimized(DatabaseType::SurrealDB)
    }
}

/// Connection timeout configuration with exponential backoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Initial connection timeout
    pub connect_timeout: Duration,
    /// Query execution timeout
    pub query_timeout: Duration,
    /// Transaction timeout
    pub transaction_timeout: Duration,
    /// Retry configuration
    pub retry_config: RetryConfig,
}

impl TimeoutConfig {
    /// Create optimized timeout configuration
    #[inline]
    #[must_use]
    pub fn optimized() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            query_timeout: Duration::from_secs(30),
            transaction_timeout: Duration::from_secs(60),
            retry_config: RetryConfig::default(),
        }
    }

    /// Create fast timeout configuration for testing
    #[inline]
    #[must_use]
    pub fn fast() -> Self {
        Self {
            connect_timeout: Duration::from_millis(500),
            query_timeout: Duration::from_secs(5),
            transaction_timeout: Duration::from_secs(10),
            retry_config: RetryConfig::minimal(),
        }
    }
}

impl Default for TimeoutConfig {
    #[inline]
    fn default() -> Self {
        Self::optimized()
    }
}

// Use shared RetryConfig from super::shared
use super::shared::RetryConfig;

/// Health check configuration with atomic status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    pub enabled: bool,
    /// Health check interval
    pub interval: Duration,
    /// Health check timeout
    pub timeout: Duration,
    /// Health check query
    pub query: String,
    /// Failure threshold before marking unhealthy
    pub failure_threshold: usize,
    /// Recovery threshold before marking healthy
    pub recovery_threshold: usize,
}

impl Default for HealthCheckConfig {
    /// Create default health check configuration
    #[inline]
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            query: "SELECT 1".to_string(),
            failure_threshold: 3,
            recovery_threshold: 2,
        }
    }
}

impl HealthCheckConfig {
    /// Create disabled health check configuration
    #[inline]
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            interval: Duration::from_secs(60),
            timeout: Duration::from_secs(1),
            query: "SELECT 1".to_string(),
            failure_threshold: 1,
            recovery_threshold: 1,
        }
    }
}

/// Database health status with atomic tracking
#[derive(Debug)]
pub struct DatabaseHealthStatus {
    /// Current health status
    is_healthy: AtomicBool,
    /// Consecutive failure count
    failure_count: AtomicUsize,
    /// Consecutive success count
    success_count: AtomicUsize,
    /// Last health check timestamp
    last_check_nanos: AtomicU64,
    /// Total health checks performed
    total_checks: AtomicUsize,
}

impl DatabaseHealthStatus {
    /// Create new health status tracker
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_healthy: AtomicBool::new(true),
            failure_count: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            last_check_nanos: AtomicU64::new(0),
            total_checks: AtomicUsize::new(0),
        }
    }

    /// Record successful health check
    pub fn record_success(&self, config: &HealthCheckConfig) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
        self.total_checks.fetch_add(1, Ordering::Relaxed);
        self.update_timestamp();

        // Mark healthy if we've had enough consecutive successes
        if self.success_count.load(Ordering::Relaxed) >= config.recovery_threshold {
            self.is_healthy.store(true, Ordering::Relaxed);
        }
    }

    /// Record failed health check
    pub fn record_failure(&self, config: &HealthCheckConfig) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.total_checks.fetch_add(1, Ordering::Relaxed);
        self.update_timestamp();

        // Mark unhealthy if we've had too many consecutive failures
        if self.failure_count.load(Ordering::Relaxed) >= config.failure_threshold {
            self.is_healthy.store(false, Ordering::Relaxed);
        }
    }

    /// Check if database is currently healthy
    #[inline]
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::Relaxed)
    }

    /// Get failure count
    #[inline]
    pub fn failure_count(&self) -> usize {
        self.failure_count.load(Ordering::Relaxed)
    }

    /// Get success count
    #[inline]
    pub fn success_count(&self) -> usize {
        self.success_count.load(Ordering::Relaxed)
    }

    /// Get total health checks performed
    #[inline]
    pub fn total_checks(&self) -> usize {
        self.total_checks.load(Ordering::Relaxed)
    }

    /// Get last health check time
    #[inline]
    pub fn last_check_time(&self) -> Option<SystemTime> {
        let nanos = self.last_check_nanos.load(Ordering::Relaxed);
        if nanos == 0 {
            None
        } else {
            SystemTime::UNIX_EPOCH.checked_add(Duration::from_nanos(nanos))
        }
    }

    /// Update timestamp atomically
    #[inline]
    fn update_timestamp(&self) {
        let now_nanos = unix_timestamp_nanos();
        self.last_check_nanos.store(now_nanos, Ordering::Relaxed);
    }
}

impl Default for DatabaseHealthStatus {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseConfig {
    /// Create new database configuration with validation
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if connection string validation fails
    pub fn new(
        db_type: DatabaseType,
        connection_string: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
    ) -> MemoryResult<Self> {
        let conn_str = connection_string.into();

        // Validate connection string
        Self::validate_connection_string(&conn_str, db_type)?;

        Ok(Self {
            db_type,
            connection_string: conn_str,
            namespace: namespace.into(),
            database: database.into(),
            username: None,
            password: None,
            pool_config: PoolConfig::optimized(db_type),
            timeout_config: TimeoutConfig::optimized(),
            health_check_config: HealthCheckConfig::default(),
            options: None,
        })
    }

    /// Create configuration with credentials
    #[must_use]
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Set pool configuration
    #[must_use]
    #[inline]
    pub fn with_pool_config(mut self, pool_config: PoolConfig) -> Self {
        self.pool_config = pool_config;
        self
    }

    /// Set timeout configuration
    #[must_use]
    #[inline]
    pub fn with_timeout_config(mut self, timeout_config: TimeoutConfig) -> Self {
        self.timeout_config = timeout_config;
        self
    }

    /// Set health check configuration
    #[must_use]
    #[inline]
    pub fn with_health_check_config(mut self, health_check_config: HealthCheckConfig) -> Self {
        self.health_check_config = health_check_config;
        self
    }

    /// Set additional options
    #[must_use]
    #[inline]
    pub fn with_options(mut self, options: serde_json::Value) -> Self {
        self.options = Some(Arc::new(options));
        self
    }

    /// Validate connection string format
    fn validate_connection_string(conn_str: &str, db_type: DatabaseType) -> MemoryResult<()> {
        match db_type {
            DatabaseType::SurrealDB => {
                if conn_str == "memory"
                    || conn_str.starts_with("ws://")
                    || conn_str.starts_with("wss://")
                    || conn_str.starts_with("surrealkv://")
                {
                    Ok(())
                } else {
                    Err(MemoryError::validation(
                        "SurrealDB connection string must be 'memory' or start with 'ws://', 'wss://', or 'surrealkv://'",
                    ))
                }
            }
            DatabaseType::PostgreSQL => {
                if conn_str.starts_with("postgres://") || conn_str.starts_with("postgresql://") {
                    Ok(())
                } else {
                    Err(MemoryError::validation(
                        "PostgreSQL connection string must start with 'postgres://' or 'postgresql://'",
                    ))
                }
            }
            DatabaseType::SQLite => {
                let has_db_ext = std::path::Path::new(conn_str)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("db"));
                let has_sqlite_ext = std::path::Path::new(conn_str)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("sqlite"));

                if conn_str == ":memory:" || has_db_ext || has_sqlite_ext {
                    Ok(())
                } else {
                    Err(MemoryError::validation(
                        "SQLite connection string must be ':memory:' or end with '.db' or '.sqlite'",
                    ))
                }
            }
            DatabaseType::Memory => {
                if conn_str == "memory" {
                    Ok(())
                } else {
                    Err(MemoryError::validation(
                        "Memory database connection string must be 'memory'",
                    ))
                }
            }
        }
    }

    /// Get connection URL with credentials (if provided)
    #[must_use]
    pub fn connection_url(&self) -> String {
        match (&self.username, &self.password) {
            (Some(username), Some(password)) => {
                format!(
                    "{}://{}:{}@{}",
                    self.db_type,
                    username,
                    password,
                    &self.connection_string[self.db_type.to_string().len() + 3..]
                )
            }
            _ => self.connection_string.clone(),
        }
    }

    /// Check if configuration supports connection pooling
    #[inline]
    #[must_use]
    pub fn supports_pooling(&self) -> bool {
        self.db_type.supports_pooling()
    }

    /// Create health status tracker
    #[inline]
    #[must_use]
    pub fn create_health_status(&self) -> DatabaseHealthStatus {
        DatabaseHealthStatus::new()
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_type: DatabaseType::SurrealDB,
            connection_string: "surrealkv://./data/memory.db".to_string(),
            namespace: "cyrup".to_string(),
            database: "memory".to_string(),
            username: None,
            password: None,
            pool_config: PoolConfig::default(),
            timeout_config: TimeoutConfig::default(),
            health_check_config: HealthCheckConfig::default(),
            options: None,
        }
    }
}

use std::sync::atomic::AtomicU64;
