//! Test what happens when `BrowserManager` drops without `shutdown()`

use kodegen_tools_citescrape::web_search::{self, BrowserManager};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to see warnings
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    println!("\n=== Testing BrowserManager drop behavior WITHOUT shutdown() ===\n");

    {
        let manager = BrowserManager::new();
        println!("1. Performing search...");
        let _results = web_search::search_with_manager(&manager, "rust").await?;
        println!("2. Search completed, browser is running");

        // Let manager drop WITHOUT calling shutdown()
        println!("3. Dropping manager WITHOUT calling shutdown()...");
    }

    println!("4. Manager dropped - checking for zombie processes and warnings...\n");

    // Give it a moment for async cleanup
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    println!("5. Done - run 'ps aux | grep enigo_chrome' to check for zombies");

    Ok(())
}
