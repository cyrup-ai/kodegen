//! Database tools example - demonstrates all 7 database tools across 4 database types
//!
//! This example follows the pattern from filesystem.rs and shows how to:
//! - Connect to databases via connection strings
//! - List schemas and tables
//! - Introspect table structure (columns, indexes)
//! - Execute SQL queries
//! - Monitor connection pool health
//! - Query stored procedures (PostgreSQL/MySQL only)

mod common;

use anyhow::Context;
use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting database tools example");

    // Connect to kodegen server with database category
    let (conn, mut server) = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Database])
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

async fn run_database_example(client: &common::LoggingClient) -> anyhow::Result<()> {
    info!("\n{:=<70}", "");
    info!(" DATABASE TOOLS EXAMPLE");
    info!("{:=<70}\n", "");
    info!("This example demonstrates all 7 database tools across 4 database types:");
    info!("  1. list_schemas - Discover available databases/schemas");
    info!("  2. list_tables - List tables in a schema");
    info!("  3. get_table_schema - Inspect table columns");
    info!("  4. get_table_indexes - View table indexes");
    info!("  5. execute_sql (SELECT) - Query data");
    info!("  6. execute_sql (JOIN) - Complex multi-table queries");
    info!("  7. get_pool_stats - Monitor connection health");
    info!("  8. get_stored_procedures - List functions/procedures (PostgreSQL/MySQL only)");
    info!("");

    // Test all 4 database types
    for db_type in &["postgres", "mysql", "mariadb", "sqlite"] {
        info!("\n{:=<70}", "");
        info!(" Testing {}", db_type.to_uppercase());
        info!("{:=<70}", "");
        match test_database_tools(client, db_type).await {
            Ok(_) => info!("✅ All tests passed for {}\n", db_type),
            Err(e) => {
                tracing::error!("❌ Tests failed for {}: {}\n", db_type, e);
                return Err(e);
            }
        }
    }

    info!("\n{:=<70}", "");
    info!(" ALL TESTS COMPLETE");
    info!("{:=<70}", "");
    info!("✅ Successfully demonstrated all 7 database tools across 4 database types");
    Ok(())
}

async fn test_database_tools(client: &common::LoggingClient, db_type: &str) -> anyhow::Result<()> {
    let dsn = get_dsn(db_type);
    info!("Connecting to: {}", dsn);
    
    // Tool 1: LIST_SCHEMAS
    info!("\n[1/8] Testing list_schemas...");
    client.call_tool(
        tools::LIST_SCHEMAS,
        json!({ "dsn": dsn })
    )
    .await
    .context("list_schemas failed")?;
    info!("✅ list_schemas completed");
    
    // Tool 2: LIST_TABLES  
    info!("\n[2/8] Testing list_tables...");
    client.call_tool(
        tools::LIST_TABLES,
        json!({ "dsn": dsn })
    )
    .await
    .context("list_tables failed")?;
    info!("✅ list_tables completed");
    
    // Tool 3: GET_TABLE_SCHEMA
    info!("\n[3/8] Testing get_table_schema on 'employees' table...");
    client.call_tool(
        tools::GET_TABLE_SCHEMA,
        json!({ "dsn": dsn, "table_name": "employees" })
    )
    .await
    .context("get_table_schema failed")?;
    info!("✅ get_table_schema completed");
    
    // Tool 4: GET_TABLE_INDEXES
    info!("\n[4/8] Testing get_table_indexes on 'employees' table...");
    client.call_tool(
        tools::GET_TABLE_INDEXES,
        json!({ "dsn": dsn, "table_name": "employees" })
    )
    .await
    .context("get_table_indexes failed")?;
    info!("✅ get_table_indexes completed");
    
    // Tool 5: EXECUTE_SQL (SELECT)
    info!("\n[5/8] Testing execute_sql with SELECT...");
    client.call_tool(
        tools::EXECUTE_SQL,
        json!({ 
            "dsn": dsn,
            "sql": "SELECT * FROM departments LIMIT 3"
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
            "dsn": dsn,
            "sql": "SELECT e.name, e.email, d.name as department \
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
        json!({ "dsn": dsn })
    )
    .await
    .context("get_pool_stats failed")?;
    info!("✅ get_pool_stats completed");
    
    // Tool 8: GET_STORED_PROCEDURES (skip for SQLite - not supported)
    if db_type != "sqlite" {
        info!("\n[8/8] Testing get_stored_procedures...");
        client.call_tool(
            tools::GET_STORED_PROCEDURES,
            json!({ "dsn": dsn })
        )
        .await
        .context("get_stored_procedures failed")?;
        info!("✅ get_stored_procedures completed");
    } else {
        info!("\n[8/8] Skipping get_stored_procedures (SQLite does not support stored procedures)");
    }
    
    Ok(())
}


fn get_dsn(db_type: &str) -> String {
    match db_type {
        "postgres" => "postgres://testuser:testpass@localhost:5432/testdb".to_string(),
        "mysql" => "mysql://testuser:testpass@localhost:3306/testdb".to_string(),
        "mariadb" => "mysql://testuser:testpass@localhost:3307/testdb".to_string(),
        _ => {  // "sqlite" and any other case
            // For SQLite, use a temporary file instead of :memory:
            // This allows connection pooling to work correctly
            let temp_dir = std::env::temp_dir();
            let db_path = temp_dir.join("kodegen_test.db");
            format!("sqlite://{}", db_path.display())
        }
    }
}
