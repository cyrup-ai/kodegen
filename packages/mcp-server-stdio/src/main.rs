use anyhow::Result;
use clap::Parser;
use kodegen_utils::usage_tracker::UsageTracker;

mod cli;
mod stdio;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Handle list-categories flag
    if cli.list_categories {
        println!("Available tool categories:");
        for category in cli::available_categories() {
            println!("  - {category}");
        }
        return Ok(());
    }

    // Get enabled categories from CLI (--tool/--tools)
    let enabled_categories = cli.enabled_categories();

    // VALIDATE IMMEDIATELY - before any initialization
    if let Some(ref categories) = enabled_categories {
        let available = cli::available_categories();
        let invalid: Vec<_> = categories
            .iter()
            .filter(|cat| !available.contains(&cat.as_str()))
            .collect();

        if !invalid.is_empty() {
            eprintln!("Error: Invalid tool categories specified:");
            for cat in &invalid {
                eprintln!("  - {cat}");
            }
            eprintln!();
            eprintln!("Available categories:");
            for cat in available {
                eprintln!("  - {cat}");
            }
            eprintln!();
            eprintln!("Tip: Use --list-categories to see all available categories");
            std::process::exit(1);
        }
    }

    // Generate unique instance ID for this server run
    let instance_id = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();

    // Initialize shared components
    let config_manager = kodegen_tools_config::ConfigManager::new();
    config_manager.init().await?;
    let usage_tracker = UsageTracker::new(instance_id.clone());

    // Initialize tool call history tracking
    kodegen_mcp_tool::tool_history::init_global_history(instance_id).await;

    log::info!("Starting stdio server (thin client with static metadata)");

    // Create cancellation token for graceful shutdown during initialization
    let shutdown_token = tokio_util::sync::CancellationToken::new();

    // Spawn signal handler for SIGINT and SIGTERM
    let signal_token = shutdown_token.clone();
    tokio::spawn(async move {
        let ctrl_c = tokio::signal::ctrl_c();

        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm = match signal(SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Failed to register SIGTERM handler: {e}");
                    // Fall back to just SIGINT
                    if ctrl_c.await.is_ok() {
                        signal_token.cancel();
                    }
                    return;
                }
            };

            tokio::select! {
                _ = ctrl_c => {
                    log::debug!("Received SIGINT, cancelling initialization");
                    signal_token.cancel();
                }
                _ = sigterm.recv() => {
                    log::debug!("Received SIGTERM, cancelling initialization");
                    signal_token.cancel();
                }
            }
        }

        #[cfg(not(unix))]
        {
            if ctrl_c.await.is_ok() {
                log::debug!("Received Ctrl+C, cancelling initialization");
                signal_token.cancel();
            }
        }
    });

    // Configure SSE client connections to category servers
    let sse_config = stdio::SseConnectionConfig {
        connection_timeout: cli.sse_connection_timeout(&config_manager),
        max_retries: cli.sse_max_retries(),
        retry_backoff: cli.sse_retry_backoff_duration(),
    };

    // Create stdio proxy server (connects to category servers on ports 30437-30449)
    let server = stdio::StdioProxyServer::new(
        config_manager,
        usage_tracker,
        &enabled_categories,
        sse_config,
        shutdown_token,
    )
    .await?;

    // Serve stdio transport (thin client mode)
    server.serve_stdio().await?;

    Ok(())
}
