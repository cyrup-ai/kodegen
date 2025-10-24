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

use crate::error::DatabaseError;
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
            // Use CAST() for sqlx::any compatibility (NAME type not supported by Any driver)
            "SELECT CAST(schema_name AS TEXT) as schema_name FROM information_schema.schemata \
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
            // Use CAST() for sqlx::any compatibility
            let sql =
                "SELECT CAST(table_name AS TEXT) as table_name FROM information_schema.tables \
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
/// ## SQLite PRAGMA Validation
///
/// For SQLite, PRAGMA commands do NOT support parameterized queries. This function
/// automatically validates table names before interpolation to prevent SQL injection.
/// Validation uses strict rules: alphanumeric + underscore only, no SQL keywords.
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
/// ## Errors
///
/// Returns `DatabaseError::QueryError` if the table name fails validation (SQLite only).
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_database::schema_queries::get_table_schema_query;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///
/// let (sql, params) = get_table_schema_query(DatabaseType::Postgres, "public", "users")?;
/// assert!(sql.contains("information_schema.columns"));
/// assert_eq!(params[0], "public");
/// assert_eq!(params[1], "users");
/// # Ok(())
/// # }
/// ```
pub fn get_table_schema_query(
    db_type: DatabaseType,
    schema: &str,
    table: &str,
) -> Result<(String, Vec<String>), DatabaseError> {
    match db_type {
        DatabaseType::Postgres => {
            // Reference: tmp/dbhub/src/connectors/postgres/index.ts:232-250
            // Use CAST() for sqlx::any compatibility
            let sql = "SELECT \
                           CAST(column_name AS TEXT) as column_name, \
                           CAST(data_type AS TEXT) as data_type, \
                           CAST(is_nullable AS TEXT) as is_nullable, \
                           CAST(column_default AS TEXT) as column_default \
                       FROM information_schema.columns \
                       WHERE table_schema = $1 AND table_name = $2 \
                       ORDER BY ordinal_position"
                .to_string();
            Ok((sql, vec![schema.to_string(), table.to_string()]))
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            // Reference: tmp/dbhub/src/connectors/mysql/index.ts:279-299
            let sql = "SELECT column_name, data_type, is_nullable, column_default \
                       FROM information_schema.columns \
                       WHERE table_schema = ? AND table_name = ? \
                       ORDER BY ordinal_position"
                .to_string();
            Ok((sql, vec![schema.to_string(), table.to_string()]))
        }
        DatabaseType::SQLite => {
            // SECURITY: Validate identifier before string interpolation
            // This prevents SQL injection in PRAGMA commands which cannot use parameters
            crate::validate::validate_sqlite_identifier(table)?;

            let sql = format!("PRAGMA table_info({})", table);
            // Note: PRAGMA returns different column names (cid, name, type, notnull, dflt_value, pk)
            // ExecuteSQL tool transforms these to match TableColumn struct
            Ok((sql, vec![]))
        }
        DatabaseType::SqlServer => {
            let sql = "SELECT column_name, data_type, is_nullable, column_default \
                       FROM information_schema.columns \
                       WHERE table_schema = @P1 AND table_name = @P2 \
                       ORDER BY ordinal_position"
                .to_string();
            Ok((sql, vec![schema.to_string(), table.to_string()]))
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
/// Uses `information_schema.statistics` with columns fetched per-index to avoid
/// GROUP_CONCAT truncation limits.
///
/// ### SQLite
/// Returns `PRAGMA index_list()` which requires follow-up calls. PRAGMA commands
/// cannot use parameterized queries, so this function automatically validates
/// table names before interpolation to prevent SQL injection.
///
/// Complete SQLite index info requires:
/// 1. `PRAGMA index_list(table_name)` - gets index names and unique flags
/// 2. For each index: `PRAGMA index_info(index_name)` - gets columns
/// 3. `PRAGMA table_info(table_name)` - finds primary key columns
///
/// ### SQL Server
/// Uses sys.indexes and sys.index_columns with `STRING_AGG()` for column aggregation.
///
/// ## Errors
///
/// Returns `DatabaseError::QueryError` if the table name fails validation (SQLite only).
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_database::schema_queries::get_indexes_query;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///
/// let (sql, params) = get_indexes_query(DatabaseType::Postgres, "public", "users")?;
/// assert!(sql.contains("pg_class") || sql.contains("information_schema"));
/// assert_eq!(params.len(), 2);
/// # Ok(())
/// # }
/// ```
pub fn get_indexes_query(
    db_type: DatabaseType,
    schema: &str,
    table: &str,
) -> Result<(String, Vec<String>), DatabaseError> {
    match db_type {
        DatabaseType::Postgres => {
            // Reference: tmp/dbhub/src/connectors/postgres/index.ts:200-230
            // Use array_to_string() instead of array_agg() for sqlx::any compatibility
            // (Any driver doesn't support TEXT[] array type)
            let sql = "SELECT \
                           CAST(i.relname AS TEXT) as index_name, \
                           array_to_string(array_agg(CAST(a.attname AS TEXT)), ',') as column_names, \
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
            Ok((sql, vec![schema.to_string(), table.to_string()]))
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            // Single query returns ALL index-column rows
            // Grouping happens in Rust (get_table_indexes.rs) to avoid:
            // 1. GROUP_CONCAT 1024-byte truncation limit
            // 2. N+1 query pattern
            let sql = "SELECT \
                           index_name, \
                           column_name, \
                           seq_in_index, \
                           NOT non_unique as is_unique, \
                           index_name = 'PRIMARY' as is_primary \
                       FROM information_schema.statistics \
                       WHERE table_schema = ? AND table_name = ? \
                       ORDER BY index_name, seq_in_index"
                .to_string();
            Ok((sql, vec![schema.to_string(), table.to_string()]))
        }
        DatabaseType::SQLite => {
            // SECURITY: Validate identifier before string interpolation
            crate::validate::validate_sqlite_identifier(table)?;

            let sql = format!("PRAGMA index_list({})", table);
            // Note: Returns index list only; ExecuteSQL tool makes follow-up calls
            // to PRAGMA index_info(index_name) for each index to get columns
            Ok((sql, vec![]))
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
            Ok((sql, vec![schema.to_string(), table.to_string()]))
        }
    }
}

/// Returns SQL to get columns for a specific index + parameters
///
/// This is used for MySQL/MariaDB to avoid GROUP_CONCAT truncation.
/// Called once per index after getting the index list.
///
/// ## Example
///
/// ```rust
/// use kodegen_tools_database::types::DatabaseType;
/// use kodegen_tools_database::schema_queries::get_index_columns_query;
///
/// let (sql, params) = get_index_columns_query(
///     DatabaseType::MySQL,
///     "public",
///     "users",
///     "idx_user_email"
/// );
/// // Returns: ("SELECT column_name FROM information_schema.statistics
/// //            WHERE table_schema = ? AND table_name = ? AND index_name = ?
/// //            ORDER BY seq_in_index", ["public", "users", "idx_user_email"])
/// ```
pub fn get_index_columns_query(
    db_type: DatabaseType,
    schema: &str,
    table: &str,
    index_name: &str,
) -> (String, Vec<String>) {
    match db_type {
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            let sql = "SELECT column_name \
                       FROM information_schema.statistics \
                       WHERE table_schema = ? AND table_name = ? AND index_name = ? \
                       ORDER BY seq_in_index"
                .to_string();
            (
                sql,
                vec![
                    schema.to_string(),
                    table.to_string(),
                    index_name.to_string(),
                ],
            )
        }
        _ => {
            // Other databases don't need this (they use array aggregation)
            (String::new(), vec![])
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
            // Use CAST() for sqlx::any compatibility
            let sql = "SELECT \
                           CAST(routine_name AS TEXT) as procedure_name, \
                           CAST(routine_type AS TEXT) as routine_type, \
                           CASE WHEN routine_type = 'PROCEDURE' THEN 'procedure' ELSE 'function' END as procedure_type, \
                           CAST(external_language AS TEXT) as language \
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
