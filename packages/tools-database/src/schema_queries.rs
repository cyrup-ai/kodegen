//! Database-specific schema introspection queries
//!
//! This module provides pure utility functions that generate SQL query strings
//! for introspecting database schemas, tables, columns, indexes, and stored procedures.
//!
//! These functions DO NOT execute queries or connect to databases - they only return
//! SQL strings that other tools (like ExecuteSQL in DBTOOL_6) will execute.
//!
//! ## Database Support
//!
//! - PostgreSQL: Uses information_schema and pg_catalog
//! - MySQL/MariaDB: Uses information_schema
//! - SQLite: Uses sqlite_master and PRAGMA commands
//! - SQL Server: Uses information_schema and sys tables (future support)
//!
//! ## Parameter Placeholders
//!
//! Different databases use different parameter placeholder syntax:
//! - PostgreSQL: `$1`, `$2`, `$3` (positional)
//! - MySQL/MariaDB: `?` (positional)
//! - SQLite: `?` (positional, but PRAGMA commands can't be parameterized)
//! - SQL Server: `@P1`, `@P2`, `@P3` (named)

use crate::types::DatabaseType;

/// Returns SQL to list schemas/databases (excludes system schemas)
///
/// ## System Schema Exclusions
///
/// - **PostgreSQL**: `pg_catalog`, `information_schema`, `pg_toast`
/// - **MySQL/MariaDB**: `information_schema`, `mysql`, `performance_schema`, `sys`
/// - **SQL Server**: `sys`, `INFORMATION_SCHEMA`
/// - **SQLite**: N/A (no schemas - tools should return `["main"]` without query)
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_database::schema_queries::get_schemas_query;
///
/// let sql = get_schemas_query(DatabaseType::Postgres);
/// // Returns: "SELECT schema_name FROM information_schema.schemata WHERE..."
/// ```
pub fn get_schemas_query(db_type: DatabaseType) -> String {
    match db_type {
        DatabaseType::Postgres => {
            // Reference: tmp/dbhub/src/connectors/postgres/index.ts:134-145
            "SELECT schema_name FROM information_schema.schemata \
             WHERE schema_name NOT IN ('pg_catalog', 'information_schema', 'pg_toast') \
             ORDER BY schema_name"
                .to_string()
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            // Reference: tmp/dbhub/src/connectors/mysql/index.ts:115-124
            "SELECT schema_name FROM information_schema.schemata \
             WHERE schema_name NOT IN ('information_schema', 'mysql', 'performance_schema', 'sys') \
             ORDER BY schema_name"
                .to_string()
        }
        DatabaseType::SQLite => {
            // Reference: tmp/dbhub/src/connectors/sqlite/index.ts:141-144
            // SQLite has no schemas - tools should return ["main"] without query
            String::new()
        }
        DatabaseType::SqlServer => "SELECT name as schema_name FROM sys.schemas \
             WHERE name NOT IN ('sys', 'INFORMATION_SCHEMA') \
             ORDER BY name"
            .to_string(),
    }
}

/// Returns SQL to list tables in a schema + parameters
///
/// ## Special Cases
///
/// - **PostgreSQL**: Uses `$1` parameter, defaults to "public" schema if None
/// - **MySQL/MariaDB**: Uses `?` parameter, or `DATABASE()` function if schema is None
/// - **SQLite**: Queries sqlite_master, excludes system tables (sqlite_%), no parameters
/// - **SQL Server**: Uses `@P1` parameter, defaults to "dbo" schema if None
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_database::schema_queries::get_tables_query;
///
/// let (sql, params) = get_tables_query(DatabaseType::Postgres, Some("public"));
/// // Returns: ("SELECT table_name FROM ... WHERE table_schema = $1", ["public"])
/// ```
pub fn get_tables_query(db_type: DatabaseType, schema: Option<&str>) -> (String, Vec<String>) {
    match db_type {
        DatabaseType::Postgres => {
            // Reference: tmp/dbhub/src/connectors/postgres/index.ts:150-166
            let sql = "SELECT table_name FROM information_schema.tables \
                       WHERE table_schema = $1 AND table_type = 'BASE TABLE' \
                       ORDER BY table_name"
                .to_string();
            let params = vec![schema.unwrap_or("public").to_string()];
            (sql, params)
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            // Reference: tmp/dbhub/src/connectors/mysql/index.ts:129-154
            if let Some(s) = schema {
                let sql = "SELECT table_name FROM information_schema.tables \
                           WHERE table_schema = ? AND table_type = 'BASE TABLE' \
                           ORDER BY table_name"
                    .to_string();
                (sql, vec![s.to_string()])
            } else {
                // Use DATABASE() to get current database
                let sql = "SELECT table_name FROM information_schema.tables \
                           WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE' \
                           ORDER BY table_name"
                    .to_string();
                (sql, vec![])
            }
        }
        DatabaseType::SQLite => {
            // Reference: tmp/dbhub/src/connectors/sqlite/index.ts:149-161
            let sql = "SELECT name as table_name FROM sqlite_master \
                       WHERE type='table' AND name NOT LIKE 'sqlite_%' \
                       ORDER BY name"
                .to_string();
            (sql, vec![])
        }
        DatabaseType::SqlServer => {
            let sql = "SELECT table_name FROM information_schema.tables \
                       WHERE table_schema = @P1 AND table_type = 'BASE TABLE' \
                       ORDER BY table_name"
                .to_string();
            let params = vec![schema.unwrap_or("dbo").to_string()];
            (sql, params)
        }
    }
}

