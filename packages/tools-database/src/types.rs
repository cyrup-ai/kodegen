//! Type definitions for database operations

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Database table column metadata
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableColumn {
    /// Column name
    pub column_name: String,

    /// Data type (e.g., "VARCHAR", "INTEGER", "TEXT")
    pub data_type: String,

    /// Whether column accepts NULL values ("YES" or "NO")
    pub is_nullable: String,

    /// Default value expression (if any)
    pub column_default: Option<String>,
}

/// Database table index metadata
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableIndex {
    /// Index name
    pub index_name: String,

    /// Columns included in the index
    pub column_names: Vec<String>,

    /// Whether index enforces uniqueness
    pub is_unique: bool,

    /// Whether this is the primary key index
    pub is_primary: bool,
}

/// Stored procedure or function metadata
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoredProcedure {
    /// Procedure or function name
    pub procedure_name: String,

    /// Type (e.g., "PROCEDURE", "FUNCTION")
    pub procedure_type: String,

    /// Programming language (e.g., "SQL", "PLPGSQL", "PLSQL")
    pub language: Option<String>,

    /// Parameter list definition
    pub parameter_list: Option<String>,

    /// Return type (for functions)
    pub return_type: Option<String>,

    /// Full procedure/function definition
    pub definition: Option<String>,
}

/// Options for SQL query execution
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteOptions {
    /// Maximum number of rows to return (None = unlimited)
    pub max_rows: Option<usize>,
}

/// SQL query execution result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SQLResult {
    /// Result rows as JSON values
    pub rows: Vec<serde_json::Value>,

    /// Total number of rows returned
    pub row_count: usize,
}

/// Database type for SQL dialect-specific handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatabaseType {
    Postgres,
    MySQL,
    MariaDB,
    SQLite,
    SqlServer, // Included for future sqlx mssql support
}

impl DatabaseType {
    /// Detect database type from connection URL scheme
    ///
    /// # Examples
    /// ```
    /// # use kodegen_tools_database::types::DatabaseType;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = DatabaseType::from_url("postgres://localhost/mydb")?;
    /// assert_eq!(db, DatabaseType::Postgres);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_url(url: &str) -> Result<Self, crate::error::DatabaseError> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(Self::Postgres)
        } else if url.starts_with("mysql://") {
            Ok(Self::MySQL)
        } else if url.starts_with("mariadb://") {
            Ok(Self::MariaDB)
        } else if url.starts_with("sqlite://") || url.starts_with("file:") {
            Ok(Self::SQLite)
        } else if url.starts_with("sqlserver://") || url.starts_with("mssql://") {
            Ok(Self::SqlServer)
        } else {
            Err(crate::error::DatabaseError::UnsupportedDatabase(format!(
                "Cannot determine database type from URL: {}",
                url
            )))
        }
    }
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "PostgreSQL"),
            Self::MySQL => write!(f, "MySQL"),
            Self::MariaDB => write!(f, "MariaDB"),
            Self::SQLite => write!(f, "SQLite"),
            Self::SqlServer => write!(f, "SQL Server"),
        }
    }
}
