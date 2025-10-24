//! Get table indexes tool

use crate::schema_queries::get_indexes_query;
use crate::tools::helpers::resolve_schema_default;
use crate::tools::timeout::execute_with_timeout;
use crate::types::{DatabaseType, TableIndex};
use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::database::{GetTableIndexesArgs, GetTableIndexesPromptArgs};
use kodegen_tools_config::ConfigManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use std::time::Duration;

/// Tool for retrieving table index information
#[derive(Clone)]
pub struct GetTableIndexesTool {
    pool: Arc<AnyPool>,
    db_type: DatabaseType,
    config: Arc<ConfigManager>,
}

impl GetTableIndexesTool {
    /// Create a new GetTableIndexesTool instance
    pub fn new(
        pool: Arc<AnyPool>,
        connection_url: &str,
        config: Arc<ConfigManager>,
    ) -> Result<Self, McpError> {
        let db_type = DatabaseType::from_url(connection_url)
            .map_err(|e| McpError::Other(anyhow::anyhow!("Invalid database URL: {}", e)))?;
        Ok(Self {
            pool,
            db_type,
            config,
        })
    }
}

impl Tool for GetTableIndexesTool {
    type Args = GetTableIndexesArgs;
    type PromptArgs = GetTableIndexesPromptArgs;

    fn name() -> &'static str {
        "get_table_indexes"
    }

    fn description() -> &'static str {
        "Get index information for a table including index names, columns, uniqueness, \
         and primary key status. Use this to understand which columns are indexed for \
         query optimization. Returns array of indexes with metadata."
    }

    fn read_only() -> bool {
        true // Only reads metadata
    }

    fn open_world() -> bool {
        true // Queries external database
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Use stored database type
        let db_type = self.db_type;

        // Resolve schema
        let schema = match args.schema {
            Some(s) => s,
            None => resolve_schema_default(db_type, &self.pool, &self.config).await?,
        };

        // Get query from helper (DBTOOL_5) - validation enforced for SQLite
        let (query, params) = get_indexes_query(db_type, &schema, &args.table)?;

        // Execute with parameters and timeout
        let pool = self.pool.clone();
        let query_owned = query.clone();
        let params_owned = params.clone();
        let rows = execute_with_timeout(
            &self.config,
            "db_metadata_query_timeout_secs",
            Duration::from_secs(10), // 10s default for metadata
            || {
                let pool = pool.clone();
                let query = query_owned.clone();
                let params = params_owned.clone();
                async move {
                    let mut q = sqlx::query(&query);
                    for param in &params {
                        q = q.bind(param);
                    }
                    q.fetch_all(&*pool).await
                }
            },
            "Getting table indexes",
        )
        .await?;

        // Parse into TableIndex structs
        let mut indexes = Vec::new();

        match db_type {
            DatabaseType::MySQL | DatabaseType::MariaDB => {
                // MySQL: Single query returns all index-column rows
                // Group by index_name in Rust to avoid GROUP_CONCAT truncation
                use std::collections::HashMap;

                // HashMap: index_name -> (columns, is_unique, is_primary)
                let mut index_map: HashMap<String, (Vec<String>, bool, bool)> = HashMap::new();

                for row in rows.iter() {
                    let index_name: String = row.try_get("index_name").unwrap_or_default();
                    let column_name: String = row.try_get("column_name").unwrap_or_default();
                    let is_unique: bool = row.try_get("is_unique").unwrap_or(false);
                    let is_primary: bool = row.try_get("is_primary").unwrap_or(false);

                    index_map
                        .entry(index_name)
                        .or_insert_with(|| (Vec::new(), is_unique, is_primary))
                        .0
                        .push(column_name);
                }

                // Convert HashMap to Vec<TableIndex>
                for (index_name, (column_names, is_unique, is_primary)) in index_map {
                    indexes.push(TableIndex {
                        index_name,
                        column_names,
                        is_unique,
                        is_primary,
                    });
                }
            }
            _ => {
                // PostgreSQL, SQLite, SQL Server: Use original single-query approach
                // (PostgreSQL uses array_agg, no truncation issue)
                for row in rows.iter() {
                    let cols_str: String = row.try_get("column_names").unwrap_or_default();
                    let column_names: Vec<String> = cols_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    indexes.push(TableIndex {
                        index_name: row.try_get("index_name").unwrap_or_default(),
                        column_names,
                        is_unique: row.try_get("is_unique").unwrap_or(false),
                        is_primary: row.try_get("is_primary").unwrap_or(false),
                    });
                }
            }
        }

        Ok(json!({
            "table": args.table,
            "schema": schema,
            "indexes": indexes,
            "index_count": indexes.len()
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::Text {
                    text: "When should I use get_table_indexes?".to_string(),
                },
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::Text {
                    text: "Use get_table_indexes to understand which columns are indexed. This helps you:\n\
                           - Write optimized WHERE clauses (use indexed columns)\n\
                           - Understand query performance implications\n\
                           - Find primary keys for joins\n\
                           - Identify unique constraints\n\n\
                           Example: get_table_indexes(table='users', schema='public') returns:\n\
                           - Primary key indexes (is_primary=true)\n\
                           - Unique indexes (is_unique=true)\n\
                           - Regular indexes\n\
                           Each index shows which columns are included (column_names array).\n\n\
                           Use this information to:\n\
                           1. Choose indexed columns in WHERE clauses for faster queries\n\
                           2. Understand join relationships via primary/foreign keys\n\
                           3. Avoid duplicate values in unique-indexed columns"
                        .to_string(),
                },
            },
        ])
    }
}
