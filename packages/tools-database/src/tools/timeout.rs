//! Query timeout utilities for database operations

use crate::error::DatabaseError;
use kodegen_mcp_tool::error::McpError;
use kodegen_tools_config::ConfigManager;
use std::time::Duration;
use tokio::time::timeout;

/// Execute a database query with timeout protection and automatic retry
///
/// Wraps any async database operation with tokio::time::timeout and retries
/// connection errors automatically with exponential backoff.
///
/// # Arguments
///
/// * `config` - ConfigManager to read timeout and retry configuration
/// * `config_key` - Key to read timeout value (e.g., "db_query_timeout_secs")
/// * `default_timeout` - Fallback timeout if config key not set
/// * `query_fn` - Closure that returns the async query operation to execute
/// * `operation_description` - Human-readable description for error messages
///
/// # Returns
///
/// * `Ok(T)` - Query result on success
/// * `Err(McpError)` - Timeout or query execution error after retries exhausted
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
///     || sqlx::query("SELECT * FROM users").fetch_all(&pool),
///     "Fetching users",
/// ).await?;
/// ```
pub async fn execute_with_timeout<T, F, Fut>(
    config: &ConfigManager,
    config_key: &str,
    default_timeout: Duration,
    query_fn: F,
    operation_description: &str,
) -> Result<T, McpError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    // Read timeout and retry configuration
    let timeout_duration = config
        .get_value(config_key)
        .and_then(|v| match v {
            kodegen_tools_config::ConfigValue::Number(n) => Some(Duration::from_secs(n as u64)),
            _ => None,
        })
        .unwrap_or(default_timeout);
    
    let max_retries = config
        .get_value("db_max_retries")
        .and_then(|v| match v {
            kodegen_tools_config::ConfigValue::Number(n) => Some(n as u32),
            _ => None,
        })
        .unwrap_or(2); // Retry twice by default (3 total attempts)
    
    let mut last_error = None;
    
    for attempt in 0..=max_retries {
        // Execute with timeout
        match timeout(timeout_duration, query_fn()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(sqlx_err)) => {
                // Check if error is retryable
                if is_connection_error(&sqlx_err) && attempt < max_retries {
                    log::warn!(
                        "Connection error on attempt {}/{}: {}. Retrying...",
                        attempt + 1,
                        max_retries + 1,
                        sqlx_err
                    );
                    last_error = Some(sqlx_err);
                    
                    // Configurable exponential backoff with jitter
                    let base_backoff_ms = config
                        .get_value("db_retry_backoff_ms")
                        .and_then(|v| match v {
                            kodegen_tools_config::ConfigValue::Number(n) => Some(n as u64),
                            _ => None,
                        })
                        .unwrap_or(500); // Default 500ms, not 100ms
                    
                    let max_backoff_ms = config
                        .get_value("db_max_backoff_ms")
                        .and_then(|v| match v {
                            kodegen_tools_config::ConfigValue::Number(n) => Some(n as u64),
                            _ => None,
                        })
                        .unwrap_or(5000); // Default 5 seconds cap
                    
                    // Add jitter to prevent thundering herd
                    let jitter = rand::random::<u64>() % 100; // 0-100ms random jitter
                    
                    // Calculate backoff with exponential growth and cap
                    let backoff_ms = (base_backoff_ms * 2_u64.pow(attempt))
                        .min(max_backoff_ms)
                        + jitter;
                    
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    continue;
                } else {
                    // Non-retryable error or max retries exhausted
                    return Err(DatabaseError::QueryError(format!(
                        "{}: {}",
                        operation_description, sqlx_err
                    ))
                    .into());
                }
            }
            Err(_elapsed) => {
                // Timeout occurred
                if attempt < max_retries {
                    log::warn!(
                        "Timeout on attempt {}/{}. Retrying...",
                        attempt + 1,
                        max_retries + 1
                    );
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                } else {
                    return Err(DatabaseError::QueryError(format!(
                        "{} timed out after {:?}. \
                         The operation may be too slow or the database may be overloaded.\n\
                         Suggestions:\n\
                         • For SELECT queries: Add WHERE clause, LIMIT, or indexes\n\
                         • For UPDATE/DELETE: Add WHERE clause to reduce rows affected\n\
                         • Check database performance with EXPLAIN\n\
                         • Increase timeout via config: {} = <seconds>",
                        operation_description, timeout_duration, config_key
                    ))
                    .into());
                }
            }
        }
    }
    
    // If we exhausted retries
    Err(DatabaseError::QueryError(format!(
        "{}: Max retries ({}) exceeded. Last error: {}",
        operation_description,
        max_retries,
        last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown".to_string())
    ))
    .into())
}

/// Check if a sqlx error is connection-related and retryable
fn is_connection_error(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::PoolClosed
        | sqlx::Error::PoolTimedOut
        | sqlx::Error::Io(_) => true,
        sqlx::Error::Database(db_err) => {
            let msg = db_err.message().to_lowercase();
            msg.contains("connection")
                || msg.contains("broken pipe")
                || msg.contains("reset by peer")
                || msg.contains("closed")
        }
        _ => false,
    }
}
