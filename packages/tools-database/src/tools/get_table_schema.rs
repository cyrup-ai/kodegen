//! Get table schema (column information) tool

use crate::error::DatabaseError;
use crate::schema_queries::get_table_schema_query;
use crate::tools::helpers::resolve_schema_default;
use crate::types::{DatabaseType, TableColumn};
use kodegen_mcp_tool::{error::McpError, Tool};
use kodegen_tools_config::ConfigManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::AnyPool;
use std::sync::Arc;

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

/// Tool for retrieving table column information
#[derive(Clone)]
pub struct GetTableSchemaTool {
    pool: Arc<AnyPool>,
    config: Arc<ConfigManager>,
}

impl GetTableSchemaTool {
    /// Create a new GetTableSchemaTool instance
    pub fn new(pool: Arc<AnyPool>, config: Arc<ConfigManager>) -> Self {
        Self { pool, config }
    }
}

impl Tool for GetTableSchemaTool {
    type Args = GetTableSchemaArgs;
    type PromptArgs = GetTableSchemaPromptArgs;

    fn name() -> &'static str {
        "get_table_schema"
    }

    fn description() -> &'static str {
        "Get column information for a table including column names, data types, \
         nullability, and default values. Use this before writing queries to \
         understand the table structure. Returns array of columns with metadata."
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

        // Resolve schema (use provided or default)
        let schema = match args.schema {
            Some(s) => s,
            None => resolve_schema_default(db_type, &self.pool).await?,
        };

        // Get query from helper (DBTOOL_5)
        let (query, params) = get_table_schema_query(db_type, &schema, &args.table);

        // Execute with parameters
        let mut q = sqlx::query(&query);
        for param in &params {
            q = q.bind(param);
        }
        let rows = q.fetch_all(&*self.pool).await.map_err(|e| {
            DatabaseError::QueryError(format!("Failed to get table schema: {}", e))
        })?;

        // Parse into TableColumn structs
        let columns: Vec<TableColumn> = rows
            .iter()
            .map(|row| {
                Ok(TableColumn {
                    column_name: row
                        .try_get("column_name")
                        .or_else(|_| row.try_get("name"))
                        .unwrap_or_default(),
                    data_type: row
                        .try_get("data_type")
                        .or_else(|_| row.try_get("type"))
                        .unwrap_or_default(),
                    is_nullable: row
                        .try_get("is_nullable")
                        .or_else(|_| {
                            // SQLite: notnull field (0 = nullable, 1 = not null)
                            row.try_get::<i32, _>("notnull")
                                .map(|v| if v == 0 { "YES" } else { "NO" }.to_string())
                        })
                        .unwrap_or_else(|_| "YES".to_string()),
                    column_default: row
                        .try_get("column_default")
                        .or_else(|_| row.try_get("dflt_value"))
                        .ok(),
                })
            })
            .collect::<Result<Vec<_>, DatabaseError>>()?;

        Ok(json!({
            "table": args.table,
            "schema": schema,
            "columns": columns,
            "column_count": columns.len()
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
                    text: "When should I use get_table_schema?".to_string(),
                },
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::Text {
                    text: "Use get_table_schema when you need to understand a table's structure \
                           before writing queries. It returns column names, data types, nullability, \
                           and default values.\n\n\
                           Example workflow:\n\
                           1. list_schemas() -> find available schemas\n\
                           2. list_tables(schema='public') -> find tables\n\
                           3. get_table_schema(table='users', schema='public') -> see columns\n\
                           4. execute_sql('SELECT id, name FROM users WHERE ...') -> write accurate query\n\n\
                           The schema parameter is optional and defaults to:\n\
                           - PostgreSQL: 'public'\n\
                           - MySQL/MariaDB: current DATABASE()\n\
                           - SQLite: 'main'\n\
                           - SQL Server: 'dbo'"
                        .to_string(),
                },
            },
        ])
    }
}
