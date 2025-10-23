//! Helper functions for database tools

use crate::error::DatabaseError;
use crate::schema_queries::get_default_schema;
use crate::tools::timeout::execute_with_timeout;
use crate::types::DatabaseType;
use kodegen_tools_config::ConfigManager;
use sqlx::{AnyPool, Row};
use std::time::Duration;

/// Resolve schema name: use provided value or query for default
///
/// For most databases, uses get_default_schema() from DBTOOL_5.
/// For MySQL (which has no static default), executes DATABASE() query with timeout protection.
///
/// # Arguments
///
/// * `db_type` - The database type
/// * `pool` - The database connection pool
/// * `config` - Configuration manager for timeout settings
///
/// # Returns
///
/// * `Ok(String)` - The resolved schema name
/// * `Err(DatabaseError)` - If schema cannot be resolved
///
/// # Examples
///
/// ```rust,no_run
/// use kodegen_tools_database::tools::helpers::resolve_schema_default;
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_config::ConfigManager;
/// use sqlx::AnyPool;
///
/// # async fn example(pool: &AnyPool, config: &ConfigManager) -> Result<(), Box<dyn std::error::Error>> {
/// let schema = resolve_schema_default(DatabaseType::Postgres, pool, config).await?;
/// assert_eq!(schema, "public");
/// # Ok(())
/// # }
/// ```
pub async fn resolve_schema_default(
    db_type: DatabaseType,
    pool: &AnyPool,
    config: &ConfigManager,
) -> Result<String, DatabaseError> {
    // Check if there's a static default
    if let Some(default) = get_default_schema(db_type) {
        return Ok(default.to_string());
    }

    // MySQL case: query DATABASE() with timeout protection
    if matches!(db_type, DatabaseType::MySQL | DatabaseType::MariaDB) {
        let pool_clone = pool.clone();
        let row = execute_with_timeout(
            config,
            "db_metadata_query_timeout_secs",
            Duration::from_secs(10),
            || {
                let pool = pool_clone.clone();
                async move {
                    sqlx::query("SELECT DATABASE() as db")
                        .fetch_one(&pool)
                        .await
                }
            },
            "Getting current database name",
        )
        .await
        .map_err(|e| DatabaseError::QueryError(format!("Failed to get current database: {}", e)))?;

        let db_name: Option<String> = row.try_get("db").map_err(|e| {
            DatabaseError::QueryError(format!("Failed to parse database name: {}", e))
        })?;

        return db_name.ok_or_else(|| {
            DatabaseError::QueryError(
                "No database selected. Use 'USE database_name' first.".to_string(),
            )
        });
    }

    // No default available and not MySQL
    Err(DatabaseError::QueryError(format!(
        "No default schema for {}",
        db_type
    )))
}
