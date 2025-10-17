mod common;

use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting introspection tools example");

    // Connect to kodegen server with introspection category
    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Introspection])
    ).await?;

    info!("Connected to server: {:?}", client.server_info());

    // 1. GET_USAGE_STATS - Get usage statistics
    info!("1. Testing get_usage_stats");
    match client.call_tool(tools::GET_USAGE_STATS, json!({})).await {
        Ok(result) => info!("Usage stats: {:?}", result),
        Err(e) => error!("Failed to get usage stats: {}", e),
    }

    // 2. GET_RECENT_TOOL_CALLS - Get recent tool call history
    info!("2. Testing get_recent_tool_calls");
    match client.call_tool(
        tools::GET_RECENT_TOOL_CALLS,
        json!({ "max_results": 10 })
    ).await {
        Ok(result) => info!("Recent tool calls: {:?}", result),
        Err(e) => error!("Failed to get recent tool calls: {}", e),
    }

    // Graceful shutdown
    client.close().await?;
    info!("Introspection tools example completed successfully");

    Ok(())
}
