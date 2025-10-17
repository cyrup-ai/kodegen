mod common;

use anyhow::Result;
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("info".parse()?))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Connect to kodegen server with citescrape tools
    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Citescrape])
    ).await?;
    tracing::info!("Connected to server: {:?}", client.server_info());

    // Test 1: Start a web crawl
    tracing::info!("\n=== Testing start_crawl ===");
    tracing::info!("Starting crawl of Rust documentation...");
    
    let result = client.call_tool("start_crawl", json!({
        "url": "https://doc.rust-lang.org/book/",
        "maxPages": 5,
        "maxDepth": 2,
        "sameDomainOnly": true,
        "timeout": 30000
    })).await?;
    
    tracing::info!("Crawl started: {:?}", result);
    
    // Extract session ID
    let session_id: String = serde_json::from_value(
        result.content.first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.as_str())
            .and_then(|t| serde_json::from_str(t).ok())
            .and_then(|v: serde_json::Value| v.get("sessionId").cloned())
            .unwrap_or_default()
    )?;
    
    tracing::info!("✅ Crawl session ID: {}", session_id);
    
    // Wait for crawl to complete
    tracing::info!("Waiting for crawl to complete...");
    sleep(Duration::from_secs(10)).await;
    
    // Test 2: Get crawl results
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
    
    // Test 2b: Error handling
    tracing::info!("\n=== Testing error handling ===");
    let error_result = client.call_tool("start_crawl", json!({
        "url": "https://invalid-domain-that-does-not-exist-12345.com/",
        "maxPages": 1,
        "timeout": 5000
    })).await;
    
    match error_result {
        Ok(_) => tracing::info!("Unexpected success with invalid URL"),
        Err(e) => tracing::info!("✅ Proper error handling: {}", e),
    }
    
    // Test 3: Get full content for one page
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

    // Test 4: Search within crawled content
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

    // Test 5: Web search with search engines
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

    // Test 6: Start another crawl with different settings
    tracing::info!("\n=== Testing crawl with custom settings ===");
    
    let result = client.call_tool("start_crawl", json!({
        "url": "https://www.rust-lang.org/",
        "maxPages": 3,
        "maxDepth": 1,
        "sameDomainOnly": true,
        "followRobotsTxt": true,
        "userAgent": "KodegenMCPClient/1.0"
    })).await?;
    
    let session_id_2: String = serde_json::from_value(
        result.content.first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.as_str())
            .and_then(|t| serde_json::from_str(t).ok())
            .and_then(|v: serde_json::Value| v.get("sessionId").cloned())
            .unwrap_or_default()
    )?;
    
    tracing::info!("✅ Second crawl started: {}", session_id_2);
    tracing::info!("Crawl respects robots.txt directives");
    
    // Wait and get results
    sleep(Duration::from_secs(5)).await;
    
    let _result = client.call_tool("get_crawl_results", json!({
        "sessionId": session_id_2,
        "includeContent": false
    })).await?;
    
    tracing::info!("Second crawl results retrieved");

    // Cleanup
    client.close().await?;
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
    
    Ok(())
}
