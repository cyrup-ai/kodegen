mod common;

use anyhow::Context;
use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting introspection tools example");

    // Connect to kodegen server with introspection category
    let (conn, mut server) =
        common::connect_to_local_sse_server().await?;
            .await?;

    // Wrap client with logging
    let log_path =
        std::path::PathBuf::from("/Volumes/samsung_t9/kodegen/tmp/mcp-client/introspection.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // 1. GET_USAGE_STATS - Get usage statistics
    info!("1. Testing get_usage_stats");
    match client.call_tool(tools::GET_USAGE_STATS, json!({})).await {
        Ok(result) => info!("Usage stats: {:?}", result),
        Err(e) => error!("Failed to get usage stats: {}", e),
    }

    // 2. GET_RECENT_TOOL_CALLS - Get recent tool call history
    info!("2. Testing get_recent_tool_calls");
    match client
        .call_tool(tools::GET_RECENT_TOOL_CALLS, json!({ "max_results": 10 }))
        .await
    {
        Ok(result) => info!("Recent tool calls: {:?}", result),
        Err(e) => error!("Failed to get recent tool calls: {}", e),
    }

    // Graceful shutdown
    conn.close().await?;
    server.shutdown().await?;
    info!("Introspection tools example completed successfully");

    Ok(())
}
