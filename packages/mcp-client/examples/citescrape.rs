mod common;

use anyhow::Result;
use kodegen_mcp_client::responses::StartCrawlResponse;
use kodegen_mcp_client::KodegenClient;
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::time::{sleep, Duration};

async fn wait_for_crawl_completion(
    client: &KodegenClient,
    session_id: &str,
    timeout: Duration,
) -> anyhow::Result<()> {
    let start = tokio::time::Instant::now();
    let mut backoff = Duration::from_millis(100);

    loop {
        let result = client.call_tool("get_crawl_results", json!({
            "sessionId": session_id,
            "includeContent": false
        })).await?;

        if let Some(text) = result.content.first().and_then(|c| c.as_text())
            && let Ok(crawl_data) = serde_json::from_str::<serde_json::Value>(&text.text)
            && let Some(status) = crawl_data.get("status").and_then(|s| s.as_str()) {
                match status {
                    "completed" => return Ok(()),
                    "failed" | "error" => {
                        anyhow::bail!("Crawl failed with status: {}", status)
                    }
                    _ => {}
                }
            }

        if start.elapsed() > timeout {
            anyhow::bail!("Crawl timed out after {:?}", timeout);
        }

        sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_millis(500));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("info".parse()?))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Citescrape])
    ).await?;
    tracing::info!("Connected to server: {:?}", client.server_info());

    let result = run_citescrape_example(&client).await;

    client.close().await?;

    result
}