/// Returns SQL to get column information for a table + parameters
///
/// ## Return Columns
///
/// Queries return columns matching the `TableColumn` struct:
/// - `column_name` (String)
/// - `data_type` (String)
/// - `is_nullable` (String - "YES" or "NO")
/// - `column_default` (Option<String>)
///
/// ## ⚠️ SECURITY WARNING - SQLite PRAGMA
///
/// SQLite PRAGMA commands do NOT support parameterized queries. Table names are
/// interpolated directly into the query string. The calling code (ExecuteSQL tool)
/// MUST validate table names before calling this function to prevent SQL injection.
///
/// Only allow: alphanumeric characters, underscores, and validate against table list first.
///
/// ## SQLite PRAGMA Return Values
///
/// SQLite's `PRAGMA table_info()` returns different column names than information_schema:
/// - `cid` - column ID
/// - `name` - use as `column_name`
/// - `type` - use as `data_type`
/// - `notnull` - convert to `is_nullable` (0 = YES, 1 = NO)
/// - `dflt_value` - use as `column_default`
/// - `pk` - primary key flag
///
/// The ExecuteSQL tool must transform these to match the TableColumn struct.
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_database::schema_queries::get_table_schema_query;
///
/// let (sql, params) = get_table_schema_query(DatabaseType::Postgres, "public", "users");
/// // Returns: ("SELECT column_name, data_type... WHERE table_schema = $1 AND table_name = $2",
/// //           ["public", "users"])
/// ```
pub fn get_table_schema_query(
    db_type: DatabaseType,
    schema: &str,
    table: &str,
) -> (String, Vec<String>) {
    match db_type {
        DatabaseType::Postgres => {
            // Reference: tmp/dbhub/src/connectors/postgres/index.ts:232-250
            let sql = "SELECT column_name, data_type, is_nullable, column_default \
                       FROM information_schema.columns \
                       WHERE table_schema = $1 AND table_name = $2 \
                       ORDER BY ordinal_position"
                .to_string();
            (sql, vec![schema.to_string(), table.to_string()])
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            // Reference: tmp/dbhub/src/connectors/mysql/index.ts:279-299
            let sql = "SELECT column_name, data_type, is_nullable, column_default \
                       FROM information_schema.columns \
                       WHERE table_schema = ? AND table_name = ? \
                       ORDER BY ordinal_position"
                .to_string();
            (sql, vec![schema.to_string(), table.to_string()])
        }
        DatabaseType::SQLite => {
            // Reference: tmp/dbhub/src/connectors/sqlite/index.ts:254-272
            // SECURITY: Table name must be validated before interpolation!
            // PRAGMA doesn't support parameterization
            let sql = format!("PRAGMA table_info({})", table);
            // Note: PRAGMA returns different column names:
            // cid, name (use as column_name), type (use as data_type),
            // notnull (convert to is_nullable), dflt_value (use as column_default), pk
            // The ExecuteSQL tool will need to transform these to match TableColumn
            (sql, vec![])
        }
        DatabaseType::SqlServer => {
            let sql = "SELECT column_name, data_type, is_nullable, column_default \
                       FROM information_schema.columns \
                       WHERE table_schema = @P1 AND table_name = @P2 \
                       ORDER BY ordinal_position"
                .to_string();
            (sql, vec![schema.to_string(), table.to_string()])
        }
    }
}

