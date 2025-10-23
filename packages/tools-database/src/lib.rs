//! Database query tools for kodegen MCP server
//!
//! Provides tools for executing SQL queries and exploring database schemas
//! across PostgreSQL, MySQL, MariaDB, SQLite, and SQL Server.

pub mod error;
pub mod types;

// Utilities (implemented in later tasks)
pub mod dsn;
pub mod ssh_tunnel;
pub mod sql_limiter;
pub mod readonly;
pub mod schema_queries;

// Tools (implemented in later tasks)
pub mod tools;

// Re-exports
pub use error::DatabaseError;
pub use types::{ExecuteOptions, SQLResult, StoredProcedure, TableColumn, TableIndex};
