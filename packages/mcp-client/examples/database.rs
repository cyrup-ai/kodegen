//! Database tools example - demonstrates all 7 database tools
//!
//! This example shows how to:
//! - Connect to a database server via connection pool
//! - List schemas and tables
//! - Introspect table structure (columns, indexes)
//! - Execute SQL queries
//! - Monitor connection pool health
//! - Query stored procedures (PostgreSQL/MySQL only)
//!
//! # Prerequisites
//!
//! This example requires a running PostgreSQL database:
//!
//! ```bash
//! cd packages/tools-database
//! docker-compose up -d
//! ```
//!
//! The Docker setup provides a test database with sample data:
//! - 5 tables: departments, employees, projects, employee_projects, audit_log
//! - Pre-loaded with test data
//! - Stored procedure: get_department_employee_count(dept_id)
//!
//! # Architecture
//!
//! Database tools maintain a connection pool at the server level.
//! The server is started with `--database-dsn` flag pointing to ONE database.
//! All tools then operate against that pre-configured connection pool.
//!
//! This design enables:
//! - Connection pooling and reuse
//! - Prepared statement caching
//! - Transaction management
//! - Health monitoring

mod common;

use anyhow::{Context, Result};
use kodegen_mcp_client::tools;
use serde_json::json;
use std::time::Duration;
use tokio::process::Command;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting database tools example");

    // Connect to kodegen server with database DSN
    let (conn, mut server) = connect_to_database_server(
        "postgres://testuser:testpass@localhost:5432/testdb"
    ).await?;

    // Wrap client with logging
    let log_path = std::path::PathBuf::from("/tmp/kodegen/mcp-client/database.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // Run example
    let result = run_database_example(&client).await;

    // Always close connection
    conn.close().await?;
    server.shutdown().await?;

    result
}

/// Connect to kodegen server with database connection pool
///
/// Spawns the server with `--database-dsn` flag to establish a connection pool.
/// The server will pre-configure database tools with this connection.
async fn connect_to_database_server(
    database_dsn: &str,
) -> Result<(kodegen_mcp_client::KodegenConnection, common::ServerHandle)> {
    let workspace_root = common::find_workspace_root()
        .context("Failed to find workspace root")?;

    // Spawn SSE server with database connection
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.args([
        "run", "--package", "kodegen", "--bin", "kodegen",
        "--no-default-features", "--features", "database",
        "--",
        "--sse", "127.0.0.1:18080",
        "--tools", "database",
        "--database-dsn", database_dsn,
    ]);

    // Clean up any stale servers
    common::cleanup_port(18080).await.ok();

    eprintln!("🚀 Starting SSE server with database connection...");

    let child = cmd.spawn()
        .context("Failed to spawn SSE server")?;

    let server_handle = common::ServerHandle::new(child);

    // Wait for server to be ready
    eprintln!("⏳ Waiting for server and database connection...");
    let (_client, connection) = common::connect_with_retry(
        "http://127.0.0.1:18080/sse",
        Duration::from_secs(90),
        Duration::from_millis(500),
    )
    .await
    .context("Failed to connect to SSE server")?;

    Ok((connection, server_handle))
}

async fn run_database_example(client: &common::LoggingClient) -> Result<()> {
    info!("\n{:=<70}", "");
    info!(" DATABASE TOOLS EXAMPLE");
    info!("{:=<70}\n", "");
    info!("This example demonstrates all 7 database tools:");
    info!("  1. list_schemas - Discover available databases/schemas");
    info!("  2. list_tables - List tables in a schema");
    info!("  3. get_table_schema - Inspect table columns");
    info!("  4. get_table_indexes - View table indexes");
    info!("  5. execute_sql (SELECT) - Query data");
    info!("  6. execute_sql (JOIN) - Complex multi-table queries");
    info!("  7. get_pool_stats - Monitor connection health");
    info!("  8. get_stored_procedures - List functions/procedures");
    info!("");

    test_database_tools(client).await?;

    info!("\n{:=<70}", "");
    info!(" ALL TESTS COMPLETE");
    info!("{:=<70}", "");
    info!("✅ Successfully demonstrated all 7 database tools");
    Ok(())
}

async fn test_database_tools(client: &common::LoggingClient) -> Result<()> {
    info!("\n{:=<70}", "");
    info!(" Testing PostgreSQL Database");
    info!("{:=<70}", "");
    
    // Tool 1: LIST_SCHEMAS
    info!("\n[1/8] Testing list_schemas...");
    client.call_tool(
        tools::LIST_SCHEMAS,
        json!({})
    )
    .await
    .context("list_schemas failed")?;
    info!("✅ list_schemas completed");
    
    // Tool 2: LIST_TABLES  
    info!("\n[2/8] Testing list_tables...");
    client.call_tool(
        tools::LIST_TABLES,
        json!({})
    )
    .await
    .context("list_tables failed")?;
    info!("✅ list_tables completed");
    
    // Tool 3: GET_TABLE_SCHEMA
    info!("\n[3/8] Testing get_table_schema on 'employees' table...");
    client.call_tool(
        tools::GET_TABLE_SCHEMA,
        json!({ "table": "employees" })
    )
    .await
    .context("get_table_schema failed")?;
    info!("✅ get_table_schema completed");
    
    // Tool 4: GET_TABLE_INDEXES
    info!("\n[4/8] Testing get_table_indexes on 'employees' table...");
    client.call_tool(
        tools::GET_TABLE_INDEXES,
        json!({ "table": "employees" })
    )
    .await
    .context("get_table_indexes failed")?;
    info!("✅ get_table_indexes completed");
    
    // Tool 5: EXECUTE_SQL (SELECT)
    info!("\n[5/8] Testing execute_sql with SELECT...");
    client.call_tool(
        tools::EXECUTE_SQL,
        json!({ 
            "sql": "SELECT id, name, CAST(budget AS TEXT) as budget, CAST(created_at AS TEXT) as created_at FROM departments LIMIT 3"
        })
    )
    .await
    .context("execute_sql (SELECT) failed")?;
    info!("✅ execute_sql (SELECT) completed");
    
    // Tool 6: EXECUTE_SQL (JOIN)
    info!("\n[6/8] Testing execute_sql with JOIN...");
    client.call_tool(
        tools::EXECUTE_SQL,
        json!({ 
            "sql": "SELECT e.name, e.email, d.name as department, CAST(e.hire_date AS TEXT) as hire_date \
                    FROM employees e \
                    JOIN departments d ON e.department_id = d.id \
                    LIMIT 5"
        })
    )
    .await
    .context("execute_sql (JOIN) failed")?;
    info!("✅ execute_sql (JOIN) completed");
    
    // Tool 7: GET_POOL_STATS
    info!("\n[7/8] Testing get_pool_stats...");
    client.call_tool(
        tools::GET_POOL_STATS,
        json!({})
    )
    .await
    .context("get_pool_stats failed")?;
    info!("✅ get_pool_stats completed");
    
    // Tool 8: GET_STORED_PROCEDURES
    info!("\n[8/8] Testing get_stored_procedures...");
    client.call_tool(
        tools::GET_STORED_PROCEDURES,
        json!({})
    )
    .await
    .context("get_stored_procedures failed")?;
    info!("✅ get_stored_procedures completed");
    
    Ok(())
}
