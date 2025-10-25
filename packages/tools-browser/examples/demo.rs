//! Browser tools demonstration
//!
//! Shows how to use browser automation tools to:
//! - Navigate to URLs
//! - Extract page content  
//! - Take screenshots
//! - Interact with elements (click, type)
//! - Control viewport (scroll, wait)

use anyhow::{Context, Result};
use serde_json::json;
use tracing::info;

mod common;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("🌐 Browser Tools Demo\n");
    info!("This example demonstrates browser automation capabilities.\n");

    // Connect to local browser SSE server
    let (conn, mut server) = common::connect_to_local_sse_server().await?;

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
    let result = client
        .call_tool(
            "browser_navigate",
            json!({
                "url": "https://example.com"
            }),
        )
        .await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        info!(
            "   ✓ Navigate result: {}",
            response
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        );
    }
    info!("");

    // Step 2: Wait for page to settle
    info!("2️⃣  Waiting 1 second for page to settle...");
    client
        .call_tool(
            "browser_wait",
            json!({
                "duration_ms": 1000
            }),
        )
        .await?;
    info!("   ✓ Wait complete");
    info!("");

    // Step 3: Extract page text
    info!("3️⃣  Extracting page text...");
    let result = client.call_tool("browser_extract_text", json!({})).await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
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
    let result = client
        .call_tool(
            "browser_screenshot",
            json!({
                "format": "png"
            }),
        )
        .await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        let size = response
            .get("size_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        info!("   ✓ Screenshot captured: {} bytes", size);
    }
    info!("");

    // Step 5: Scroll down
    info!("5️⃣  Scrolling down 500px...");
    client
        .call_tool(
            "browser_scroll",
            json!({
                "y": 500
            }),
        )
        .await?;
    info!("   ✓ Scroll complete");
    info!("");

    // Step 6: Navigate to DuckDuckGo
    info!("6️⃣  Navigating to duckduckgo.com...");
    client
        .call_tool(
            "browser_navigate",
            json!({
                "url": "https://duckduckgo.com"
            }),
        )
        .await?;
    info!("   ✓ Navigation complete");
    info!("");

    // Wait for search page to load
    info!("   Waiting for page to load...");
    client
        .call_tool(
            "browser_wait",
            json!({
                "duration_ms": 2000
            }),
        )
        .await?;

    // Step 7: Type search query
    info!("7️⃣  Typing search query into search field...");
    client
        .call_tool(
            "browser_type_text",
            json!({
                "selector": "input[name='q']",
                "text": "Rust programming language"
            }),
        )
        .await?;
    info!("   ✓ Search query entered");
    info!("");

    // Step 8: Submit the search
    info!("8️⃣  Clicking search button...");
    client
        .call_tool(
            "browser_click",
            json!({
                "selector": "button[type='submit']"
            }),
        )
        .await?;
    info!("   ✓ Search submitted");
    info!("");

    // Wait for results to load
    info!("   Waiting for search results...");
    client
        .call_tool(
            "browser_wait",
            json!({
                "duration_ms": 3000
            }),
        )
        .await?;

    // Step 9: Extract and describe search results
    info!("9️⃣  Extracting search results...");
    let result = client.call_tool("browser_extract_text", json!({})).await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        let extracted = response.get("text").and_then(|v| v.as_str()).unwrap_or("");

        // Look for search result indicators
        let has_results = extracted.contains("rust-lang.org")
            || extracted.contains("Rust")
            || extracted.contains("programming");

        if has_results {
            let preview = if extracted.len() > 500 {
                format!("{}...", &extracted[..500])
            } else {
                extracted.to_string()
            };
            info!("   ✓ Search results found ({} chars):", extracted.len());
            info!("   {}", preview);
        } else {
            info!("   ⚠ Unexpected results (may need selector adjustment)");
            info!("   First 300 chars: {}", &extracted[..300.min(extracted.len())]);
        }
    }
    info!("");

    info!("✅ Browser demo complete!");
    info!("\nAll browser tools demonstrated:");
    info!("  - browser_navigate: Load URLs (example.com, duckduckgo.com)");
    info!("  - browser_wait: Pause for page loads");
    info!("  - browser_extract_text: Get page content and search results");
    info!("  - browser_screenshot: Capture page images");
    info!("  - browser_scroll: Navigate within pages");
    info!("  - browser_type_text: Enter search queries");
    info!("  - browser_click: Submit forms and interact with elements");
    info!("\nExample workflow: Navigate → Search DuckDuckGo → Extract results");

    Ok(())
}
