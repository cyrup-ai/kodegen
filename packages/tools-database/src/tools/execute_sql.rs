//! ExecuteSQL tool - Primary interface for SQL query execution
//!
//! Integrates read-only mode enforcement, row limiting, multi-statement support,
//! and transaction wrapping for consistent database operations.

use crate::{
    DatabaseType, apply_row_limit, error::DatabaseError, split_sql_statements,
    validate_readonly_sql,
};
use anyhow::Context;
use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_tools_config::ConfigManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::AnyPool;
use sqlx::{Column, Row, TypeInfo};
use std::sync::Arc;

// ============================================================================
// TOOL ARGUMENTS
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

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct ExecuteSQLTool {
    pool: Arc<AnyPool>,
    config: ConfigManager,
    db_type: DatabaseType, // Store database type for validation/limiting
}

impl ExecuteSQLTool {
    /// Create a new ExecuteSQL tool instance
    ///
    /// # Errors
    /// Returns error if connection_url cannot be parsed to determine database type
    pub fn new(
        pool: Arc<AnyPool>,
        config: ConfigManager,
        connection_url: &str,
    ) -> Result<Self, McpError> {
        let db_type = DatabaseType::from_url(connection_url)
            .map_err(|e| anyhow::anyhow!("Failed to determine database type: {}", e))?;
        Ok(Self {
            pool,
            config,
            db_type,
        })
    }

    /// Get database type from stored field
    fn get_database_type(&self) -> Result<DatabaseType, McpError> {
        Ok(self.db_type)
    }

    /// Execute a single SQL statement
    async fn execute_single(&self, sql: &str) -> Result<Value, McpError> {
        // Execute query
        let rows = sqlx::query(sql)
            .fetch_all(&*self.pool)
            .await
            .context("SQL execution failed")?;

        // Convert rows to JSON
        let json_rows: Result<Vec<Value>, _> = rows
            .iter()
            .map(|row| row_to_json(row).map_err(|e| anyhow::anyhow!("{}", e)))
            .collect();

        let json_rows = json_rows?;
        let row_count = json_rows.len();

        Ok(json!({
            "rows": json_rows,
            "row_count": row_count
        }))
    }

    /// Execute multiple SQL statements within a transaction
    async fn execute_multi(&self, statements: &[String]) -> Result<Value, McpError> {
        // Begin transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction")?;

        let mut all_rows = Vec::new();

        // Execute each statement in sequence
        for statement in statements {
            let rows = sqlx::query(statement)
                .fetch_all(&mut *tx)
                .await
                .context("SQL execution failed. Transaction rolled back.")?;

            // Collect rows from SELECT/WITH/EXPLAIN statements
            if !rows.is_empty() {
                for row in &rows {
                    let json_row = row_to_json(row).map_err(|e| anyhow::anyhow!("{}", e))?;
                    all_rows.push(json_row);
                }
            }
        }

        // Commit transaction
        tx.commit().await.context("Failed to commit transaction")?;

        let row_count = all_rows.len();

        Ok(json!({
            "rows": all_rows,
            "row_count": row_count
        }))
    }
}

// ============================================================================
// ROW TO JSON CONVERSION
// ============================================================================

