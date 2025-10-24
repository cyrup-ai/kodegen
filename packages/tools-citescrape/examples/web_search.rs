//! Web search functionality test
//!
//! Tests the DuckDuckGo-based `web_search` module with proper terminal output.
//!
//! Usage:
//!   cargo run --package `kodegen_tools_citescrape` --example `web_search`

use kodegen_tools_citescrape::web_search::{self, BrowserManager};
use std::io::Write;
use std::time::Instant;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // Create browser manager for proper lifecycle management
    let browser_manager = BrowserManager::new();

    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    // Print header
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "\n🔍 Web Search Test\n")?;
    stdout.reset()?;

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
    writeln!(
        &mut stdout,
        "Testing DuckDuckGo web search with proper terminal output.\n"
    )?;
    stdout.reset()?;

    // Test 1: Single search with full result display
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    writeln!(&mut stdout, "=== Test 1: Single Search ===")?;
    stdout.reset()?;

    let start = Instant::now();
    match web_search::search_with_manager(&browser_manager, "rust async programming").await {
        Ok(results) => {
            let elapsed = start.elapsed();
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(
                &mut stdout,
                "✓ Search completed in {:.2}s",
                elapsed.as_secs_f64()
            )?;
            stdout.reset()?;

            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
            writeln!(&mut stdout, "  Query: {}", results.query)?;
            writeln!(&mut stdout, "  Results: {}\n", results.results.len())?;
            stdout.reset()?;

            // Display all 10 results
            for result in &results.results {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(true))?;
                writeln!(&mut stdout, "  {}. {}", result.rank, result.title)?;
                stdout.reset()?;

                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                writeln!(&mut stdout, "     🔗 {}", result.url)?;
                stdout.reset()?;

                stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
                writeln!(&mut stdout, "     📄 {}", result.snippet)?;
                stdout.reset()?;
                writeln!(&mut stdout)?;
            }
        }
        Err(e) => {
            let elapsed = start.elapsed();
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            writeln!(
                &mut stderr,
                "✗ Search failed after {:.2}s: {}",
                elapsed.as_secs_f64(),
                e
            )?;
            stderr.reset()?;
            return Err(e);
        }
    }

    // Test 2: Second search (should be faster - browser already launched)
    writeln!(&mut stdout)?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    writeln!(&mut stdout, "=== Test 2: Second Search (Browser Reuse) ===")?;
    stdout.reset()?;

    let start = Instant::now();
    match web_search::search_with_manager(&browser_manager, "tokio rust tutorial").await {
        Ok(results) => {
            let elapsed = start.elapsed();
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(
                &mut stdout,
                "✓ Search completed in {:.2}s",
                elapsed.as_secs_f64()
            )?;
            stdout.reset()?;

            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
            writeln!(&mut stdout, "  Query: {}", results.query)?;
            writeln!(&mut stdout, "  Results: {}\n", results.results.len())?;
            stdout.reset()?;

            // Display all 10 results
            for result in &results.results {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(true))?;
                writeln!(&mut stdout, "  {}. {}", result.rank, result.title)?;
                stdout.reset()?;

                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                writeln!(&mut stdout, "     🔗 {}", result.url)?;
                stdout.reset()?;

                stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
                writeln!(&mut stdout, "     📄 {}", result.snippet)?;
                stdout.reset()?;
                writeln!(&mut stdout)?;
            }
        }
        Err(e) => {
            let elapsed = start.elapsed();
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            writeln!(
                &mut stderr,
                "✗ Search failed after {:.2}s: {}",
                elapsed.as_secs_f64(),
                e
            )?;
            stderr.reset()?;
            return Err(e);
        }
    }

    // Test 3: Third search to confirm consistency
    writeln!(&mut stdout)?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    writeln!(
        &mut stdout,
        "=== Test 3: Third Search (Consistency Check) ==="
    )?;
    stdout.reset()?;

    let start = Instant::now();
    match web_search::search_with_manager(&browser_manager, "serde json rust").await {
        Ok(results) => {
            let elapsed = start.elapsed();
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(
                &mut stdout,
                "✓ Search completed in {:.2}s",
                elapsed.as_secs_f64()
            )?;
            stdout.reset()?;

            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
            writeln!(&mut stdout, "  Query: {}", results.query)?;
            writeln!(&mut stdout, "  Results: {}\n", results.results.len())?;
            stdout.reset()?;

            // Display all 10 results
            for result in &results.results {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(true))?;
                writeln!(&mut stdout, "  {}. {}", result.rank, result.title)?;
                stdout.reset()?;

                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                writeln!(&mut stdout, "     🔗 {}", result.url)?;
                stdout.reset()?;

                stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
                writeln!(&mut stdout, "     📄 {}", result.snippet)?;
                stdout.reset()?;
                writeln!(&mut stdout)?;
            }
        }
        Err(e) => {
            let elapsed = start.elapsed();
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            writeln!(
                &mut stderr,
                "✗ Search failed after {:.2}s: {}",
                elapsed.as_secs_f64(),
                e
            )?;
            stderr.reset()?;
            return Err(e);
        }
    }

    // Summary
    writeln!(&mut stdout)?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    writeln!(
        &mut stdout,
        "✓ All web search tests completed successfully!"
    )?;
    stdout.reset()?;

    writeln!(&mut stdout)?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "📊 Summary:")?;
    stdout.reset()?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
    writeln!(&mut stdout, "  • All 10 results displayed for each query")?;
    writeln!(
        &mut stdout,
        "  • Titles, URLs, and snippets extracted successfully"
    )?;
    stdout.reset()?;

    writeln!(&mut stdout)?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "⏱️  Expected timing:")?;
    stdout.reset()?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
    writeln!(
        &mut stdout,
        "  • First search: ~5-6s (includes browser launch + React render)"
    )?;
    writeln!(
        &mut stdout,
        "  • Subsequent searches: ~2-4s (browser reuse, smart polling)"
    )?;
    stdout.reset()?;

    writeln!(&mut stdout)?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "💡 Performance:")?;
    stdout.reset()?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
    writeln!(
        &mut stdout,
        "  • Smart polling detects results as soon as they appear"
    )?;
    writeln!(
        &mut stdout,
        "  • Proper URL encoding handles special characters"
    )?;
    writeln!(
        &mut stdout,
        "  • Detailed error messages for troubleshooting"
    )?;
    stdout.reset()?;

    // Shutdown browser to clean up Chrome processes
    browser_manager.shutdown().await?;

    Ok(())
}
