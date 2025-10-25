//! Browser tools comprehensive demonstration
//!
//! Demonstrates all 9 public browser tools using real-world examples:
//! - Workflow 1: docs.rs search (7 tools)
//! - Workflow 2: Web search (1 tool)
//! - Workflow 3: AI research (1 tool)
//! - Workflow 4: Autonomous agent (1 tool)

use anyhow::{Context, Result};
use serde_json::json;
use tracing::info;

mod common;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("🌐 Browser Tools Comprehensive Demo\n");
    info!("Demonstrating all 9 public browser tools\n");

    // Connect to local browser SSE server
    let (conn, mut server) = common::connect_to_local_sse_server().await?;

    // Wrap client with logging
    let workspace_root = common::find_workspace_root()
        .context("Failed to find workspace root")?;
    let log_path = workspace_root.join("tmp/mcp-client/browser.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    // Run all workflows
    let result = run_all_workflows(&client).await;

    // Always close connection
    conn.close().await?;
    server.shutdown().await?;

    result
}

async fn run_all_workflows(client: &common::LoggingClient) -> Result<()> {
    // ========================================================================
    // Workflow 1: docs.rs Search - 7 Tools
    // ========================================================================
    info!("\n╔══════════════════════════════════════════════════════════╗");
    info!("║ Workflow 1: docs.rs Search                              ║");
    info!("║ Tools: navigate, click x2, type_text, extract_text,     ║");
    info!("║        scroll, screenshot                                ║");
    info!("╚══════════════════════════════════════════════════════════╝\n");

    // Step 1: Navigate to docs.rs
    info!("1️⃣  browser_navigate → docs.rs");
    client
        .call_tool(
            "browser_navigate",
            json!({
                "url": "https://docs.rs"
            }),
        )
        .await?;
    info!("   ✓ Navigated to docs.rs\n");

    // Step 2: Click into search field
    info!("2️⃣  browser_click → Search field");
    client
        .call_tool(
            "browser_click",
            json!({
                "selector": "input[type=\"search\"], input[name=\"query\"], .search-input"
            }),
        )
        .await?;
    info!("   ✓ Clicked search field\n");

    // Step 3: Type search query
    info!("3️⃣  browser_type_text → \"async\"");
    client
        .call_tool(
            "browser_type_text",
            json!({
                "selector": "input[type=\"search\"], input[name=\"query\"], .search-input",
                "text": "async"
            }),
        )
        .await?;
    info!("   ✓ Typed search query\n");

    // Step 4: Click submit/search button
    info!("4️⃣  browser_click → Submit button");
    client
        .call_tool(
            "browser_click",
            json!({
                "selector": "button[type=\"submit\"], .search-button, form button"
            }),
        )
        .await?;
    info!("   ✓ Submitted search\n");

    // Step 5: Extract search results
    info!("5️⃣  browser_extract_text → Search results");
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
        info!("   ✓ Extracted {} chars", extracted.len());
        info!("   Preview: {}\n", preview);
    }

    // Step 6: Scroll down
    info!("6️⃣  browser_scroll → Scroll down 500px");
    client
        .call_tool(
            "browser_scroll",
            json!({
                "y": 500
            }),
        )
        .await?;
    info!("   ✓ Scrolled down\n");

    // Step 7: Take screenshot
    info!("7️⃣  browser_screenshot → Capture results");
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
        info!("   ✓ Screenshot: {} bytes\n", size);
    }

    // ========================================================================
    // Workflow 2: Web Search - 1 Tool
    // ========================================================================
    info!("\n╔══════════════════════════════════════════════════════════╗");
    info!("║ Workflow 2: Web Search (DuckDuckGo)                     ║");
    info!("║ Tool: web_search                                         ║");
    info!("╚══════════════════════════════════════════════════════════╝\n");

    info!("8️⃣  web_search → \"Rust MCP server examples\"");
    let result = client
        .call_tool(
            "web_search",
            json!({
                "query": "Rust MCP server examples"
            }),
        )
        .await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;
        let result_count = response
            .get("result_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        info!("   ✓ Found {} search results", result_count);

        if let Some(results) = response.get("results").and_then(|v| v.as_array()) {
            info!("   Top 3 results:");
            for (i, r) in results.iter().take(3).enumerate() {
                let title = r.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let url = r.get("url").and_then(|v| v.as_str()).unwrap_or("Unknown");
                info!("   {}. {} - {}", i + 1, title, url);
            }
        }
    }
    info!("");

    // ========================================================================
    // Workflow 3: AI-Powered Research - 1 Tool
    // ========================================================================
    info!("\n╔══════════════════════════════════════════════════════════╗");
    info!("║ Workflow 3: AI-Powered Deep Research                    ║");
    info!("║ Tool: browser_research                                   ║");
    info!("╚══════════════════════════════════════════════════════════╝\n");

    info!("9️⃣  browser_research → \"precedent setting USA Antitrust cases\"");
    info!("   (Multi-page research with AI summarization)\n");

    let result = client
        .call_tool(
            "browser_research",
            json!({
                "query": "precedent setting USA Antitrust cases",
                "max_pages": 5,
                "summarize": true
            }),
        )
        .await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;

        if let Some(summary) = response.get("summary").and_then(|v| v.as_str()) {
            info!("   ✓ AI Summary:");
            info!("\n{}\n", summary);
        }

        if let Some(sources) = response.get("sources").and_then(|v| v.as_array()) {
            info!("   📚 Sources ({} pages):", sources.len());
            for (i, source) in sources.iter().enumerate().take(5) {
                if let Some(url) = source.get("url").and_then(|v| v.as_str()) {
                    info!("   {}. {}", i + 1, url);
                }
            }
        }
    }
    info!("");

    // ========================================================================
    // Workflow 4: Autonomous Browser Agent - 1 Tool
    // ========================================================================
    info!("\n╔══════════════════════════════════════════════════════════╗");
    info!("║ Workflow 4: Autonomous AI Agent                         ║");
    info!("║ Tool: browser_agent                                      ║");
    info!("╚══════════════════════════════════════════════════════════╝\n");

    info!("🔟  browser_agent → Compare axum vs actix-web");
    info!("   (AI autonomously navigates and extracts data)\n");

    let result = client
        .call_tool(
            "browser_agent",
            json!({
                "task": "Compare axum vs actix-web crates on crates.io - find downloads, latest version, and key features for each",
                "start_url": "https://crates.io",
                "max_steps": 10,
                "temperature": 0.3
            }),
        )
        .await?;

    if let Some(content) = result.content.first()
        && let Some(text) = content.as_text()
    {
        let response: serde_json::Value = serde_json::from_str(&text.text)?;

        let success = response.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let steps_taken = response.get("steps_taken").and_then(|v| v.as_u64()).unwrap_or(0);

        info!("   {} Agent completed in {} steps",
            if success { "✓" } else { "⚠" },
            steps_taken
        );

        if let Some(final_result) = response.get("final_result").and_then(|v| v.as_str()) {
            info!("\n   Result:\n{}\n", final_result);
        }

        if let Some(actions) = response.get("actions").and_then(|v| v.as_array()) {
            info!("   Actions taken:");
            for action in actions {
                if let Some(step) = action.get("step").and_then(|v| v.as_u64()) {
                    if let Some(summary) = action.get("summary").and_then(|v| v.as_str()) {
                        info!("   Step {}: {}", step, summary);
                    }
                }
            }
        }
    }
    info!("");

    // ========================================================================
    // Summary
    // ========================================================================
    info!("\n╔══════════════════════════════════════════════════════════╗");
    info!("║ ✅ All 9 Browser Tools Demonstrated                      ║");
    info!("╠══════════════════════════════════════════════════════════╣");
    info!("║ Core Automation (6 tools):                              ║");
    info!("║   ✓ browser_navigate    ✓ browser_click                 ║");
    info!("║   ✓ browser_type_text   ✓ browser_extract_text          ║");
    info!("║   ✓ browser_scroll      ✓ browser_screenshot            ║");
    info!("║                                                          ║");
    info!("║ Advanced Tools (3 tools):                               ║");
    info!("║   ✓ web_search          ✓ browser_research              ║");
    info!("║   ✓ browser_agent                                        ║");
    info!("╚══════════════════════════════════════════════════════════╝\n");

    Ok(())
}
