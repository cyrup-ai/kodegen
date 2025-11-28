//! Database tools: execute SQL, schema inspection, connection management

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn database_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: DB_EXECUTE_SQL,
            category: "database",
            description: "Execute SQL query or multiple SQL statements (separated by semicolons). nn MULTI-STATEMENT BEHAVIOR:n - Write operations (INSERT/UPDATE/DELETE/CREA...",
            schema: build_schema::<database::ExecuteSQLArgs>(),
        },
        ToolMetadata {
            name: DB_POOL_STATS,
            category: "database",
            description: "Get connection pool health metrics including active connections, idle connections, and pool configuration. Use this to diagnose connection pool exh...",
            schema: build_schema::<database::GetPoolStatsArgs>(),
        },
        ToolMetadata {
            name: DB_STORED_PROCEDURES,
            category: "database",
            description: "List stored procedures in a schema. Returns procedure names and optionally detailed information including parameters and definitions. Not supported...",
            schema: build_schema::<database::GetStoredProceduresArgs>(),
        },
        ToolMetadata {
            name: DB_TABLE_INDEXES,
            category: "database",
            description: "Get index information for a table including index names, columns, uniqueness, and primary key status. Use this to understand which columns are inde...",
            schema: build_schema::<database::GetTableIndexesArgs>(),
        },
        ToolMetadata {
            name: DB_TABLE_SCHEMA,
            category: "database",
            description: "Get column information for a table including column names, data types, nullability, and default values. Use this before writing queries to understa...",
            schema: build_schema::<database::GetTableSchemaArgs>(),
        },
        ToolMetadata {
            name: DB_LIST_SCHEMAS,
            category: "database",
            description: "List all schemas (databases) in the current database connection. For PostgreSQL, returns all user schemas (excludes pg_catalog, information_schema)...",
            schema: build_schema::<database::ListSchemasArgs>(),
        },
        ToolMetadata {
            name: DB_LIST_TABLES,
            category: "database",
            description: "List all tables in a schema. If schema not provided, uses default schema (public for PostgreSQL, current database for MySQL, main for SQLite, dbo f...",
            schema: build_schema::<database::ListTablesArgs>(),
        },
    ]
}
