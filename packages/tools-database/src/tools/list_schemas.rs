//! ListSchemas tool for database schema exploration

use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::database::{ListSchemasArgs, ListSchemasPromptArgs};
use kodegen_tools_config::ConfigManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use std::time::Duration;

use crate::tools::timeout::execute_with_timeout;
use crate::types::DatabaseType;

// =============================================================================
// Tool Struct
// =============================================================================

#[derive(Clone)]
pub struct ListSchemasTool {
    pool: Arc<AnyPool>,
    db_type: DatabaseType,
    config: ConfigManager,
}

impl ListSchemasTool {
    /// Create a new ListSchemas tool instance
    ///
    /// # Errors
    /// Returns error if connection_url cannot be parsed to determine database type
    pub fn new(
        pool: Arc<AnyPool>,
        connection_url: &str,
        config: ConfigManager,
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

        // Get SQL query from centralized schema_queries module
        let sql = crate::schema_queries::get_schemas_query(db_type);

        // Execute query with timeout (metadata queries should be fast)
        let pool = self.pool.clone();
        let sql_owned = sql.to_string();
        let rows = execute_with_timeout(
            &self.config,
            "db_metadata_query_timeout_secs",
            Duration::from_secs(10), // 10s default for metadata
            || {
                let pool = pool.clone();
                let sql = sql_owned.clone();
                async move { sqlx::query(&sql).fetch_all(&*pool).await }
            },
            "Listing database schemas",
        )
        .await?;

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
                    "How do I discover what databases/schemas are available to query?",
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
