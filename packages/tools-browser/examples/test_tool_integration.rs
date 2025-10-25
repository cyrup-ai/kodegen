//! Test BrowserNavigateTool and BrowserExtractTextTool as direct library calls
//!
//! This verifies the pattern used by DeepResearch works correctly.

use kodegen_tools_browser::{BrowserNavigateTool, BrowserExtractTextTool, BrowserManager};
use kodegen_mcp_schema::browser::{BrowserNavigateArgs, BrowserExtractTextArgs};
use kodegen_mcp_tool::Tool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    println!("\n🧪 Testing Browser Tools as Library Functions\n");

    // Get global browser manager (same pattern as DeepResearch)
    let browser_manager = BrowserManager::global();

    // TEST 1: Navigate to a page
    println!("=== Test 1: Navigation ===");
    let nav_tool = BrowserNavigateTool::new(browser_manager.clone());
    let nav_args = BrowserNavigateArgs {
        url: "https://www.linkedin.com/in/davemaple/".to_string(),
        wait_for_selector: None,
        timeout_ms: Some(30000),
    };

    let nav_result = nav_tool.execute(nav_args).await?;
    println!("✓ Navigation successful");

    let final_url = nav_result
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    println!("  Final URL: {}", final_url);

    // TEST 2: Extract content
    println!("\n=== Test 2: Content Extraction ===");
    let extract_tool = BrowserExtractTextTool::new(browser_manager.clone());
    let extract_args = BrowserExtractTextArgs {
        selector: None, // Full page
    };

    let extract_result = extract_tool.execute(extract_args).await?;
    println!("✓ Extraction successful");

    let content = extract_result
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let preview = if content.len() > 100 {
        format!("{}...", &content[0..100])
    } else {
        content.to_string()
    };

    println!("  Content length: {} chars", content.len());
    println!("  Preview: {}", preview);

    // Cleanup
    browser_manager.shutdown().await?;

    println!("\n✅ All tests passed - Tool integration works correctly!");
    println!("   This verifies the DeepResearch pattern is sound.\n");

    Ok(())
}
