//! ExecuteSQL tool - Primary interface for SQL query execution
//!
//! Integrates read-only mode enforcement, row limiting, multi-statement support,
//! and transaction wrapping for consistent database operations.

use crate::{
    DatabaseType, apply_row_limit, error::DatabaseError, split_sql_statements,
    tools::timeout::execute_with_timeout, validate_readonly_sql,
};
use base64::Engine as _; // For base64 encoding of binary data
use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::database::{ExecuteSQLArgs, ExecuteSQLPromptArgs};
use kodegen_tools_config::ConfigManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use sqlx::AnyPool;
use sqlx::{Column, Row, TypeInfo};
use std::sync::Arc;
use std::time::Duration;

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
        // Execute query with timeout
        let pool = self.pool.clone();
        let sql_owned = sql.to_string();
        let rows = execute_with_timeout(
            &self.config,
            "db_query_timeout_secs",
            Duration::from_secs(60), // 60s default for data queries
            || {
                let pool = pool.clone();
                let sql = sql_owned.clone();
                async move { sqlx::query(&sql).fetch_all(&*pool).await }
            },
            &format!(
                "Executing SQL: {}",
                sql.chars().take(50).collect::<String>()
            ),
        )
        .await?;

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
    /// Returns partial results if execution fails partway through
    async fn execute_multi_transactional(&self, statements: &[String]) -> Result<Value, McpError> {
        // Begin transaction with timeout
        let pool = self.pool.clone();
        let mut tx = execute_with_timeout(
            &self.config,
            "db_query_timeout_secs",
            Duration::from_secs(30),
            || {
                let pool = pool.clone();
                async move { pool.begin().await }
            },
            "Starting transaction",
        )
        .await?;
        let mut all_rows = Vec::new();
        let mut executed_statements = 0;

        for (index, statement) in statements.iter().enumerate() {
            // Execute each statement with timeout (no retry - statements within transactions are atomic)
            let timeout_duration = self
                .config
                .get_value("db_query_timeout_secs")
                .and_then(|v| match v {
                    kodegen_tools_config::ConfigValue::Number(n) => {
                        Some(Duration::from_secs(n as u64))
                    }
                    _ => None,
                })
                .unwrap_or(Duration::from_secs(60));

            let rows_result = match tokio::time::timeout(
                timeout_duration,
                sqlx::query(statement).fetch_all(&mut *tx),
            )
            .await
            {
                Ok(Ok(rows)) => Ok(rows),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(sqlx::Error::PoolTimedOut),
            };

            match rows_result {
                Ok(rows) => {
                    executed_statements += 1;
                    if !rows.is_empty() {
                        for row in &rows {
                            let json_row =
                                row_to_json(row).map_err(|e| anyhow::anyhow!("{}", e))?;
                            all_rows.push(json_row);
                        }
                    }
                }
                Err(e) => {
                    // Rollback transaction
                    let _ = tx.rollback().await;

                    // Return error WITHOUT uncommitted data (transaction was rolled back)
                    return Ok(json!({
                        "success": false,
                        "error": format!("Statement {} failed: {}", index + 1, e),
                        "failed_statement": statement,
                        "failed_at_index": index + 1,
                        "executed_statements": executed_statements,
                        "total_statements": statements.len(),
                        "transaction_status": "rolled_back",
                        "note": "All changes were rolled back due to error. No data was committed."
                    }));
                }
            }
        }

        // Commit transaction with timeout (no retry - transaction commit is atomic)
        let timeout_duration = self
            .config
            .get_value("db_query_timeout_secs")
            .and_then(|v| match v {
                kodegen_tools_config::ConfigValue::Number(n) => Some(Duration::from_secs(n as u64)),
                _ => None,
            })
            .unwrap_or(Duration::from_secs(30));

        match tokio::time::timeout(timeout_duration, tx.commit()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                return Err(
                    DatabaseError::QueryError(format!("Transaction commit failed: {}", e)).into(),
                );
            }
            Err(_) => {
                return Err(
                    DatabaseError::QueryError("Transaction commit timed out".to_string()).into(),
                );
            }
        }

        Ok(json!({
            "rows": all_rows,
            "row_count": all_rows.len(),
            "executed_statements": executed_statements,
            "total_statements": statements.len()
        }))
    }

    /// Execute multiple SQL statements WITHOUT transaction
    /// Continues execution on error, collecting all results and errors
    async fn execute_multi_non_transactional(
        &self,
        statements: &[String],
    ) -> Result<Value, McpError> {
        let mut all_rows = Vec::new();
        let mut errors = Vec::new();
        let mut executed_statements = 0;

        for (index, statement) in statements.iter().enumerate() {
            // Execute each statement with timeout
            let pool = self.pool.clone();
            let statement_owned = statement.clone();
            let rows_result = execute_with_timeout(
                &self.config,
                "db_query_timeout_secs",
                Duration::from_secs(60),
                || {
                    let pool = pool.clone();
                    let stmt = statement_owned.clone();
                    async move { sqlx::query(&stmt).fetch_all(&*pool).await }
                },
                &format!(
                    "Executing: {}",
                    statement.chars().take(50).collect::<String>()
                ),
            )
            .await;

            match rows_result {
                Ok(rows) => {
                    executed_statements += 1;
                    if !rows.is_empty() {
                        for row in &rows {
                            let json_row =
                                row_to_json(row).map_err(|e| anyhow::anyhow!("{}", e))?;
                            all_rows.push(json_row);
                        }
                    }
                }
                Err(e) => {
                    // Record error but continue execution
                    errors.push(json!({
                        "statement_index": index + 1,
                        "statement": statement,
                        "error": e.to_string()
                    }));
                }
            }
        }

        Ok(json!({
            "rows": all_rows,
            "row_count": all_rows.len(),
            "executed_statements": executed_statements,
            "total_statements": statements.len(),
            "errors": errors,
            "has_errors": !errors.is_empty()
        }))
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Determine if statements contain write operations requiring transaction
fn should_use_transaction(statements: &[String], db_type: DatabaseType) -> bool {
    use crate::extract_first_keyword;

    statements.iter().any(|stmt| {
        if let Ok(keyword) = extract_first_keyword(stmt, db_type) {
            matches!(
                keyword.as_str(),
                "insert" | "update" | "delete" | "create" | "alter" | "drop" | "truncate"
            )
        } else {
            // If can't parse keyword, assume write for safety
            true
        }
    })
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
            "TEXT" | "VARCHAR" | "CHAR" | "STRING" | "BPCHAR" | "NAME" | "CITEXT" => {
                match row.try_get::<Option<String>, _>(ordinal) {
                    Ok(Some(s)) => Value::String(s),
                    Ok(None) => Value::Null,
                    Err(e) => {
                        return Err(DatabaseError::QueryError(format!(
                            "Failed to extract column '{}' as TEXT: {}",
                            name, e
                        )));
                    }
                }
            }
            // Integer types
            "INTEGER" | "INT" | "INT2" | "INT4" | "INT8" | "BIGINT" | "SMALLINT" | "MEDIUMINT"
            | "SERIAL" | "BIGSERIAL" => match row.try_get::<Option<i64>, _>(ordinal) {
                Ok(Some(v)) => json!(v),
                Ok(None) => Value::Null,
                Err(e) => {
                    return Err(DatabaseError::QueryError(format!(
                        "Failed to extract column '{}' as INTEGER: {}",
                        name, e
                    )));
                }
            },
            // Boolean types
            "BOOLEAN" | "BOOL" | "TINYINT(1)" => match row.try_get::<Option<bool>, _>(ordinal) {
                Ok(Some(b)) => Value::Bool(b),
                Ok(None) => Value::Null,
                Err(e) => {
                    return Err(DatabaseError::QueryError(format!(
                        "Failed to extract column '{}' as BOOLEAN: {}",
                        name, e
                    )));
                }
            },
            // Float types
            "REAL" | "FLOAT" | "FLOAT4" | "FLOAT8" | "DOUBLE" | "DOUBLE PRECISION" => {
                match row.try_get::<Option<f64>, _>(ordinal) {
                    Ok(Some(v)) => json!(v),
                    Ok(None) => Value::Null,
                    Err(e) => {
                        return Err(DatabaseError::QueryError(format!(
                            "Failed to extract column '{}' as FLOAT: {}",
                            name, e
                        )));
                    }
                }
            }
            // DECIMAL/NUMERIC - sqlx::any doesn't support these types
            // Try as f64 first (may lose precision for very large numbers)
            "NUMERIC" | "DECIMAL" | "NUMBER" => {
                match row.try_get::<Option<f64>, _>(ordinal) {
                    Ok(Some(v)) => json!(v),
                    Ok(None) => Value::Null,
                    Err(_) => {
                        // If f64 fails, try as string
                        match row.try_get::<Option<String>, _>(ordinal) {
                            Ok(Some(s)) => Value::String(s),
                            Ok(None) => Value::Null,
                            Err(e) => {
                                return Err(DatabaseError::QueryError(format!(
                                    "Failed to extract column '{}' as DECIMAL (tried f64 and string): {}. \
                                 Consider using CAST({} AS TEXT) in your query.",
                                    name, e, name
                                )));
                            }
                        }
                    }
                }
            }
            // JSON types - parse as serde_json::Value
            "JSON" | "JSONB" => {
                match row.try_get::<Option<String>, _>(ordinal) {
                    Ok(Some(json_str)) => {
                        serde_json::from_str(&json_str).unwrap_or_else(|e| {
                            log::warn!("Failed to parse JSON column '{}': {}", name, e);
                            Value::String(json_str) // Fallback to raw string
                        })
                    }
                    Ok(None) => Value::Null,
                    Err(e) => {
                        return Err(DatabaseError::QueryError(format!(
                            "Failed to extract column '{}' as JSON: {}",
                            name, e
                        )));
                    }
                }
            }
            // Binary types - encode as base64 string
            "BYTEA" | "BLOB" | "BINARY" | "VARBINARY" => {
                match row.try_get::<Option<Vec<u8>>, _>(ordinal) {
                    Ok(Some(bytes)) => {
                        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        json!({
                            "type": "base64",
                            "data": encoded
                        })
                    }
                    Ok(None) => Value::Null,
                    Err(e) => {
                        return Err(DatabaseError::QueryError(format!(
                            "Failed to extract column '{}' as BYTEA: {}",
                            name, e
                        )));
                    }
                }
            }
            // Date/Time types - extract as strings
            "TIMESTAMP" | "TIMESTAMPTZ" | "DATETIME" | "DATE" | "TIME" | "INTERVAL" => {
                match row.try_get::<Option<String>, _>(ordinal) {
                    Ok(Some(s)) => Value::String(s),
                    Ok(None) => Value::Null,
                    Err(e) => {
                        return Err(DatabaseError::QueryError(format!(
                            "Failed to extract column '{}' as {}: {}",
                            name, type_name, e
                        )));
                    }
                }
            }
            // UUID - extract as string
            "UUID" => match row.try_get::<Option<String>, _>(ordinal) {
                Ok(Some(s)) => Value::String(s),
                Ok(None) => Value::Null,
                Err(e) => {
                    return Err(DatabaseError::QueryError(format!(
                        "Failed to extract column '{}' as UUID: {}",
                        name, e
                    )));
                }
            },
            // Fallback for unsupported types
            _ => {
                return Err(DatabaseError::QueryError(format!(
                    "Unsupported column type '{}' for column '{}'. \
                     Supported types: TEXT, VARCHAR, INTEGER, BIGINT, BOOLEAN, REAL, FLOAT, DOUBLE, \
                     NUMERIC, DECIMAL, JSON, JSONB, BYTEA, BLOB, TIMESTAMP, DATE, TIME, UUID. \
                     Consider casting this column in your query: CAST({} AS TEXT)",
                    type_name, name, name
                )));
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
         \n\n\
         MULTI-STATEMENT BEHAVIOR:\n\
         - Write operations (INSERT/UPDATE/DELETE/CREATE/ALTER/DROP) use transactions\n\
         - Read operations (SELECT/EXPLAIN/SHOW) execute independently without transaction\n\
         - On transactional error: returns error details without data (all changes rolled back)\n\
         - On non-transactional error: returns committed data plus errors array\n\
         \n\
         Returns query results as JSON with:\n\
         - rows: array of result rows\n\
         - row_count: number of rows returned\n\
         - errors: array of errors (if any failures in non-transactional mode)\n\
         \n\
         Supports read-only mode enforcement and automatic row limiting."
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
        let statements = split_sql_statements(&sql, db_type)
            .map_err(|e| anyhow::anyhow!("SQL parse error: {}", e))?;

        // 6. Execute single or multi-statement
        if statements.len() == 1 {
            self.execute_single(&statements[0]).await
        } else {
            // Route based on statement types
            if should_use_transaction(&statements, db_type) {
                self.execute_multi_transactional(&statements).await
            } else {
                self.execute_multi_non_transactional(&statements).await
            }
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
