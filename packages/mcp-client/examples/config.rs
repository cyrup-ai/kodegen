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

    info!("Starting config tools example");

    // Connect to kodegen server (config tools are always enabled, no category needed)
    let conn = common::connect_to_server_with_categories(None)
        .await?;
    let client = conn.client();

    info!("Connected to server: {:?}", client.server_info());

    // 1. GET_CONFIG - Get current configuration
    info!("1. Testing get_config");
    match client.call_tool(tools::GET_CONFIG, json!({})).await {
        Ok(result) => info!("Current config: {:?}", result),
        Err(e) => error!("Failed to get config: {}", e),
    }

    // 2. SET_CONFIG_VALUE - Set a configuration value
    info!("2. Testing set_config_value");
    match client.call_tool(
        tools::SET_CONFIG_VALUE,
        json!({
            "key": "file_read_line_limit",
            "value": 2000
        })
    ).await {
        Ok(result) => info!("Set config value: {:?}", result),
        Err(e) => error!("Failed to set config value: {}", e),
    }

    // Verify the change by getting config again
    info!("Verifying config change");
    match client.call_tool(tools::GET_CONFIG, json!({})).await {
        Ok(result) => info!("Updated config: {:?}", result),
        Err(e) => error!("Failed to get updated config: {}", e),
    }

    // Graceful shutdown
    conn.close().await?;
    info!("Config tools example completed successfully");

    Ok(())
}
