//! Database tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// GET STORED PROCEDURES
// ============================================================================

/// Arguments for get_stored_procedures tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetStoredProceduresArgs {
    /// Schema name (optional, uses default if not provided)
    #[serde(default)]
    pub schema: Option<String>,

    /// Include detailed information (parameters, return type, definition)
    /// Warning: definition can be large for complex procedures
    #[serde(default)]
    pub include_details: bool,
}

/// Prompt arguments for get_stored_procedures tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetStoredProceduresPromptArgs {}

// ============================================================================
// GET TABLE INDEXES
// ============================================================================

/// Arguments for get_table_indexes tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTableIndexesArgs {
    /// Table name to inspect
    pub table: String,

    /// Schema name (optional, uses default if not provided)
    #[serde(default)]
    pub schema: Option<String>,
}

/// Prompt arguments for get_table_indexes tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTableIndexesPromptArgs {}

// ============================================================================
// LIST SCHEMAS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSchemasArgs {
    // Empty - no parameters needed
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSchemasPromptArgs {}

// ============================================================================
// LIST TABLES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTablesArgs {
    /// Optional schema name. If not provided, uses default schema:
    /// - PostgreSQL: 'public'
    /// - MySQL/MariaDB: Current database (from DATABASE())
    /// - SQLite: 'main'
    /// - SQL Server: 'dbo'
    #[serde(default)]
    pub schema: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTablesPromptArgs {}

// ============================================================================
// GET POOL STATS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPoolStatsArgs {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetPoolStatsPromptArgs {}

// ============================================================================
// GET TABLE SCHEMA
// ============================================================================

/// Arguments for get_table_schema tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTableSchemaArgs {
    /// Table name to inspect
    pub table: String,

    /// Schema name (optional, uses default if not provided)
    /// PostgreSQL: defaults to "public"
    /// MySQL/MariaDB: defaults to current DATABASE()
    /// SQLite: defaults to "main"
    /// SQL Server: defaults to "dbo"
    #[serde(default)]
    pub schema: Option<String>,
}

/// Prompt arguments for get_table_schema tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTableSchemaPromptArgs {}

// ============================================================================
// EXECUTE SQL
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteSQLArgs {
    /// SQL query or multiple SQL statements (separated by semicolons)
    /// Multi-statement queries are executed within a transaction for consistency.
    pub sql: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteSQLPromptArgs {
    /// Optional: database type to focus examples on (postgres, mysql, sqlite, etc.)
    #[serde(default)]
    pub database_type: Option<String>,
}
