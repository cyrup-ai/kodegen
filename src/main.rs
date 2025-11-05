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

    // Handle list-tools flag
    if cli.list_tools {
        println!("Available tools:");
        for tool in cli::available_tools() {
            println!("  - {tool}");
        }
        return Ok(());
    }

    // Get enabled tools from CLI (--tool/--tools/--toolset)
    let enabled_tools = cli.enabled_tools()?;

    // VALIDATE IMMEDIATELY - before any initialization
    if let Some(ref tools) = enabled_tools {
        let available = cli::available_tools();
        let invalid: Vec<_> = tools
            .iter()
            .filter(|tool| !available.contains(&tool.as_str()))
            .collect();

        if !invalid.is_empty() {
            eprintln!("Error: Invalid tool names specified:");
            for tool in &invalid {
                eprintln!("  - {tool}");
            }
            eprintln!();
            eprintln!("Available tools:");
            for tool in available {
                eprintln!("  - {tool}");
            }
            eprintln!();
            eprintln!("Tip: Use --list-tools to see all available tools");
            eprintln!("Tip: Use --toolset path/to/toolset.yaml to load from config file");
            std::process::exit(1);
        }
    }

    // Generate unique instance ID for this server run
    let timestamp = chrono::Utc::now();
    let pid = std::process::id();
    let instance_id = format!("{}-{}", timestamp.format("%Y%m%d-%H%M%S-%9f"), pid);

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

    // Configure HTTP client connections to category servers
    let http_config = stdio::HttpConnectionConfig {
        connection_timeout: cli.http_connection_timeout(&config_manager),
        max_retries: cli.http_max_retries(),
        retry_backoff: cli.http_retry_backoff_duration(),
        host: cli.host.clone(),
        no_tls: cli.no_tls,
    };

    // Create stdio proxy server (connects to category servers on ports 30437-30449)
    let server = stdio::StdioProxyServer::new(
        config_manager,
        usage_tracker,
        &enabled_tools,
        http_config,
        shutdown_token,
    )
    .await?;

    // Serve stdio transport (thin client mode)
    server.serve_stdio().await?;

    Ok(())
}