/// Returns SQL to get index information for a table + parameters
///
/// ## Return Columns
///
/// Queries return columns matching the `TableIndex` struct:
/// - `index_name` (String)
/// - `column_names` (Vec<String>) - Array of column names in the index
/// - `is_unique` (bool)
/// - `is_primary` (bool)
///
/// ## Database-Specific Notes
///
/// ### PostgreSQL
/// Uses complex joins on pg_catalog tables (pg_class, pg_index, pg_attribute, pg_namespace).
/// Uses `array_agg()` to aggregate multi-column indexes.
///
/// ### MySQL/MariaDB
/// Uses `information_schema.statistics` with `GROUP_CONCAT()` to aggregate columns.
/// Note: `NON_UNIQUE=0` means the index IS unique.
///
/// ### SQLite
/// Returns `PRAGMA index_list()` which only provides index names and unique flags.
/// Complete index information requires multiple PRAGMA calls:
/// 1. `PRAGMA index_list(table_name)` - gets index names and unique flags
/// 2. For each index: `PRAGMA index_info(index_name)` - gets columns
/// 3. `PRAGMA table_info(table_name)` - finds primary key columns
///
/// The ExecuteSQL tool will need to make follow-up calls to get complete index information.
///
/// ### SQL Server
/// Uses sys.indexes and sys.index_columns with `STRING_AGG()` for column aggregation.
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_database::schema_queries::get_indexes_query;
///
/// let (sql, params) = get_indexes_query(DatabaseType::Postgres, "public", "users");
/// // Returns: ("SELECT i.relname as index_name... FROM pg_class t...", ["public", "users"])
/// ```
pub fn get_indexes_query(
    db_type: DatabaseType,
    schema: &str,
    table: &str,
) -> (String, Vec<String>) {
    match db_type {
        DatabaseType::Postgres => {
            // Reference: tmp/dbhub/src/connectors/postgres/index.ts:200-230
            let sql = "SELECT \
                           i.relname as index_name, \
                           array_agg(a.attname) as column_names, \
                           ix.indisunique as is_unique, \
                           ix.indisprimary as is_primary \
                       FROM \
                           pg_class t, \
                           pg_class i, \
                           pg_index ix, \
                           pg_attribute a, \
                           pg_namespace ns \
                       WHERE \
                           t.oid = ix.indrelid \
                           AND i.oid = ix.indexrelid \
                           AND a.attrelid = t.oid \
                           AND a.attnum = ANY(ix.indkey) \
                           AND t.relkind = 'r' \
                           AND t.relname = $2 \
                           AND ns.oid = t.relnamespace \
                           AND ns.nspname = $1 \
                       GROUP BY \
                           i.relname, \
                           ix.indisunique, \
                           ix.indisprimary \
                       ORDER BY \
                           i.relname"
                .to_string();
            (sql, vec![schema.to_string(), table.to_string()])
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            // Reference: tmp/dbhub/src/connectors/mysql/index.ts:189-238
            let sql = "SELECT \
                           index_name, \
                           GROUP_CONCAT(column_name ORDER BY seq_in_index) as column_names, \
                           NOT non_unique as is_unique, \
                           index_name = 'PRIMARY' as is_primary \
                       FROM information_schema.statistics \
                       WHERE table_schema = ? AND table_name = ? \
                       GROUP BY index_name, non_unique \
                       ORDER BY index_name"
                .to_string();
            (sql, vec![schema.to_string(), table.to_string()])
        }
        DatabaseType::SQLite => {
            // Reference: tmp/dbhub/src/connectors/sqlite/index.ts:189-252
            // Note: Complete index info requires multiple PRAGMA calls
            // This returns the index list; ExecuteSQL tool will need to make follow-up calls
            let sql = format!("PRAGMA index_list({})", table);
            (sql, vec![])
        }
        DatabaseType::SqlServer => {
            let sql = "SELECT \
                           i.name as index_name, \
                           STRING_AGG(c.name, ',') as column_names, \
                           i.is_unique, \
                           i.is_primary_key as is_primary \
                       FROM sys.indexes i \
                       JOIN sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id \
                       JOIN sys.columns c ON ic.object_id = c.object_id AND ic.column_id = c.column_id \
                       WHERE OBJECT_NAME(i.object_id) = @P2 \
                         AND SCHEMA_NAME(OBJECTPROPERTY(i.object_id, 'SchemaId')) = @P1 \
                       GROUP BY i.name, i.is_unique, i.is_primary_key \
                       ORDER BY i.name".to_string();
            (sql, vec![schema.to_string(), table.to_string()])
        }
    }
}