/// Convert a sqlx Row to a JSON object
///
/// Dynamically extracts column names and values, converting to appropriate JSON types.
/// Handles NULL values gracefully by returning Value::Null.
///
/// # Type Name Variations
/// Type names vary by database:
/// - PostgreSQL: TEXT, INT4, INT8, BOOL, FLOAT8, etc.
/// - MySQL: VARCHAR, INT, BIGINT, TINYINT, DOUBLE, etc.
/// - SQLite: TEXT, INTEGER, REAL, BLOB, etc.
fn row_to_json(row: &sqlx::any::AnyRow) -> Result<Value, DatabaseError> {
    let mut map = serde_json::Map::new();

    for column in row.columns() {
        let ordinal = column.ordinal();
        let name = column.name().to_string();
        let type_name = column.type_info().name();

        // Match on database type names
        let value = match type_name {
            // Text types (most databases)
            "TEXT" | "VARCHAR" | "CHAR" | "STRING" | "BPCHAR" => row
                .try_get::<Option<String>, _>(ordinal)
                .ok()
                .flatten()
                .map(Value::String)
                .unwrap_or(Value::Null),
            // Integer types
            "INTEGER" | "INT" | "INT2" | "INT4" | "INT8" | "BIGINT" | "SMALLINT" | "MEDIUMINT" => {
                row.try_get::<Option<i64>, _>(ordinal)
                    .ok()
                    .flatten()
                    .map(|v| json!(v))
                    .unwrap_or(Value::Null)
            }
            // Boolean types
            "BOOLEAN" | "BOOL" | "TINYINT(1)" => row
                .try_get::<Option<bool>, _>(ordinal)
                .ok()
                .flatten()
                .map(Value::Bool)
                .unwrap_or(Value::Null),
            // Float types
            "REAL" | "FLOAT" | "FLOAT4" | "FLOAT8" | "DOUBLE" | "NUMERIC" | "DECIMAL" => row
                .try_get::<Option<f64>, _>(ordinal)
                .ok()
                .flatten()
                .map(|v| json!(v))
                .unwrap_or(Value::Null),
            // Fallback for unsupported types
            _ => {
                // Log warning but don't fail
                log::warn!(
                    "Unsupported column type '{}' for column '{}'",
                    type_name,
                    name
                );
                json!(format!("UNSUPPORTED_TYPE: {}", type_name))
            }
        };

        map.insert(name, value);
    }

    Ok(Value::Object(map))
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for ExecuteSQLTool {
    type Args = ExecuteSQLArgs;
    type PromptArgs = ExecuteSQLPromptArgs;

    fn name() -> &'static str {
        "execute_sql"
    }

    fn description() -> &'static str {
        "Execute SQL query or multiple SQL statements (separated by semicolons). \
         Supports read-only mode enforcement and automatic row limiting. \
         Returns query results as JSON array with row count. \
         Multi-statement queries are executed within a transaction for consistency. \
         Supported operations: SELECT, INSERT, UPDATE, DELETE, CREATE, DROP, ALTER (based on configuration)."
    }

    fn read_only() -> bool {
        false // Can execute write operations (based on config)
    }

    fn destructive() -> bool {
        true // Can delete/modify data
    }

    fn idempotent() -> bool {
        false // Multiple executions have different effects
    }

    fn open_world() -> bool {
        true // Network database connection
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // 1. Get configuration
        let readonly = self
            .config
            .get_value("readonly")
            .and_then(|v| match v {
                kodegen_tools_config::ConfigValue::Boolean(b) => Some(b),
                _ => None,
            })
            .unwrap_or(false);

        let max_rows = self.config.get_value("max_rows").and_then(|v| match v {
            kodegen_tools_config::ConfigValue::Number(n) => Some(n as usize),
            _ => None,
        });

        // 2. Get database type
        let db_type = self.get_database_type()?;

        // 3. Validate read-only mode if enabled
        if readonly {
            validate_readonly_sql(&args.sql, db_type)
                .map_err(|e| anyhow::anyhow!("Read-only violation: {}", e))?;
        }

        // 4. Apply row limiting if configured
        let sql = if let Some(max_rows) = max_rows {
            apply_row_limit(&args.sql, max_rows, db_type)
                .map_err(|e| anyhow::anyhow!("Row limit failed: {}", e))?
        } else {
            args.sql.clone()
        };

        // 5. Split into statements
        let statements = split_sql_statements(&sql);

        // 6. Execute single or multi-statement
        if statements.len() == 1 {
            self.execute_single(&statements[0]).await
        } else {
            self.execute_multi(&statements).await
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "database_type".to_string(),
            title: None,
            description: Some(
                "Database type to show examples for (postgres, mysql, sqlite)".to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use execute_sql to query and modify a database?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The execute_sql tool executes SQL queries and returns results as JSON:\n\n\
                     BASIC USAGE:\n\
                     1. Single query:\n   \
                        execute_sql({\"sql\": \"SELECT * FROM users LIMIT 10\"})\n   \
                        Returns: {\"rows\": [{...}, {...}], \"row_count\": 10}\n\n\
                     2. Multi-statement (uses transaction):\n   \
                        execute_sql({\"sql\": \"BEGIN; INSERT INTO logs VALUES (1, 'test'); COMMIT;\"})\n   \
                        All statements execute atomically - rolls back on error\n\n\
                     3. Data modification:\n   \
                        execute_sql({\"sql\": \"UPDATE users SET status = 'active' WHERE id = 5\"})\n\n\
                     FEATURES:\n\
                     • Read-only mode: When enabled, only SELECT/SHOW/DESCRIBE/EXPLAIN allowed\n\
                     • Row limiting: Automatically applied if max_rows configured\n\
                     • Transactions: Multi-statement queries execute in transaction for consistency\n\
                     • NULL handling: NULL values returned as JSON null\n\n\
                     EXAMPLES BY DATABASE:\n\
                     • PostgreSQL: Supports CTEs, EXPLAIN ANALYZE, JSON types\n\
                     • MySQL: Use SHOW TABLES, DESCRIBE table_name for schema\n\
                     • SQLite: Use .schema or SELECT * FROM sqlite_master\n\n\
                     BEST PRACTICES:\n\
                     • Use LIMIT in SELECT queries to avoid large result sets\n\
                     • Wrap multiple statements in explicit transaction for clarity\n\
                     • Check row_count in response to verify operations\n\
                     • Use schema tools (get_tables, get_table_schema) before querying",
                ),
            },
        ])
    }
}
