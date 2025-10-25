//! Simple browser navigation example
//!
//! Demonstrates basic navigation and text extraction.

use anyhow::{Context, Result};
use serde_json::json;
use tracing::info;

mod common;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    // Connect to local browser SSE server
    let (conn, mut server) = common::connect_to_local_sse_server().await?;

    // Wrap client with logging
    let workspace_root = common::find_workspace_root()
        .context("Failed to find workspace root")?;
    let log_path = workspace_root.join("tmp/mcp-client/browser.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    // Run example with cleanup
    let result = run_example(&client).await;

    // Always close connection
    conn.close().await?;
    server.shutdown().await?;

    result
}

async fn run_example(client: &common::LoggingClient) -> Result<()> {
    info!("🌐 Navigating to httpbin.org/html...");
    let result = client
        .call_tool(
            "browser_navigate",
            json!({
                "url": "https://httpbin.org/html"
            }),
        )
        .await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        info!("✓ Navigation successful!");
        info!(
            "  URL: {}",
            response
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        );
    }

    info!("\n📄 Extracting page text...");
    let result = client.call_tool("browser_extract_text", json!({})).await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        let extracted = response.get("text").and_then(|v| v.as_str()).unwrap_or("");
        info!("✓ Extracted {} characters", extracted.len());
        info!("\nPage content preview:");
        info!("{}", &extracted[..extracted.len().min(300)]);
    }

    Ok(())
}
