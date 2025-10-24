//! Configuration for the memory system

use serde::{Deserialize, Serialize};

/// Main configuration for the memory system
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Database configuration
    pub database: DatabaseConfig,
    /// Vector store configuration
    pub vector_store: VectorStoreConfig,
    /// API configuration (optional)
    pub api: Option<APIConfig>,
    /// Cache configuration
    pub cache: CacheConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type
    pub db_type: DatabaseType,
    /// Database connection string
    pub connection_string: String,
    /// Database namespace
    pub namespace: String,
    /// Database name
    pub database: String,
    /// Username (optional)
    pub username: Option<String>,
    /// Password (optional)
    pub password: Option<String>,
    /// Connection pool size (optional)
    pub pool_size: Option<usize>,
    /// Additional options (optional)
    pub options: Option<serde_json::Value>,
}

/// Database types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DatabaseType {
    /// SurrealDB
    SurrealDB,
}

/// Vector store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    /// Vector store type
    pub store_type: VectorStoreType,
    /// Embedding model configuration
    pub embedding_model: EmbeddingModelConfig,
    /// Vector dimension
    pub dimension: usize,
    /// Connection string (optional, for external vector stores)
    pub connection_string: Option<String>,
    /// API key (optional, for hosted vector stores)
    pub api_key: Option<String>,
    /// Additional options (optional)
    pub options: Option<serde_json::Value>,
}

/// Vector store types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VectorStoreType {
    /// SurrealDB vector store
    SurrealDB,
    /// In-memory vector store (for testing)
    Memory,
    /// FAISS vector store
    FAISS,
    /// HNSW vector store
    HNSW,
}

/// Embedding model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelConfig {
    /// Model name
    pub registry_key: String,
    /// API key (optional)
    pub api_key: Option<String>,
    /// API base URL (optional)
    pub api_base: Option<String>,
    /// Additional options (optional)
    pub options: Option<serde_json::Value>,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIConfig {
    /// Host to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Enable CORS
    pub cors_enabled: bool,
    /// CORS allowed origins
    pub cors_allowed_origins: Vec<String>,
    /// Enable authentication
    pub auth_enabled: bool,
    /// Authentication type
    pub auth_type: Option<AuthType>,
    /// Rate limiting enabled
    pub rate_limit_enabled: bool,
    /// Rate limit requests per minute
    pub rate_limit_rpm: Option<usize>,
    /// Additional options (optional)
    pub options: Option<serde_json::Value>,
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthType {
    /// API key authentication
    ApiKey,
    /// JWT authentication
    JWT,
    /// OAuth authentication
    OAuth,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,
    /// Cache type
    pub cache_type: CacheType,
    /// Cache size (optional)
    pub size: Option<usize>,
    /// Cache TTL in seconds (optional)
    pub ttl: Option<u64>,
    /// Additional options (optional)
    pub options: Option<serde_json::Value>,
}

/// Cache types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CacheType {
    /// In-memory cache
    Memory,
    /// Redis cache
    Redis,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: LogLevel,
    /// Log file path (optional)
    pub file: Option<String>,
    /// Log to console
    pub console: bool,
    /// Additional options (optional)
    pub options: Option<serde_json::Value>,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warn,
    /// Error level
    Error,
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
            pool_size: Some(10),
            options: None,
        }
    }
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            store_type: VectorStoreType::SurrealDB,
            embedding_model: EmbeddingModelConfig::default(),
            dimension: 1536,
            connection_string: None,
            api_key: None,
            options: None,
        }
    }
}

impl Default for EmbeddingModelConfig {
    fn default() -> Self {
        Self {
            registry_key: "text-embedding-3-small".to_string(),
            api_key: None,
            api_base: None,
            options: None,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_type: CacheType::Memory,
            size: Some(10000),
            ttl: Some(3600),
            options: None,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            file: None,
            console: true,
            options: None,
        }
    }
}
