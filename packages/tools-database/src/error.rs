//! Error types for database operations

use kodegen_mcp_tool::error::McpError;
use thiserror::Error;

/// Database operation errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Failed to connect to database
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// SQL query execution failed
    #[error("Query error: {0}")]
    QueryError(String),

    /// Database schema not found
    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    /// Database table not found
    #[error("Table not found: {0}")]
    TableNotFound(String),

    /// Attempted write operation in read-only mode
    #[error("Read-only violation: {0}")]
    ReadOnlyViolation(String),

    /// SSH tunnel establishment failed
    #[error("SSH tunnel error: {0}")]
    SSHTunnelError(String),

    /// Database type not supported
    #[error("Unsupported database: {0}")]
    UnsupportedDatabase(String),

    /// Feature not supported for this database
    #[error("Feature not supported: {0}")]
    FeatureNotSupported(String),
}

/// Convert DatabaseError to McpError via anyhow
impl From<DatabaseError> for McpError {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::ReadOnlyViolation(msg) => McpError::ReadOnlyViolation(msg),
            DatabaseError::SchemaNotFound(msg) | DatabaseError::TableNotFound(msg) => {
                McpError::ResourceNotFound(msg)
            }
            _ => McpError::Other(anyhow::Error::new(err)),
        }
    }
}

/// Convert sqlx errors to McpError
impl From<sqlx::Error> for McpError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Configuration(msg) => {
                McpError::InvalidArguments(format!("Database configuration error: {}", msg))
            }
            sqlx::Error::Database(db_err) => {
                McpError::Other(anyhow::anyhow!("Database error: {}", db_err))
            }
            sqlx::Error::Io(io_err) => McpError::Io(io_err),
            sqlx::Error::Tls(tls_err) => {
                McpError::Network(format!("TLS error: {}", tls_err))
            }
            sqlx::Error::Protocol(msg) => {
                McpError::Network(format!("Protocol error: {}", msg))
            }
            sqlx::Error::RowNotFound => {
                McpError::ResourceNotFound("No rows returned by query".to_string())
            }
            sqlx::Error::TypeNotFound { type_name } => {
                McpError::InvalidArguments(format!("Type not found: {}", type_name))
            }
            sqlx::Error::ColumnIndexOutOfBounds { index, len } => {
                McpError::InvalidArguments(format!(
                    "Column index {} out of bounds (len: {})",
                    index, len
                ))
            }
            sqlx::Error::ColumnNotFound(col) => {
                McpError::InvalidArguments(format!("Column not found: {}", col))
            }
            sqlx::Error::ColumnDecode { index, source } => McpError::Other(anyhow::anyhow!(
                "Failed to decode column {}: {}",
                index,
                source
            )),
            sqlx::Error::Decode(err) => {
                McpError::Other(anyhow::anyhow!("Decode error: {}", err))
            }
            sqlx::Error::PoolTimedOut => {
                McpError::Network("Connection pool timed out".to_string())
            }
            sqlx::Error::PoolClosed => {
                McpError::Network("Connection pool closed".to_string())
            }
            sqlx::Error::WorkerCrashed => {
                McpError::Other(anyhow::anyhow!("Database worker crashed"))
            }
            _ => McpError::Other(anyhow::anyhow!("Database error: {}", err)),
        }
    }
}

/// Convert SSH errors to McpError
impl From<ssh2::Error> for McpError {
    fn from(err: ssh2::Error) -> Self {
        McpError::Network(format!("SSH error: {}", err))
    }
}

/// Convert URL parse errors to McpError
impl From<url::ParseError> for McpError {
    fn from(err: url::ParseError) -> Self {
        McpError::InvalidArguments(format!("Invalid URL: {}", err))
    }
}
