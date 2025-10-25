//! Database query tools for kodegen MCP server
//!
//! Provides tools for executing SQL queries and exploring database schemas
//! across PostgreSQL, MySQL, MariaDB, SQLite, and SQL Server.

pub mod error;
pub mod types;

// Utilities (implemented in later tasks)
pub mod dsn;
pub mod readonly;
pub mod schema_queries;
pub mod sql_limiter;
pub mod sql_parser;
pub mod ssh_tunnel;
pub mod validate;
pub mod connection;

// Tools (implemented in later tasks)
pub mod tools;

// Re-export secrecy types for consumers
pub use secrecy::{ExposeSecret, SecretString};

// Re-exports
pub use dsn::{
    DSNInfo, detect_database_type, extract_database, extract_host, extract_port, parse_dsn,
    rewrite_dsn_for_tunnel, validate_dsn,
};
pub use error::DatabaseError;
pub use readonly::validate_readonly_sql;
pub use schema_queries::{
    get_default_schema, get_indexes_query, get_schemas_query, get_stored_procedures_query,
    get_table_schema_query, get_tables_query,
};
pub use sql_limiter::apply_row_limit;
pub use sql_parser::{extract_first_keyword, split_sql_statements, strip_comments};
pub use ssh_tunnel::{SSHAuth, SSHConfig, SSHTunnel, TunnelConfig, establish_tunnel};
pub use connection::{DatabaseConnection, setup_database_pool, warmup_pool};
pub use tools::ExecuteSQLTool;
pub use types::{
    DatabaseType, ExecuteOptions, SQLResult, StoredProcedure, TableColumn, TableIndex,
};
pub use validate::validate_sqlite_identifier;
