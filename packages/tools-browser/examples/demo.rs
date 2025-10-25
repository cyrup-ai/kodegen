//! Browser tools demonstration
//!
//! Part 1: Crates.io exploration - Real-world Rust library search
//! Part 2: Legal research - AI-powered multi-page research on antitrust cases
//!
//! Demonstrates:
//! - Basic browser automation (navigate, click, type, extract, scroll, screenshot)
//! - Advanced AI research capabilities (browser_research with deep analysis)

use anyhow::{Context, Result};
use serde_json::json;
use tracing::info;

mod common;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("🌐 Browser Tools Demo\n");
    info!("Part 1: Crates.io library exploration");
    info!("Part 2: AI-powered legal research\n");

    // Connect to local browser SSE server
    let (conn, mut server) = common::connect_to_local_sse_server().await?;

    // Wrap client with logging
    let workspace_root = common::find_workspace_root()
        .context("Failed to find workspace root")?;
    let log_path = workspace_root.join("tmp/mcp-client/browser.log");
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
    info!("\n═══════════════════════════════════════");
    info!("PART 1: Crates.io Library Exploration");
    info!("═══════════════════════════════════════\n");

    // Step 1: Navigate to crates.io
    info!("1️⃣  Navigating to crates.io...");
    let result = client
        .call_tool(
            "browser_navigate",
            json!({
                "url": "https://crates.io"
            }),
        )
        .await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        info!(
            "   ✓ Loaded: {}",
            response
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        );
    }
    info!("");

    // Step 2: Extract crates.io homepage content
    info!("2️⃣  Extracting homepage content...");
    client
        .call_tool(
            "browser_wait",
            json!({
                "duration_ms": 1500
            }),
        )
        .await?;
    
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
        info!("   ✓ Extracted homepage ({} chars): {}", extracted.len(), preview);
    }
    info!("");

    // Step 3: Navigate to tokio crate page
    info!("3️⃣  Navigating to tokio crate page...");
    client
        .call_tool(
            "browser_navigate",
            json!({
                "url": "https://crates.io/crates/tokio"
            }),
        )
        .await?;
    info!("   ✓ Loaded tokio crate page");
    info!("");

    // Wait for crate page to load
    client
        .call_tool(
            "browser_wait",
            json!({
                "duration_ms": 2000
            }),
        )
        .await?;

    // Step 4: Extract crate details
    info!("4️⃣  Extracting tokio crate information...");
    let result = client.call_tool("browser_extract_text", json!({})).await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        let extracted = response.get("text").and_then(|v| v.as_str()).unwrap_or("");
        
        // Look for tokio-specific content
        let has_tokio_info = extracted.contains("tokio") || extracted.contains("async") || extracted.contains("runtime");
        
        if has_tokio_info {
            let preview = if extracted.len() > 400 {
                format!("{}...", &extracted[..400])
            } else {
                extracted.to_string()
            };
            info!("   ✓ Extracted crate details ({} chars)", extracted.len());
            info!("   {}", preview);
        }
    }
    info!("");

    // Step 5: Scroll down to see more details
    info!("5️⃣  Scrolling to view README...");
    client
        .call_tool(
            "browser_scroll",
            json!({
                "y": 800
            }),
        )
        .await?;
    info!("   ✓ Scrolled to README section");
    info!("");

    // Step 6: Take screenshot of crate page
    info!("6️⃣  Taking screenshot of tokio crate page...");
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

    info!("\n═══════════════════════════════════════");
    info!("PART 2: AI-Powered Legal Research");
    info!("═══════════════════════════════════════\n");

    // Step 7: Use browser_research for deep multi-page legal research
    info!("7️⃣  Researching 'most important US antitrust precedent cases'...");
    info!("   (This will search, navigate multiple sources, and generate AI summary)\n");
    
    let research_result = client
        .call_tool(
            "browser_research",
            json!({
                "query": "most important US antitrust precedent setting cases",
                "max_pages": 5,
                "summarize": true
            }),
        )
        .await?;

    if let Some(content) = research_result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        
        if let Some(summary) = response.get("summary").and_then(|v| v.as_str()) {
            info!("   ✓ Research complete! AI-generated summary:");
            info!("\n{}\n", summary);
        }
        
        if let Some(sources) = response.get("sources").and_then(|v| v.as_array()) {
            info!("   📚 Sources consulted ({} pages):", sources.len());
            for (i, source) in sources.iter().enumerate().take(5) {
                if let Some(url) = source.get("url").and_then(|v| v.as_str()) {
                    info!("      {}. {}", i + 1, url);
                }
            }
        }
    }
    info!("");

    info!("✅ Browser demo complete!\n");
    info!("═══════════════════════════════════════");
    info!("Tools Demonstrated");
    info!("═══════════════════════════════════════");
    info!("\nBasic Automation (7 tools):");
    info!("  ✓ browser_navigate - Navigate to crates.io and tokio page");
    info!("  ✓ browser_wait - Wait for page loads");
    info!("  ✓ browser_type_text - Enter search queries");
    info!("  ✓ browser_click - Submit search forms");
    info!("  ✓ browser_extract_text - Extract search results and crate info");
    info!("  ✓ browser_scroll - View more content on page");
    info!("  ✓ browser_screenshot - Capture tokio crate page");
    info!("\nAdvanced AI Research (1 tool):");
    info!("  ✓ browser_research - Multi-page legal research with AI summary");
    info!("\nWorkflow Examples:");
    info!("  1. Crates.io: Search → Navigate → Extract → Screenshot");
    info!("  2. Legal Research: Query → Multi-page crawl → AI summarization\n");

    Ok(())
}
