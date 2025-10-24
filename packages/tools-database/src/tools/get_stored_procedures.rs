//! Get stored procedures tool

use crate::error::DatabaseError;
use crate::schema_queries::get_stored_procedures_query;
use crate::tools::helpers::resolve_schema_default;
use crate::tools::timeout::execute_with_timeout;
use crate::types::{DatabaseType, StoredProcedure};
use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::database::{GetStoredProceduresArgs, GetStoredProceduresPromptArgs};
use kodegen_tools_config::ConfigManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use std::time::Duration;

/// Tool for listing stored procedures and functions
#[derive(Clone)]
pub struct GetStoredProceduresTool {
    pool: Arc<AnyPool>,
    db_type: DatabaseType,
    config: Arc<ConfigManager>,
}

impl GetStoredProceduresTool {
    /// Create a new GetStoredProceduresTool instance
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

impl Tool for GetStoredProceduresTool {
    type Args = GetStoredProceduresArgs;
    type PromptArgs = GetStoredProceduresPromptArgs;

    fn name() -> &'static str {
        "get_stored_procedures"
    }

    fn description() -> &'static str {
        "List stored procedures in a schema. Returns procedure names and optionally \
         detailed information including parameters and definitions. \
         Not supported for SQLite."
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

        // SQLite doesn't support stored procedures
        if matches!(db_type, DatabaseType::SQLite) {
            return Err(DatabaseError::FeatureNotSupported(
                "SQLite does not support stored procedures".to_string(),
            )
            .into());
        }

        // Resolve schema
        let schema = match args.schema {
            Some(s) => s,
            None => resolve_schema_default(db_type, &self.pool, &self.config).await?,
        };

        // Get query from helper (DBTOOL_5)
        let Some((query, params)) = get_stored_procedures_query(db_type, &schema) else {
            return Err(DatabaseError::FeatureNotSupported(format!(
                "{} does not support stored procedures",
                db_type
            ))
            .into());
        };

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
            "Getting stored procedures",
        )
        .await?;

        // Parse into StoredProcedure structs
        let procedures: Vec<StoredProcedure> = rows
            .iter()
            .map(|row| {
                Ok(StoredProcedure {
                    procedure_name: row.try_get("procedure_name").unwrap_or_default(),
                    procedure_type: row.try_get("procedure_type").unwrap_or_default(),
                    language: row.try_get("language").ok(),
                    parameter_list: row.try_get("parameter_list").ok(),
                    return_type: row.try_get("return_type").ok(),
                    definition: if args.include_details {
                        row.try_get("definition").ok()
                    } else {
                        None
                    },
                })
            })
            .collect::<Result<Vec<_>, DatabaseError>>()?;

        Ok(json!({
            "schema": schema,
            "procedures": procedures,
            "count": procedures.len()
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
                    text: "When should I use get_stored_procedures?".to_string(),
                },
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::Text {
                    text: "Use get_stored_procedures to discover available procedures and functions.\n\n\
                           Supported databases: PostgreSQL, MySQL/MariaDB, SQL Server\n\
                           NOT supported: SQLite (returns error)\n\n\
                           Set include_details=true to see:\n\
                           - Parameter lists\n\
                           - Return types (for functions)\n\
                           - Full definitions\n\n\
                           Set include_details=false for just procedure names (faster, less data).\n\n\
                           Example:\n\
                           get_stored_procedures(schema='public', include_details=false)\n\
                           -> Returns list of procedure names and types"
                        .to_string(),
                },
            },
        ])
    }
}
