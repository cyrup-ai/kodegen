mod common;

use anyhow::Context;
use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting prompt tools example");

    // Connect to kodegen server with prompt category
    let (conn, mut server) =
        common::connect_to_local_sse_server().await?;

    // Wrap client with logging
    let log_path =
        std::path::PathBuf::from("/Volumes/samsung_t9/kodegen/tmp/mcp-client/prompt.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    let test_prompt_name = "test_prompt_example";

    // 1. ADD_PROMPT - Add a new prompt
    info!("1. Testing add_prompt");
    match client
        .call_tool(
            tools::ADD_PROMPT,
            json!({
                "name": test_prompt_name,
                "description": "A test prompt for demonstration",
                "content": "This is a test prompt content: {{variable}}"
            }),
        )
        .await
    {
        Ok(result) => info!("Added prompt: {:?}", result),
        Err(e) => error!("Failed to add prompt: {}", e),
    }

    // 2. GET_PROMPT - Retrieve the prompt
    info!("2. Testing get_prompt");
    match client
        .call_tool(tools::GET_PROMPT, json!({ "name": test_prompt_name }))
        .await
    {
        Ok(result) => info!("Got prompt: {:?}", result),
        Err(e) => error!("Failed to get prompt: {}", e),
    }

    // 3. EDIT_PROMPT - Edit the prompt
    info!("3. Testing edit_prompt");
    match client
        .call_tool(
            tools::EDIT_PROMPT,
            json!({
                "name": test_prompt_name,
                "description": "Updated test prompt description",
                "content": "This is updated content: {{variable}}"
            }),
        )
        .await
    {
        Ok(result) => info!("Edited prompt: {:?}", result),
        Err(e) => error!("Failed to edit prompt: {}", e),
    }

    // 4. DELETE_PROMPT - Delete the prompt
    info!("4. Testing delete_prompt");
    match client
        .call_tool(tools::DELETE_PROMPT, json!({ "name": test_prompt_name }))
        .await
    {
        Ok(result) => info!("Deleted prompt: {:?}", result),
        Err(e) => error!("Failed to delete prompt: {}", e),
    }

    // Graceful shutdown
    conn.close().await?;
    server.shutdown().await?;
    info!("Prompt tools example completed successfully");

    Ok(())
}
