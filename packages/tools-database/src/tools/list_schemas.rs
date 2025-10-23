//! ListSchemas tool for database schema exploration

use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_tool::Tool;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{AnyPool, Row};
use std::sync::Arc;

use crate::types::DatabaseType;

// =============================================================================
// Args Structs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSchemasArgs {
    // Empty - no parameters needed
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSchemasPromptArgs {}

// =============================================================================
// Tool Struct
// =============================================================================

#[derive(Clone)]
pub struct ListSchemasTool {
    pool: Arc<AnyPool>,
    db_type: DatabaseType,
}

impl ListSchemasTool {
    /// Create a new ListSchemas tool instance
    ///
    /// # Errors
    /// Returns error if connection_url cannot be parsed to determine database type
    pub fn new(pool: Arc<AnyPool>, connection_url: &str) -> Result<Self, McpError> {
        let db_type = DatabaseType::from_url(connection_url)
            .map_err(|e| McpError::Other(anyhow::anyhow!("Invalid database URL: {}", e)))?;
        Ok(Self { pool, db_type })
    }
}

// =============================================================================
// Tool Trait Implementation
// =============================================================================

impl Tool for ListSchemasTool {
    type Args = ListSchemasArgs;
    type PromptArgs = ListSchemasPromptArgs;

    fn name() -> &'static str {
        "list_schemas"
    }

    fn description() -> &'static str {
        "List all schemas (databases) in the current database connection. \
         For PostgreSQL, returns all user schemas (excludes pg_catalog, information_schema). \
         For MySQL/MariaDB, returns all databases you have access to. \
         For SQLite, returns ['main']. \
         Returns JSON with schemas array and count."
    }

    fn read_only() -> bool {
        true
    }

    fn open_world() -> bool {
        false
    }

    async fn execute(&self, _args: Self::Args) -> Result<Value, McpError> {
        // Use stored database type
        let db_type = self.db_type;

        // SQLite special case - no query needed
        if matches!(db_type, DatabaseType::SQLite) {
            return Ok(json!({
                "schemas": ["main"],
                "count": 1
            }));
        }

        // Get SQL query (inline queries since DBTOOL_5 not yet ready)
        let sql = match db_type {
            DatabaseType::Postgres => {
                "SELECT schema_name FROM information_schema.schemata \
                 WHERE schema_name NOT IN ('pg_catalog', 'information_schema', 'pg_toast') \
                 ORDER BY schema_name"
            }
            DatabaseType::MySQL | DatabaseType::MariaDB => {
                "SELECT schema_name FROM information_schema.schemata \
                 WHERE schema_name NOT IN ('information_schema', 'mysql', 'performance_schema', 'sys') \
                 ORDER BY schema_name"
            }
            DatabaseType::SQLite => unreachable!(), // Handled above
            DatabaseType::SqlServer => {
                "SELECT name as schema_name FROM sys.schemas \
                 WHERE name NOT IN ('sys', 'INFORMATION_SCHEMA') \
                 ORDER BY name"
            }
        };

        // Execute query
        let rows = sqlx::query(sql)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to fetch schemas: {}", e)))?;

        // Extract schema names
        let schemas: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get("schema_name").ok())
            .collect();

        Ok(json!({
            "schemas": schemas,
            "count": schemas.len()
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I discover what databases/schemas are available to query?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the list_schemas tool to discover available schemas/databases:\n\n\
                     **Usage**: list_schemas({})\n\n\
                     **What it returns per database type**:\n\
                     - **PostgreSQL**: User schemas like 'public', 'myapp', 'analytics' (excludes system schemas)\n\
                     - **MySQL/MariaDB**: All databases you have access to\n\
                     - **SQLite**: Always returns ['main'] (SQLite has no schema concept)\n\n\
                     **Example response**:\n\
                     ```json\n\
                     {\n\
                       \"schemas\": [\"public\", \"analytics\", \"staging\"],\n\
                       \"count\": 3\n\
                     }\n\
                     ```\n\n\
                     **Typical workflow**:\n\
                     1. list_schemas({}) - discover available schemas\n\
                     2. list_tables({\"schema\": \"public\"}) - see tables in a schema\n\
                     3. describe_table({\"schema\": \"public\", \"table\": \"users\"}) - explore table structure\n\
                     4. Execute queries on discovered tables",
                ),
            },
        ])
    }
}
