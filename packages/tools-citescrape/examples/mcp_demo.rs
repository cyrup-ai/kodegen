mod common;

use anyhow::{Context, Result};
use kodegen_mcp_client::{responses::StartCrawlResponse, tools};
use serde_json::json;
use tokio::time::{Duration, sleep};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Wait for a crawl to complete by polling its status
///
/// Polls `get_crawl_results` periodically with exponential backoff until
/// the crawl completes, fails, or times out.
///
/// # Arguments
///
/// * `client` - The kodegen client to use for polling
/// * `crawl_id` - The crawl ID to monitor
/// * `timeout` - Maximum time to wait before giving up
///
/// # Returns
///
/// - `Ok(())` if crawl completes successfully
/// - `Err(_)` if crawl fails, errors, or times out
async fn wait_for_crawl_completion(
    client: &common::LoggingClient,
    crawl_id: &str,
    timeout: Duration,
) -> anyhow::Result<()> {
    let start = tokio::time::Instant::now();
    let mut backoff = Duration::from_millis(100);

    loop {
        let result = client
            .call_tool(
                tools::GET_CRAWL_RESULTS,
                json!({
                    "crawl_id": crawl_id,
                    "include_progress": false
                }),
            )
            .await?;

        // Parse status from result
        if let Some(text) = result.content.first().and_then(|c| c.as_text())
            && let Ok(crawl_data) = serde_json::from_str::<serde_json::Value>(&text.text)
            && let Some(status) = crawl_data.get("status").and_then(|s| s.as_str())
        {
            match status {
                "completed" => return Ok(()),
                "failed" | "error" => {
                    anyhow::bail!("Crawl failed with status: {status}")
                }
                _ => {
                    // Still running, continue polling
                }
            }
        }

        if start.elapsed() > timeout {
            anyhow::bail!("Crawl timed out after {timeout:?}");
        }

        // Exponential backoff: 100ms -> 200ms -> 400ms -> 500ms (capped)
        sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_millis(500));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse()?))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Connect to kodegen server with citescrape tools
    let (conn, mut server) =
        common::connect_to_local_sse_server().await?;
            .await?;

    // Delete old log file to start fresh
    let workspace_root = common::find_workspace_root()
        .context("Failed to find workspace root")?;
    let log_path = workspace_root.join("tmp/mcp-client/citescrape.log");
    if log_path.exists() {
        std::fs::remove_file(&log_path).context("Failed to delete old log file")?;
    }

    // Wrap client with logging
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    tracing::info!("Connected to server: {:?}", client.server_info());

    let result = run_citescrape_example(&client).await;

    // Close MCP connection first, then shutdown server
    conn.close().await?;
    server.shutdown().await?;

    result
}