/// Returns SQL to list stored procedures in a schema + parameters
///
/// ## Return Columns
///
/// Queries return columns matching the `StoredProcedure` struct (minimum required):
/// - `procedure_name` (String)
/// - `procedure_type` (String) - "procedure" or "function"
/// - `language` (Option<String>)
///
/// ## SQLite Support
///
/// SQLite does NOT support stored procedures or functions. This function returns `None` for SQLite.
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_database::schema_queries::get_stored_procedures_query;
///
/// let result = get_stored_procedures_query(DatabaseType::Postgres, "public");
/// // Returns: Some(("SELECT routine_name as procedure_name... WHERE routine_schema = $1", ["public"]))
///
/// let result = get_stored_procedures_query(DatabaseType::SQLite, "main");
/// // Returns: None
/// ```
pub fn get_stored_procedures_query(
    db_type: DatabaseType,
    schema: &str,
) -> Option<(String, Vec<String>)> {
    match db_type {
        DatabaseType::Postgres => {
            // Reference: tmp/dbhub/src/connectors/postgres/index.ts:283-297
            let sql = "SELECT \
                           routine_name as procedure_name, \
                           routine_type, \
                           CASE WHEN routine_type = 'PROCEDURE' THEN 'procedure' ELSE 'function' END as procedure_type, \
                           external_language as language \
                       FROM information_schema.routines \
                       WHERE routine_schema = $1 \
                       ORDER BY routine_name".to_string();
            Some((sql, vec![schema.to_string()]))
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            let sql = "SELECT \
                           routine_name as procedure_name, \
                           routine_type, \
                           CASE WHEN routine_type = 'PROCEDURE' THEN 'procedure' ELSE 'function' END as procedure_type, \
                           external_language as language \
                       FROM information_schema.routines \
                       WHERE routine_schema = ? \
                       ORDER BY routine_name".to_string();
            Some((sql, vec![schema.to_string()]))
        }
        DatabaseType::SQLite => {
            // SQLite doesn't support stored procedures
            None
        }
        DatabaseType::SqlServer => {
            let sql = "SELECT \
                           routine_name as procedure_name, \
                           routine_type, \
                           CASE WHEN routine_type = 'PROCEDURE' THEN 'procedure' ELSE 'function' END as procedure_type, \
                           'SQL' as language \
                       FROM information_schema.routines \
                       WHERE routine_schema = @P1 \
                       ORDER BY routine_name".to_string();
            Some((sql, vec![schema.to_string()]))
        }
    }
}

/// Returns the default schema name for each database type
///
/// ## Return Values
///
/// - **PostgreSQL**: `Some("public")` - Standard default schema
/// - **MySQL/MariaDB**: `None` - Must execute `SELECT DATABASE()` to get current database
/// - **SQLite**: `Some("main")` - Default database name
/// - **SQL Server**: `Some("dbo")` - Default schema for user objects
///
/// ## MySQL Special Case
///
/// MySQL's default "schema" (database) depends on which database the connection is using.
/// Tools must execute `SELECT DATABASE()` to determine the current database name.
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_database::schema_queries::get_default_schema;
///
/// let schema = get_default_schema(DatabaseType::Postgres);
/// // Returns: Some("public")
///
/// let schema = get_default_schema(DatabaseType::MySQL);
/// // Returns: None - must query DATABASE()
/// ```
pub fn get_default_schema(db_type: DatabaseType) -> Option<&'static str> {
    match db_type {
        DatabaseType::Postgres => Some("public"),
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            // MySQL requires DATABASE() query to get current database
            // Tools should execute "SELECT DATABASE()" and use the result
            None
        }
        DatabaseType::SQLite => Some("main"),
        DatabaseType::SqlServer => Some("dbo"),
    }
}
