//! Get table indexes tool

use crate::error::DatabaseError;
use crate::schema_queries::get_indexes_query;
use crate::tools::helpers::resolve_schema_default;
use crate::types::{DatabaseType, TableIndex};
use kodegen_mcp_tool::{error::McpError, Tool};
use kodegen_tools_config::ConfigManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::AnyPool;
use std::sync::Arc;

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

/// Tool for retrieving table index information
#[derive(Clone)]
pub struct GetTableIndexesTool {
    pool: Arc<AnyPool>,
    config: Arc<ConfigManager>,
}

impl GetTableIndexesTool {
    /// Create a new GetTableIndexesTool instance
    pub fn new(pool: Arc<AnyPool>, config: Arc<ConfigManager>) -> Self {
        Self { pool, config }
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
        // Detect database type
        let db_type = DatabaseType::from(self.pool.any_kind());

        // Resolve schema
        let schema = match args.schema {
            Some(s) => s,
            None => resolve_schema_default(db_type, &self.pool).await?,
        };

        // Get query from helper (DBTOOL_5)
        let (query, params) = get_indexes_query(db_type, &schema, &args.table);

        // Execute with parameters
        let mut q = sqlx::query(&query);
        for param in &params {
            q = q.bind(param);
        }
        let rows = q.fetch_all(&*self.pool).await.map_err(|e| {
            DatabaseError::QueryError(format!("Failed to get table indexes: {}", e))
        })?;

        // Parse into TableIndex structs
        let indexes: Vec<TableIndex> = rows
            .iter()
            .map(|row| {
                // Handle column_names as comma-separated string or array
                let column_names: Vec<String> = row
                    .try_get::<String, _>("column_names")
                    .map(|cols_str| {
                        cols_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .or_else(|_| {
                        // Try as array (PostgreSQL might return array type)
                        row.try_get::<Vec<String>, _>("column_names")
                    })
                    .unwrap_or_default();

                Ok(TableIndex {
                    index_name: row.try_get("index_name").unwrap_or_default(),
                    column_names,
                    is_unique: row.try_get("is_unique").unwrap_or(false),
                    is_primary: row.try_get("is_primary").unwrap_or(false),
                })
            })
            .collect::<Result<Vec<_>, DatabaseError>>()?;

        Ok(json!({
            "table": args.table,
            "schema": schema,
            "indexes": indexes,
            "index_count": indexes.len()
        }))
    }

    fn prompt_arguments() -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
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