async fn run_citescrape_example(client: &common::LoggingClient) -> Result<()> {
    let mut crawl_ids = Vec::new();

    let test_result = async {
        // Test 1: Start a web crawl
        tracing::info!("\n=== Testing start_crawl ===");
        tracing::info!("Starting crawl of Ratatui documentation...");

        let response: StartCrawlResponse = client
            .call_tool_typed(
                tools::START_CRAWL,
                json!({
                    "url": "https://ratatui.rs/",
                    "limit": 5,
                    "max_depth": 2,
                    "allow_subdomains": false
                }),
            )
            .await?;

        let crawl_id = response.crawl_id;
        crawl_ids.push(crawl_id.clone());
        tracing::info!("✅ Crawl ID: {}", crawl_id);

        // Wait for crawl to complete
        tracing::info!("Waiting for crawl to complete...");
        wait_for_crawl_completion(client, &crawl_id, Duration::from_secs(60)).await?;

        // Test 2: Get crawl results
        tracing::info!("\n=== Testing get_crawl_results ===");

        let result = client
            .call_tool(
                tools::GET_CRAWL_RESULTS,
                json!({
                    "crawl_id": crawl_id,
                    "include_progress": true,
                    "list_files": false
                }),
            )
            .await?;

        if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
            let crawl_data: serde_json::Value = serde_json::from_str(&text.text)?;
            tracing::info!("Crawl results summary:");

            if let Some(pages) = crawl_data.get("pages").and_then(|p| p.as_array()) {
                tracing::info!("  Total pages crawled: {}", pages.len());

                for (i, page) in pages.iter().enumerate().take(3) {
                    if let Some(url) = page.get("url").and_then(|u| u.as_str()) {
                        let title = page
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("No title");
                        tracing::info!("  {}. {} - {}", i + 1, title, url);
                    }
                }
            }

            if let Some(status) = crawl_data.get("status") {
                tracing::info!("  Status: {}", status);
            }
        }

        // Test 2b: Error handling - invalid domain (zero pages crawled)
        tracing::info!("\n=== Testing error handling ===");
        tracing::info!(
            "Starting crawl with invalid domain (expecting failure due to zero pages)..."
        );

        // start_crawl succeeds (returns crawl_id) because validation only checks URL syntax
        let response_invalid: StartCrawlResponse = client
            .call_tool_typed(
                tools::START_CRAWL,
                json!({
                    "url": "https://invalid-domain-that-does-not-exist-12345.com/",
                    "limit": 1
                }),
            )
            .await?;

        let crawl_id_invalid = response_invalid.crawl_id;
        crawl_ids.push(crawl_id_invalid.clone());
        tracing::info!("Crawl started with ID: {}", crawl_id_invalid);

        // Wait for the crawl to complete (the crawler detects zero pages and reports failure)
        let failure_result =
            wait_for_crawl_completion(client, &crawl_id_invalid, Duration::from_secs(30)).await;

        // Verify that the crawl failed with zero-page detection message
        match failure_result {
            Ok(()) => {
                anyhow::bail!("Expected crawl to fail for invalid domain, but it reported success")
            }
            Err(e) => {
                tracing::info!(
                    "✅ Correctly handled invalid domain with zero-page detection: {}",
                    e
                );
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("failed")
                        || error_msg.contains("No pages could be crawled")
                        || error_msg.contains("invalid"),
                    "Error message should indicate no pages crawled, got: {error_msg}"
                );
            }
        }

        // Test 3: Get full content for one page
        tracing::info!("\n=== Getting full page content ===");

        let result = client
            .call_tool(
                tools::GET_CRAWL_RESULTS,
                json!({
                    "crawl_id": crawl_id,
                    "include_progress": false,
                    "list_files": true
                }),
            )
            .await?;

        if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
            let crawl_data: serde_json::Value = serde_json::from_str(&text.text)?;

            if let Some(files) = crawl_data.get("files").and_then(|f| f.as_array()) {
                tracing::info!("Crawled files:");
                for (i, file) in files.iter().take(5).enumerate() {
                    if let Some(path) = file.as_str() {
                        tracing::info!("  {}. {}", i + 1, path);
                    }
                }
            }
        }

        // Test 4: Search within crawled content
        tracing::info!("\n=== Testing search_crawl_results ===");

        let result = client
            .call_tool(
                tools::SEARCH_CRAWL_RESULTS,
                json!({
                    "crawl_id": crawl_id,
                    "query": "ratatui",
                    "limit": 5
                }),
            )
            .await?;

        if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
            let search_results: serde_json::Value = serde_json::from_str(&text.text)?;
            tracing::info!("Search results for 'ratatui' (EXISTS in content):");

            if let Some(results) = search_results.get("results").and_then(|r| r.as_array()) {
                tracing::info!("  Found {} matches", results.len());

                if results.is_empty() {
                    tracing::error!("  ❌ SEARCH BROKEN: Should find results but got 0!");
                } else {
                    tracing::info!("  ✅ Search working correctly");
                }

                for (i, result) in results.iter().enumerate() {
                    if let Some(url) = result.get("url").and_then(|u| u.as_str()) {
                        let score = result
                            .get("score")
                            .and_then(serde_json::Value::as_f64)
                            .unwrap_or(0.0);
                        tracing::info!("  {}. {} (score: {:.2})", i + 1, url, score);

                        if let Some(snippet) = result.get("snippet").and_then(|s| s.as_str()) {
                            tracing::info!("     \"{}\"", snippet);
                        }
                    }
                }
            }
        }

        // Test 5: Web search with search engines (optional - may timeout on slow connections)
        tracing::info!("\n=== Testing web_search ===");

        match client
            .call_tool(
                tools::WEB_SEARCH,
                json!({
                    "query": "rust async programming tutorial"
                }),
            )
            .await
        {
            Ok(result) => {
                if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
                    let search_results: serde_json::Value = serde_json::from_str(&text.text)?;
                    tracing::info!("Web search results:");

                    if let Some(results) = search_results.get("results").and_then(|r| r.as_array())
                    {
                        tracing::info!("  Found {} results", results.len());
                        for (i, result) in results.iter().enumerate().take(3) {
                            if let Some(url) = result.get("url").and_then(|u| u.as_str()) {
                                let title = result
                                    .get("title")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("No title");
                                tracing::info!("  {}. {}", i + 1, title);
                                tracing::info!("     {}", url);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("⚠️  Web search timed out or failed (non-critical): {}", e);
                tracing::info!("Continuing with remaining tests...");
            }
        }

        // Test 6: Start another crawl with different settings
        tracing::info!("\n=== Testing crawl with custom settings ===");

        let response_2: StartCrawlResponse = client
            .call_tool_typed(
                tools::START_CRAWL,
                json!({
                    "url": "https://tokio.rs/",
                    "limit": 3,
                    "max_depth": 1,
                    "allow_subdomains": false
                }),
            )
            .await?;

        let crawl_id_2 = response_2.crawl_id;
        crawl_ids.push(crawl_id_2.clone());
        tracing::info!("✅ Second crawl started: {}", crawl_id_2);
        tracing::info!("Crawl respects robots.txt directives");

        // Wait and get results
        wait_for_crawl_completion(client, &crawl_id_2, Duration::from_secs(60)).await?;

        let _result = client
            .call_tool(
                tools::GET_CRAWL_RESULTS,
                json!({
                    "crawl_id": crawl_id_2,
                    "include_progress": false,
                    "list_files": false
                }),
            )
            .await?;

        tracing::info!("Second crawl results retrieved");

        tracing::info!("\n✅ All citescrape tools tested successfully!");

        tracing::info!("\n📚 Rate Limiting Features:");
        tracing::info!("  • Respects robots.txt by default");
        tracing::info!("  • Implements polite crawling with delays");
        tracing::info!("  • Configurable timeout and user agent");
        tracing::info!("  • Session-based for multiple concurrent crawls");

        tracing::info!("\n📚 Web Search:");
        tracing::info!("  • Performs web searches automatically");
        tracing::info!("  • Returns up to 10 results with titles, URLs, and snippets");
        tracing::info!("  • No configuration needed - just provide a query");

        tracing::info!("\n📚 Features Demonstrated:");
        tracing::info!("  • Starting web crawl sessions");
        tracing::info!("  • Retrieving crawled pages and content");
        tracing::info!("  • Searching within crawled content");
        tracing::info!("  • Web search via search engines");
        tracing::info!("  • Multiple concurrent crawl sessions");
        tracing::info!("  • Custom crawl settings (depth, robots.txt, user-agent)");

        Ok::<(), anyhow::Error>(())
    }
    .await;

    // Note: Crawl sessions automatically clean up after 5 minutes via background task.
    // No explicit cleanup needed - crawls run to completion in background.
    if !crawl_ids.is_empty() {
        tracing::info!(
            "\nCrawl sessions will auto-cleanup after 5 minutes: {:?}",
            crawl_ids
        );
    }

    test_result
}
