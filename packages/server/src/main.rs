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
        return cli::install::run_install();
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
            cli::daemon::ensure_daemon_running().await?;
            
            let timeout = cli.sse_connection_timeout(&config_manager);
            let server = stdio::StdioProxyServer::new(
                proxy_url.as_deref(),
                config_manager,
                usage_tracker,
                &enabled_categories,
                timeout,
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
            
            let server_handle = server.serve(addr).await?;
            
            log::info!("SSE server running on http://{}", addr);
            log::info!("Press Ctrl+C to initiate graceful shutdown");
            
            tokio::signal::ctrl_c().await?;
            
            let timeout = cli.shutdown_timeout();
            log::info!(
                "Shutdown signal received, initiating graceful shutdown (maximum timeout: {:?})",
                timeout
            );
            
            // Signal server to begin shutdown
            server_handle.cancel();
            
            // Wait for server to complete shutdown, with timeout as safety maximum
            match server_handle.wait_for_completion(timeout).await {
                Ok(()) => {
                    log::info!("Server shutdown completed successfully");
                }
                Err(_elapsed) => {
                    log::warn!(
                        "Graceful shutdown timeout ({:?}) elapsed before completion. \
                         Forcing exit. Some in-flight requests may have been interrupted.",
                        timeout
                    );
                }
            }
            
            log::info!("SSE server stopped");
        }
    }
    
    Ok(())
}
