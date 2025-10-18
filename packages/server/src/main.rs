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
            
            let timeout = cli.shutdown_timeout();
            log::info!("Shutting down SSE server gracefully (timeout: {:?})...", timeout);
            
            // Cancel the server and wait for graceful shutdown
            ct.cancel();
            
            // Give in-flight requests time to complete
            tokio::time::sleep(timeout).await;
        }
    }
    
    Ok(())
}
