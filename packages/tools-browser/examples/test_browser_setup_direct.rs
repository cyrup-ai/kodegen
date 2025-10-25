//! Direct browser_setup testing (no MCP, no managers)
//!
//! Tests raw browser_setup functions to see exact chromiumoxide errors.
//!
//! Usage:
//!   cargo run --package kodegen_tools_browser --example test_browser_setup_direct

use kodegen_tools_browser::browser_setup;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging with DEBUG level to see chromiumoxide internals
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .init();

    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    // Print header
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "\n🔧 Direct browser_setup Test\n")?;
    stdout.reset()?;

    // Test 1: find_browser_executable
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    writeln!(&mut stdout, "=== Test 1: find_browser_executable() ===")?;
    stdout.reset()?;

    match browser_setup::find_browser_executable().await {
        Ok(path) => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(&mut stdout, "✓ Found Chrome at: {}", path.display())?;
            stdout.reset()?;
        }
        Err(e) => {
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            writeln!(&mut stderr, "✗ Failed to find Chrome: {:#}", e)?;
            stderr.reset()?;
            return Err(e);
        }
    }

    writeln!(&mut stdout)?;

    // Test 2: launch_browser
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    writeln!(&mut stdout, "=== Test 2: launch_browser(headless=true) ===")?;
    stdout.reset()?;

    match browser_setup::launch_browser(true, None).await {
        Ok((browser, handler)) => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(&mut stdout, "✓ Browser launched successfully")?;
            stdout.reset()?;

            // Test 3: Create a page and navigate
            writeln!(&mut stdout)?;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
            writeln!(&mut stdout, "=== Test 3: Create page and navigate ===")?;
            stdout.reset()?;

            match browser.new_page("https://example.com").await {
                Ok(page) => {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                    writeln!(&mut stdout, "✓ Page created and navigated to example.com")?;
                    stdout.reset()?;

                    // Wait a moment for page to load
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    // Try to get page title
                    if let Ok(title_result) = page.evaluate("document.title").await {
                        if let Ok(Some(title)) = title_result.into_value::<String>() {
                            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
                            writeln!(&mut stdout, "  Page title: {}", title)?;
                            stdout.reset()?;
                        }
                    }
                }
                Err(e) => {
                    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
                    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                    writeln!(&mut stderr, "✗ Failed to create page: {:#}", e)?;
                    stderr.reset()?;
                }
            }

            // Clean shutdown
            writeln!(&mut stdout)?;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
            writeln!(&mut stdout, "=== Test 4: Clean shutdown ===")?;
            stdout.reset()?;

            if let Err(e) = browser.close().await {
                let mut stderr = StandardStream::stderr(ColorChoice::Auto);
                stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(&mut stderr, "⚠ Browser close warning: {}", e)?;
                stderr.reset()?;
            }

            if let Err(e) = browser.wait().await {
                let mut stderr = StandardStream::stderr(ColorChoice::Auto);
                stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(&mut stderr, "⚠ Browser wait warning: {}", e)?;
                stderr.reset()?;
            }

            handler.abort();

            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(&mut stdout, "✓ Browser shut down cleanly")?;
            stdout.reset()?;
        }
        Err(e) => {
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            writeln!(&mut stderr, "\n✗ FAILED TO LAUNCH BROWSER")?;
            writeln!(&mut stderr, "\nFull error chain:")?;
            writeln!(&mut stderr, "{:#}", e)?;
            stderr.reset()?;
            return Err(e);
        }
    }

    // Success summary
    writeln!(&mut stdout)?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    writeln!(&mut stdout, "✓ All browser_setup tests passed!")?;
    stdout.reset()?;

    Ok(())
}
