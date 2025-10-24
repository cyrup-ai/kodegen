//! Browser tools demonstration
//!
//! Shows how to use browser automation tools to:
//! - Navigate to URLs
//! - Extract page content  
//! - Take screenshots
//! - Interact with elements (click, type)
//! - Control viewport (scroll, wait)

use anyhow::{Result, Context};
use serde_json::json;
use tracing::info;

mod common;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🌐 Browser Tools Demo\n");
    info!("This example demonstrates browser automation capabilities.\n");

    // Connect to kodegen server with browser category
    let (conn, mut server) = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Browser])
    ).await?;

    // Wrap client with logging
    let log_path = std::path::PathBuf::from("/tmp/mcp-client/browser_demo.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // Run example with cleanup
    let result = run_browser_example(&client).await;

    // Always close connection, regardless of example result
    conn.close().await?;
    server.shutdown().await?;

    // Propagate any error from the example
    result
}

async fn run_browser_example(client: &common::LoggingClient) -> Result<()> {
    // Step 1: Navigate to example.com
    info!("1️⃣  Navigating to example.com...");
    let result = client.call_tool(
        "browser_navigate",
        json!({
            "url": "https://example.com"
        })
    ).await?;
    
    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text() {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        info!("   ✓ Navigate result: {}", response.get("url").and_then(|v| v.as_str()).unwrap_or("unknown"));
    }
    info!("");

    // Step 2: Wait for page to settle
    info!("2️⃣  Waiting 1 second for page to settle...");
    client.call_tool(
        "browser_wait",
        json!({
            "duration_ms": 1000
        })
    ).await?;
    info!("   ✓ Wait complete");
    info!("");

    // Step 3: Extract page text
    info!("3️⃣  Extracting page text...");
    let result = client.call_tool(
        "browser_extract_text",
        json!({})
    ).await?;
    
    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text() {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        let extracted = response.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let preview = if extracted.len() > 200 {
            format!("{}...", &extracted[..200])
        } else {
            extracted.to_string()
        };
        info!("   ✓ Extracted {} chars: {}", extracted.len(), preview);
    }
    info!("");

    // Step 4: Take screenshot
    info!("4️⃣  Taking screenshot...");
    let result = client.call_tool(
        "browser_screenshot",
        json!({
            "format": "png"
        })
    ).await?;
    
    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text() {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        let size = response.get("size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
        info!("   ✓ Screenshot captured: {} bytes", size);
    }
    info!("");

    // Step 5: Scroll down
    info!("5️⃣  Scrolling down 500px...");
    client.call_tool(
        "browser_scroll",
        json!({
            "y": 500
        })
    ).await?;
    info!("   ✓ Scroll complete");
    info!("");

    // Step 6: Navigate to another page with form
    info!("6️⃣  Navigating to httpbin.org/forms/post...");
    client.call_tool(
        "browser_navigate",
        json!({
            "url": "https://httpbin.org/forms/post"
        })
    ).await?;
    info!("   ✓ Navigation complete");
    info!("");

    // Step 7: Type into a form field
    info!("7️⃣  Typing into custname field...");
    client.call_tool(
        "browser_type_text",
        json!({
            "selector": "input[name='custname']",
            "text": "Test User"
        })
    ).await?;
    info!("   ✓ Text typed");
    info!("");

    // Step 8: Click a checkbox
    info!("8️⃣  Clicking size checkbox...");
    client.call_tool(
        "browser_click",
        json!({
            "selector": "input[value='small']"
        })
    ).await?;
    info!("   ✓ Element clicked");
    info!("");

    info!("✅ Browser demo complete!");
    info!("\nAll browser tools demonstrated:");
    info!("  - browser_navigate: Load URLs");
    info!("  - browser_wait: Pause execution");
    info!("  - browser_extract_text: Get page content");
    info!("  - browser_screenshot: Capture images");
    info!("  - browser_scroll: Scroll pages");
    info!("  - browser_type_text: Input text");
    info!("  - browser_click: Click elements");

    Ok(())
}
