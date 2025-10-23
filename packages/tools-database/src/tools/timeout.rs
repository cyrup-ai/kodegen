//! Query timeout utilities for database operations

use crate::error::DatabaseError;
use kodegen_mcp_tool::error::McpError;
use kodegen_tools_config::ConfigManager;
use std::time::Duration;
use tokio::time::timeout;

/// Execute a database query with timeout protection
///
/// Wraps any async database operation with tokio::time::timeout, providing
/// consistent error messages and configurable timeout durations.
///
/// # Arguments
///
/// * `config` - ConfigManager to read timeout configuration
/// * `config_key` - Key to read timeout value (e.g., "db_query_timeout_secs")
/// * `default_timeout` - Fallback timeout if config key not set
/// * `query_future` - The async query operation to execute
/// * `operation_description` - Human-readable description for error messages
///
/// # Returns
///
/// * `Ok(T)` - Query result on success
/// * `Err(McpError)` - Timeout or query execution error
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
/// 
/// let result = execute_with_timeout(
///     &config_manager,
///     "db_query_timeout_secs",
///     Duration::from_secs(60),
///     sqlx::query("SELECT * FROM users").fetch_all(&pool),
///     "Fetching users",
/// ).await?;
/// ```
pub async fn execute_with_timeout<T, F>(
    config: &ConfigManager,
    config_key: &str,
    default_timeout: Duration,
    query_future: F,
    operation_description: &str,
) -> Result<T, McpError>
where
    F: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    // Read timeout from config
    let timeout_duration = config
        .get_value(config_key)
        .and_then(|v| match v {
            kodegen_tools_config::ConfigValue::Number(n) => Some(Duration::from_secs(n as u64)),
            _ => None,
        })
        .unwrap_or(default_timeout);

    // Execute with timeout
    match timeout(timeout_duration, query_future).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(sqlx_err)) => {
            // Query failed (not timeout)
            Err(DatabaseError::QueryError(format!(
                "{}: {}",
                operation_description, sqlx_err
            ))
            .into())
        }
        Err(_elapsed) => {
            // Timeout occurred
            Err(DatabaseError::QueryError(format!(
                "{} timed out after {:?}. \
                 The operation may be too slow or the database may be overloaded.\n\
                 Suggestions:\n\
                 • For SELECT queries: Add WHERE clause, LIMIT, or indexes\n\
                 • For UPDATE/DELETE: Add WHERE clause to reduce rows affected\n\
                 • Check database performance with EXPLAIN\n\
                 • Increase timeout via config: {} = <seconds>",
                operation_description, timeout_duration, config_key
            ))
            .into())
        }
    }
}
