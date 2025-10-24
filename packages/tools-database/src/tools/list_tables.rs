//! ListTables tool for database table exploration

use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::database::{ListTablesArgs, ListTablesPromptArgs};
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
pub struct ListTablesTool {
    pool: Arc<AnyPool>,
    db_type: DatabaseType,
    config: ConfigManager,
}

impl ListTablesTool {
    /// Create a new ListTables tool instance
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

impl Tool for ListTablesTool {
    type Args = ListTablesArgs;
    type PromptArgs = ListTablesPromptArgs;

    fn name() -> &'static str {
        "list_tables"
    }

    fn description() -> &'static str {
        "List all tables in a schema. If schema not provided, uses default schema \
         (public for PostgreSQL, current database for MySQL, main for SQLite, dbo for SQL Server). \
         Returns JSON with tables array, schema name, and count."
    }

    fn read_only() -> bool {
        true
    }

    fn open_world() -> bool {
        false
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Use stored database type
        let db_type = self.db_type;

        // Get SQL query from centralized schema_queries module
        let (sql, params) =
            crate::schema_queries::get_tables_query(db_type, args.schema.as_deref());

        // Determine resolved schema for response
        let resolved_schema = args.schema.unwrap_or_else(|| {
            crate::schema_queries::get_default_schema(db_type)
                .unwrap_or("main")
                .to_string()
        });

        // Execute query with parameters and timeout
        let pool = self.pool.clone();
        let sql_owned = sql.to_string();
        let params_owned = params.clone();
        let rows = execute_with_timeout(
            &self.config,
            "db_metadata_query_timeout_secs",
            Duration::from_secs(10), // 10s default for metadata
            || {
                let pool = pool.clone();
                let sql = sql_owned.clone();
                let params = params_owned.clone();
                async move {
                    let mut query = sqlx::query(&sql);
                    for param in &params {
                        query = query.bind(param);
                    }
                    query.fetch_all(&*pool).await
                }
            },
            "Listing tables",
        )
        .await?;

        // Extract table names
        let tables: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get("table_name").ok())
            .collect();

        Ok(json!({
            "tables": tables,
            "schema": resolved_schema,
            "count": tables.len()
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
                    "How do I see what tables are available in a database?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the list_tables tool to discover available tables:\n\n\
                     **Usage examples**:\n\
                     ```json\n\
                     // List tables in specific schema\n\
                     list_tables({\"schema\": \"public\"})\n\n\
                     // List tables in default schema\n\
                     list_tables({})\n\
                     ```\n\n\
                     **What happens when schema is omitted**:\n\
                     - **PostgreSQL**: Uses 'public' schema\n\
                     - **MySQL/MariaDB**: Uses currently connected database\n\
                     - **SQLite**: Uses 'main' (only schema available)\n\
                     - **SQL Server**: Uses 'dbo' schema\n\n\
                     **Example response**:\n\
                     ```json\n\
                     {\n\
                       \"tables\": [\"users\", \"posts\", \"comments\"],\n\
                       \"schema\": \"public\",\n\
                       \"count\": 3\n\
                     }\n\
                     ```\n\n\
                     **Discovery workflow**:\n\
                     1. list_schemas({}) - see what schemas/databases exist\n\
                     2. list_tables({\"schema\": \"public\"}) - see tables in chosen schema\n\
                     3. describe_table({...}) - explore structure of interesting tables\n\
                     4. Execute SQL queries on discovered tables",
                ),
            },
        ])
    }
}
