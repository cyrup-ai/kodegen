use anyhow::Result;
use clap::Parser;
use kodegen_utils::usage_tracker::UsageTracker;

mod cli;
mod common;
mod sse;
mod stdio;

use cli::{Cli, Commands, ServerMode};

fn main() -> Result<()> {
    // Pre-initialize citescrape LazyLock statics BEFORE starting tokio runtime
    // This prevents "Cannot block the current thread" panics
    #[cfg(feature = "citescrape")]
    {
        kodegen_citescrape::preinit_lazy_statics();
    }
    
    // Create tokio runtime manually so preinit happens first
    tokio::runtime::Runtime::new()?.block_on(async_main())
}

async fn async_main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Handle list-categories flag
    if cli.list_categories {
        println!("Available tool categories:");
        for category in cli::available_categories() {
            println!("  - {}", category);
        }
        return Ok(());
    }
    
    // Handle install command
    if let Some(Commands::Install) = cli.command {
        return run_install();
    }
    
    // Get enabled categories
    let enabled_categories = cli.enabled_categories();
    
    // Initialize shared components
    let config_manager = kodegen_config::ConfigManager::new();
    config_manager.init().await?;
    let usage_tracker = UsageTracker::new();
    
    // Launch appropriate server based on mode
    match cli.server_mode() {
        ServerMode::Stdio { proxy_url } => {
            log::info!("Starting stdio server (proxy: {:?})", proxy_url);
            
            // Ensure daemon is running for stdio mode
            ensure_daemon_running().await?;
            
            let server = stdio::StdioProxyServer::new(
                proxy_url.as_deref(),
                config_manager,
                usage_tracker,
                &enabled_categories,
            ).await?;
            
            server.serve_stdio().await?;
        }
        ServerMode::Sse(addr) => {
            log::info!("Starting SSE server on {}", addr);
            
            let routers = common::build_routers::<sse::SseServer>(
                &config_manager,
                &usage_tracker,
                &enabled_categories,
            ).await?;
            
            let server = sse::SseServer::new(
                routers.tool_router,
                routers.prompt_router,
                usage_tracker,
                config_manager,
            );
            
            let ct = server.serve(addr).await?;
            
            log::info!("SSE server running, press Ctrl+C to stop");
            tokio::signal::ctrl_c().await?;
            
            log::info!("Shutting down SSE server...");
            ct.cancel();
        }
    }
    
    Ok(())
}

/// Run the install command
fn run_install() -> Result<()> {
    use kodegen_client_autoconfig::install_all_clients;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
    use std::io::Write;

    let results = install_all_clients()?;

    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // Header
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "\n┌─────────────────────────────────────────────┐")?;
    writeln!(&mut stdout, "│   🔍 MCP Editor Configuration Results       │")?;
    writeln!(&mut stdout, "└─────────────────────────────────────────────┘\n")?;
    stdout.reset()?;

    let mut configured = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for result in &results {
        if result.success {
            // Success - green checkmark
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
            write!(&mut stdout, "  ✓ ")?;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(&mut stdout, "{}", result.client_name)?;
        } else {
            // Failed - red X or dim skip
            if result.message == "Not installed" {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
                write!(&mut stdout, "  ○ ")?;
                writeln!(&mut stdout, "{}", result.client_name)?;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                write!(&mut stdout, "  ✗ ")?;
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(&mut stdout, "{}", result.client_name)?;
            }
        }

        // Config path
        if let Some(ref path) = result.config_path {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
            writeln!(&mut stdout, "     {}", path.display())?;
        }

        // Status message
        if result.success {
            if result.message.contains("Already") {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                writeln!(&mut stdout, "     {}\n", result.message)?;
                skipped += 1;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                writeln!(&mut stdout, "     {}\n", result.message)?;
                configured += 1;
            }
        } else {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
            writeln!(&mut stdout, "     {}\n", result.message)?;
            failed += 1;
        }
        stdout.reset()?;
    }

    // Summary
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "─────────────────────────────────────────────")?;
    stdout.reset()?;

    write!(&mut stdout, "  ")?;
    if configured > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        write!(&mut stdout, "{} configured", configured)?;
        stdout.reset()?;
    }
    if skipped > 0 {
        if configured > 0 { write!(&mut stdout, " • ")?; }
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        write!(&mut stdout, "{} already configured", skipped)?;
        stdout.reset()?;
    }
    if failed > 0 {
        if configured > 0 || skipped > 0 { write!(&mut stdout, " • ")?; }
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
        write!(&mut stdout, "{} not installed", failed)?;
        stdout.reset()?;
    }
    writeln!(&mut stdout, "\n")?;

    Ok(())
}

/// Ensure the kodegend daemon is running before starting stdio mode
///
/// Checks daemon status and starts it if not running, then waits for ready
async fn ensure_daemon_running() -> Result<()> {
    use tokio::process::Command;
    use tokio::time::{sleep, Duration};

    // Check if daemon is already running
    let status = Command::new("kodegend")
        .arg("status")
        .status()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to check daemon status: {}", e))?;

    if status.success() {
        log::info!("kodegend daemon is already running");
        return Ok(());
    }

    // Daemon not running, attempt to start it
    log::info!("kodegend daemon not running, starting...");
    let start = Command::new("kodegend")
        .arg("start")
        .status()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start daemon: {}", e))?;

    if !start.success() {
        anyhow::bail!("Failed to start kodegend daemon");
    }

    // Wait for daemon to be ready (poll with backoff)
    for attempt in 1..=10 {
        sleep(Duration::from_millis(500)).await;

        let check = Command::new("kodegend")
            .arg("status")
            .status()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to check daemon status: {}", e))?;

        if check.success() {
            log::info!("kodegend daemon started successfully after {} attempts", attempt);
            return Ok(());
        }

        if attempt == 10 {
            anyhow::bail!("Daemon failed to start after 10 attempts");
        }
    }

    Ok(())
}
