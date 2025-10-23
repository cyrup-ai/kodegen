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
pub mod ssh_tunnel;

// Tools (implemented in later tasks)
pub mod tools;

// Re-exports
pub use dsn::{
    DSNInfo, detect_database_type, extract_database, extract_host, extract_port, parse_dsn,
    rewrite_dsn_for_tunnel, validate_dsn,
};
pub use error::DatabaseError;
pub use types::{ExecuteOptions, SQLResult, StoredProcedure, TableColumn, TableIndex};