async fn run_citescrape_example(client: &KodegenClient) -> Result<()> {
    let mut session_ids = Vec::new();
    
    let test_result = async {
        tracing::info!("\n=== Testing start_crawl ===");
        tracing::info!("Starting crawl of Rust documentation...");
        
        let response: StartCrawlResponse = client.call_tool_typed("start_crawl", json!({
            "url": "https://doc.rust-lang.org/book/",
            "maxPages": 5,
            "maxDepth": 2,
            "sameDomainOnly": true,
            "timeout": 30000
        })).await?;
        
        let session_id = response.session_id;
        session_ids.push(session_id.clone());
        tracing::info!("✅ Crawl session ID: {}", session_id);
        
        tracing::info!("Waiting for crawl to complete...");
        wait_for_crawl_completion(&client, &session_id, Duration::from_secs(60)).await?;
        
        tracing::info!("\n=== Testing get_crawl_results ===");
        
        let result = client.call_tool("get_crawl_results", json!({
            "sessionId": session_id,
            "includeContent": false
        })).await?;
        
        if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
            let crawl_data: serde_json::Value = serde_json::from_str(&text.text)?;
            tracing::info!("Crawl results summary:");
            
            if let Some(pages) = crawl_data.get("pages").and_then(|p| p.as_array()) {
                tracing::info!("  Total pages crawled: {}", pages.len());
                
                for (i, page) in pages.iter().enumerate().take(3) {
                    if let Some(url) = page.get("url").and_then(|u| u.as_str()) {
                        let title = page.get("title")
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
        
        tracing::info!("\n=== Testing error handling ===");
        let error_result = client.call_tool("start_crawl", json!({
            "url": "https://invalid-domain-that-does-not-exist-12345.com/",
            "maxPages": 1,
            "timeout": 5000
        })).await;
        
        match error_result {
            Ok(_) => anyhow::bail!("Expected error for invalid URL, but got success"),
            Err(e) => {
                tracing::info!("✅ Correctly handled invalid URL: {}", e);
                assert!(
                    e.to_string().contains("invalid") ||
                    e.to_string().contains("not found") ||
                    e.to_string().contains("failed")
                );
            }
        }
        
        tracing::info!("\n=== Getting full page content ===");
        
        let result = client.call_tool("get_crawl_results", json!({
            "sessionId": session_id,
            "includeContent": true
        })).await?;
        
        if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
            let crawl_data: serde_json::Value = serde_json::from_str(&text.text)?;
            
            if let Some(content) = crawl_data.get("pages")
                .and_then(|p| p.as_array())
                .and_then(|pages| pages.first())
                .and_then(|first_page| first_page.get("content"))
                .and_then(|c| c.as_str()) {
                let preview = &content.chars().take(200).collect::<String>();
                tracing::info!("First page content preview:");
                tracing::info!("  {}", preview);
            }
        }

        tracing::info!("\n=== Testing search_crawl_results ===");
        
        let result = client.call_tool("search_crawl_results", json!({
            "sessionId": session_id,
            "query": "ownership",
            "maxResults": 5
        })).await?;
        
        if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
            let search_results: serde_json::Value = serde_json::from_str(&text.text)?;
            tracing::info!("Search results for 'ownership':");
            
            if let Some(results) = search_results.get("results").and_then(|r| r.as_array()) {
                tracing::info!("  Found {} matches", results.len());
                
                for (i, result) in results.iter().enumerate() {
                    if let Some(url) = result.get("url").and_then(|u| u.as_str()) {
                        let score = result.get("score")
                            .and_then(|s| s.as_f64())
                            .unwrap_or(0.0);
                        tracing::info!("  {}. {} (score: {:.2})", i + 1, url, score);
                        
                        if let Some(snippet) = result.get("snippet").and_then(|s| s.as_str()) {
                            tracing::info!("     \"{}\"", snippet);
                        }
                    }
                }
            }
        }

        tracing::info!("\n=== Testing web_search ===");
        
        let result = client.call_tool("web_search", json!({
            "query": "rust async programming tutorial",
            "engine": "duckduckgo",
            "maxResults": 5
        })).await?;
        
        if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
            let search_results: serde_json::Value = serde_json::from_str(&text.text)?;
            tracing::info!("Web search results:");
            
            if let Some(results) = search_results.get("results").and_then(|r| r.as_array()) {
                for (i, result) in results.iter().enumerate() {
                    if let Some(url) = result.get("url").and_then(|u| u.as_str()) {
                        let title = result.get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("No title");
                        tracing::info!("  {}. {}", i + 1, title);
                        tracing::info!("     {}", url);
                    }
                }
            }
        }

        tracing::info!("\n=== Testing crawl with custom settings ===");
        
        let response_2: StartCrawlResponse = client.call_tool_typed("start_crawl", json!({
            "url": "https://www.rust-lang.org/",
            "maxPages": 3,
            "maxDepth": 1,
            "sameDomainOnly": true,
            "followRobotsTxt": true,
            "userAgent": "KodegenMCPClient/1.0"
        })).await?;
        
        let session_id_2 = response_2.session_id;
        session_ids.push(session_id_2.clone());
        tracing::info!("✅ Second crawl started: {}", session_id_2);
        tracing::info!("Crawl respects robots.txt directives");
        
        wait_for_crawl_completion(&client, &session_id_2, Duration::from_secs(60)).await?;
        
        let _result = client.call_tool("get_crawl_results", json!({
            "sessionId": session_id_2,
            "includeContent": false
        })).await?;
        
        tracing::info!("Second crawl results retrieved");

        tracing::info!("\n✅ All citescrape tools tested successfully!");
        
        tracing::info!("\n📚 Rate Limiting Features:");
        tracing::info!("  • Respects robots.txt by default");
        tracing::info!("  • Implements polite crawling with delays");
        tracing::info!("  • Configurable timeout and user agent");
        tracing::info!("  • Session-based for multiple concurrent crawls");
        
        tracing::info!("\n📚 Search Engines Supported:");
        tracing::info!("  • DuckDuckGo - Privacy-focused, no API key required");
        tracing::info!("  • Google - Requires API key and custom search engine ID");
        tracing::info!("  • Bing - Requires API key");
        
        tracing::info!("\n📚 Features Demonstrated:");
        tracing::info!("  • Starting web crawl sessions");
        tracing::info!("  • Retrieving crawled pages and content");
        tracing::info!("  • Searching within crawled content");
        tracing::info!("  • Web search via search engines");
        tracing::info!("  • Multiple concurrent crawl sessions");
        tracing::info!("  • Custom crawl settings (depth, robots.txt, user-agent)");
        
        Ok::<(), anyhow::Error>(())
    }.await;

    cleanup_citescrape_sessions(client, &session_ids).await;

    test_result
}

async fn cleanup_citescrape_sessions(client: &KodegenClient, session_ids: &[String]) {
    tracing::info!("\nCleaning up crawl sessions...");
    
    for sid in session_ids {
        if let Err(e) = client.call_tool("stop_search", json!({
            "session_id": sid
        })).await {
            tracing::error!("⚠️  Failed to stop crawl session {}: {}", sid, e);
        } else {
            tracing::info!("✅ Stopped crawl session: {}", sid);
        }
    }
}
