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

    /// sqlx database error
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// SSH connection error
    #[error("SSH error: {0}")]
    Ssh(#[from] ssh2::Error),

    /// URL parsing error
    #[error("Invalid URL: {0}")]
    UrlParse(#[from] url::ParseError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convert DatabaseError to McpError
impl From<DatabaseError> for McpError {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::ConnectionError(msg) => {
                McpError::Network(format!("[DB Connection] {}", msg))
            }
            DatabaseError::QueryError(msg) => {
                McpError::Other(anyhow::anyhow!("[DB Query] {}", msg))
            }
            DatabaseError::SchemaNotFound(msg) => {
                McpError::ResourceNotFound(format!("[Schema] {}", msg))
            }
            DatabaseError::TableNotFound(msg) => {
                McpError::ResourceNotFound(format!("[Table] {}", msg))
            }
            DatabaseError::ReadOnlyViolation(msg) => {
                McpError::ReadOnlyViolation(format!("[DB] {}", msg))
            }
            DatabaseError::SSHTunnelError(msg) => {
                McpError::Network(format!("[SSH Tunnel] {}", msg))
            }
            DatabaseError::UnsupportedDatabase(msg) => {
                McpError::InvalidArguments(format!("[Unsupported DB] {}", msg))
            }
            DatabaseError::FeatureNotSupported(msg) => {
                McpError::InvalidArguments(format!("[Feature Not Supported] {}", msg))
            }
            DatabaseError::Sqlx(sqlx_err) => convert_sqlx_error(sqlx_err),
            DatabaseError::Ssh(ssh_err) => McpError::Network(format!("[SSH] {}", ssh_err)),
            DatabaseError::UrlParse(url_err) => {
                McpError::InvalidArguments(format!("[URL Parse] {}", url_err))
            }
            DatabaseError::Io(io_err) => McpError::Io(io_err),
        }
    }
}

/// Convert sqlx errors to McpError with detailed error handling
fn convert_sqlx_error(err: sqlx::Error) -> McpError {
    match err {
        sqlx::Error::Configuration(msg) => {
            McpError::InvalidArguments(format!("Database configuration error: {}", msg))
        }
        sqlx::Error::Database(db_err) => {
            McpError::Other(anyhow::anyhow!("Database error: {}", db_err))
        }
        sqlx::Error::Io(io_err) => McpError::Io(io_err),
        sqlx::Error::Tls(tls_err) => McpError::Network(format!("TLS error: {}", tls_err)),
        sqlx::Error::Protocol(msg) => McpError::Network(format!("Protocol error: {}", msg)),
        sqlx::Error::RowNotFound => {
            McpError::ResourceNotFound("No rows returned by query".to_string())
        }
        sqlx::Error::TypeNotFound { type_name } => {
            McpError::InvalidArguments(format!("Type not found: {}", type_name))
        }
        sqlx::Error::ColumnIndexOutOfBounds { index, len } => McpError::InvalidArguments(format!(
            "Column index {} out of bounds (len: {})",
            index, len
        )),
        sqlx::Error::ColumnNotFound(col) => {
            McpError::InvalidArguments(format!("Column not found: {}", col))
        }
        sqlx::Error::ColumnDecode { index, source } => McpError::Other(anyhow::anyhow!(
            "Failed to decode column {}: {}",
            index,
            source
        )),
        sqlx::Error::Decode(err) => McpError::Other(anyhow::anyhow!("Decode error: {}", err)),
        sqlx::Error::PoolTimedOut => McpError::Network("Connection pool timed out".to_string()),
        sqlx::Error::PoolClosed => McpError::Network("Connection pool closed".to_string()),
        sqlx::Error::WorkerCrashed => McpError::Other(anyhow::anyhow!("Database worker crashed")),
        _ => McpError::Other(anyhow::anyhow!("Database error: {}", err)),
    }
}
