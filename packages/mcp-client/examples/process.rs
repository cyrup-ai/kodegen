mod common;

use anyhow::Context;
use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting process tools example");

    // Connect to kodegen server with process category
    let (conn, mut server) =
        common::connect_to_server_with_categories(Some(vec![common::ToolCategory::Process]))
            .await?;

    // Wrap client with logging
    let log_path =
        std::path::PathBuf::from("/Volumes/samsung_t9/kodegen/tmp/mcp-client/process.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // 1. LIST_PROCESSES - List all running processes
    info!("1. Testing list_processes");
    match client.call_tool(tools::LIST_PROCESSES, json!({})).await {
        Ok(result) => {
            info!("Listed processes: {:?}", result);
            // In a real scenario, you would parse the result and extract process info
        }
        Err(e) => error!("Failed to list processes: {}", e),
    }

    // 2. KILL_PROCESS - Kill a process (demonstration only - not actually killing)
    info!("2. Testing kill_process (demo with invalid PID)");
    // Note: Using an invalid PID to demonstrate without actually killing anything
    match client
        .call_tool(
            tools::KILL_PROCESS,
            json!({ "pid": 999999 }), // Invalid PID for demo
        )
        .await
    {
        Ok(result) => info!("Kill process result: {:?}", result),
        Err(e) => info!("Expected error for invalid PID: {}", e),
    }

    // Graceful shutdown
    conn.close().await?;
    server.shutdown().await?;
    info!("Process tools example completed successfully");

    Ok(())
}
