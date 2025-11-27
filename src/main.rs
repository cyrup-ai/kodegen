use anyhow::Result;
use clap::Parser;
use kodegen_utils::usage_tracker::UsageTracker;

mod cli;
mod commands;
mod stdio;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Handle subcommands FIRST (before stdio server logic)
    if let Some(command) = cli.command {
        return match command {
            Commands::Install => {
                println!("Install not yet implemented");
                Ok(())
            }
            Commands::Monitor { interval } => {
                commands::handle_monitor(interval).await
            }
        };
    }

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
            return Err(anyhow::anyhow!("Invalid tool names specified"));
        }
    }

    // Generate unique instance ID for this server run
    let timestamp = chrono::Utc::now();
    let pid = std::process::id();
    let instance_id = format!("{}-{}", timestamp.format("%Y%m%d-%H%M%S-%9f"), pid);

    // Initialize shared components
    let config_manager = kodegen_config_manager::ConfigManager::new();
    config_manager.init().await?;
    let usage_tracker = UsageTracker::new(instance_id.clone());

    // Initialize tool call history tracking
    kodegen_mcp_tool::tool_history::init_global_history(instance_id).await;

    log::info!("Starting stdio server (thin client with static metadata)");

    // Create cancellation token for graceful shutdown during initialization
    let shutdown_token = tokio_util::sync::CancellationToken::new();

    // Spawn cross-platform signal handler
    let signal_token = shutdown_token.clone();
    tokio::spawn(async move {
        wait_for_interrupt().await;
        log::debug!("Received interrupt signal, cancelling initialization");
        signal_token.cancel();
    });

    // Configure HTTP client connections to category servers
    let http_config = stdio::HttpConnectionConfig {
        connection_timeout: cli.http_connection_timeout(&config_manager),
        max_retries: cli.http_max_retries(),
        retry_backoff: cli.http_retry_backoff_duration(),
        host: cli.effective_host().to_string(),
        no_tls: cli.no_tls,
    };

    // Create stdio proxy server (connects to category servers on ports 30437-30449)
    let server = match stdio::StdioProxyServer::new(
        config_manager,
        usage_tracker,
        &enabled_tools,
        http_config,
        shutdown_token,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            log::error!(
                "STDIO_HEALTH: Stdio server failed to start due to category connection failures: {}",
                e
            );
            log::error!(
                "STDIO_HEALTH: Check that category HTTP servers are running (use 'just mcp' to start all servers)"
            );
            return Err(e);
        }
    };

    // Serve stdio transport (thin client mode)
    server.serve_stdio().await?;

    Ok(())
}

/// Wait for interrupt signal (cross-platform)
#[cfg(unix)]
async fn wait_for_interrupt() {
    use tokio::signal::unix::{signal, SignalKind};
    
    let mut sigterm_result = signal(SignalKind::terminate());
    let mut sigint_result = signal(SignalKind::interrupt());
    
    match (sigterm_result.as_mut(), sigint_result.as_mut()) {
        (Ok(sigterm), Ok(sigint)) => {
            tokio::select! {
                _ = sigterm.recv() => {}
                _ = sigint.recv() => {}
            }
        }
        (Ok(sigterm), Err(_)) => {
            let _ = sigterm.recv().await;
        }
        (Err(_), Ok(sigint)) => {
            let _ = sigint.recv().await;
        }
        (Err(_), Err(_)) => {
            // If both fail, just wait forever (shouldn't happen)
            let () = std::future::pending().await;
        }
    }
}

/// Wait for interrupt signal (cross-platform)
#[cfg(windows)]
async fn wait_for_interrupt() {
    use tokio::signal::windows;
    
    let ctrl_c_result = windows::ctrl_c();
    
    match ctrl_c_result {
        Ok(mut ctrl_c) => {
            let _ = ctrl_c.recv().await;
        }
        Err(_) => {
            // If ctrl_c fails, wait forever (shouldn't happen)
            let () = std::future::pending().await;
        }
    }
}
