//! Get table schema (column information) tool

use crate::error::DatabaseError;
use crate::schema_queries::get_table_schema_query;
use crate::tools::helpers::resolve_schema_default;
use crate::tools::timeout::execute_with_timeout;
use crate::types::{DatabaseType, TableColumn};
use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::database::{GetTableSchemaArgs, GetTableSchemaPromptArgs};
use kodegen_tools_config::ConfigManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use std::time::Duration;

/// Tool for retrieving table column information
#[derive(Clone)]
pub struct GetTableSchemaTool {
    pool: Arc<AnyPool>,
    db_type: DatabaseType,
    config: Arc<ConfigManager>,
}

impl GetTableSchemaTool {
    /// Create a new GetTableSchemaTool instance
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
        // Use stored database type
        let db_type = self.db_type;

        // Resolve schema (use provided or default)
        let schema = match args.schema {
            Some(s) => s,
            None => resolve_schema_default(db_type, &self.pool, &self.config).await?,
        };

        // Get query from helper (DBTOOL_5) - validation enforced for SQLite
        let (query, params) = get_table_schema_query(db_type, &schema, &args.table)?;

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
            "Getting table schema",
        )
        .await?;

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

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
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
