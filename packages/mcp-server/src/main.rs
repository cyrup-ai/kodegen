use anyhow::Result;
use clap::Parser;
use kodegen_utils::usage_tracker::UsageTracker;

mod cli;
mod common;
mod sse;
mod stdio;

use cli::{Cli, Commands, ServerMode};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Install default CryptoProvider for rustls (required for GitHub API client)
    // This must be called before any HTTPS connections are made
    let _ = rustls::crypto::ring::default_provider().install_default();

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
    
    // Handle install command
    if let Some(Commands::Install) = cli.command {
        return cli::install::run_install();
    }
    
    // Get enabled categories
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
            eprintln!("Available categories (based on compiled features):");
            for cat in available {
                eprintln!("  - {cat}");
            }
            eprintln!();
            eprintln!("Tip: Use --list-categories to see all available categories");
            std::process::exit(1);
        }
    }

    // Generate unique instance ID for this server run
    let instance_id = chrono::Utc::now()
        .format("%Y%m%d-%H%M%S")
        .to_string();

    // Initialize shared components
    let config_manager = kodegen_tools_config::ConfigManager::new();
    config_manager.init().await?;
    let usage_tracker = UsageTracker::new(instance_id.clone());

    // Initialize tool call history tracking
    kodegen_mcp_tool::tool_history::init_global_history(instance_id).await;

    // Launch appropriate server based on mode
    match cli.server_mode() {
        ServerMode::Stdio { proxy_url } => {
            log::info!("Starting stdio server (proxy: {proxy_url})");
            
            // Ensure daemon is running for stdio mode
            if let Err(e) = cli::daemon::ensure_daemon_running().await {
                eprintln!("\n❌ Failed to start stdio server\n");
                eprintln!("Error: {e}\n");
                eprintln!("Stdio mode requires the kodegend daemon to be running.");
                eprintln!("\nTroubleshooting steps:");
                eprintln!("  1. Check if kodegend is installed:");
                eprintln!("     $ which kodegend");
                eprintln!("\n  2. If not installed:");
                eprintln!("     $ cargo install kodegend");
                eprintln!("\n  3. Check daemon status:");
                eprintln!("     $ kodegend status");
                eprintln!("\n  4. View daemon logs:");
                eprintln!("     $ kodegend logs");
                eprintln!("\n  5. Try running daemon in foreground to see errors:");
                eprintln!("     $ kodegend run --foreground");
                eprintln!("\nAlternative: Use SSE mode (no daemon required):");
                eprintln!("  $ kodegen --sse 127.0.0.1:8080");
                eprintln!();
                
                std::process::exit(1);
            }
            
            // Create cancellation token for graceful shutdown during initialization
            let shutdown_token = tokio_util::sync::CancellationToken::new();
            
            // Spawn signal handler for SIGINT and SIGTERM
            let signal_token = shutdown_token.clone();
            tokio::spawn(async move {
                let ctrl_c = tokio::signal::ctrl_c();
                
                #[cfg(unix)]
                {
                    use tokio::signal::unix::{signal, SignalKind};
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
            
            let sse_config = stdio::SseConnectionConfig {
                connection_timeout: cli.sse_connection_timeout(&config_manager),
                max_retries: cli.sse_max_retries(),
                retry_backoff: cli.sse_retry_backoff_duration(),
            };
            let server = stdio::StdioProxyServer::new(
                &proxy_url,
                config_manager,
                usage_tracker,
                &enabled_categories,
                sse_config,
                shutdown_token,
            ).await?;
            
            server.serve_stdio().await?;
        }
        ServerMode::Sse(addr) => {
            let protocol = if cli.tls_config().is_some() { "https" } else { "http" };
            log::info!("Starting SSE server on {protocol}://{addr}");
            
            // Prepare database configuration
            #[cfg(feature = "database")]
            let (database_dsn, ssh_config) = if let Some(ref dsn) = cli.database_dsn {
                // Store config values
                config_manager.set_value("readonly", kodegen_tools_config::ConfigValue::Boolean(cli.database_readonly)).await?;
                if let Some(max_rows) = cli.database_max_rows {
                    config_manager.set_value("max_rows", kodegen_tools_config::ConfigValue::Number(max_rows as i64)).await?;
                }
                
                // Build SSH config if requested
                let ssh_cfg = if cli.ssh_host.is_some() {
                    use kodegen_tools_database::{SSHConfig, TunnelConfig, SSHAuth};
                    
                    let ssh_host = cli.ssh_host.clone()
                        .ok_or_else(|| anyhow::anyhow!("SSH host required"))?;
                    let ssh_user = cli.ssh_user.clone()
                        .ok_or_else(|| anyhow::anyhow!("SSH user required"))?;
                    
                    // Determine auth method
                    let auth = if let Some(key_path) = &cli.ssh_key {
                        SSHAuth::Key {
                            path: key_path.clone(),
                            passphrase: None,
                        }
                    } else if let Some(password) = &cli.ssh_password {
                        SSHAuth::Password(password.clone())
                    } else {
                        return Err(anyhow::anyhow!(
                            "SSH authentication required: provide --ssh-key or --ssh-password"
                        ));
                    };
                    
                    let ssh_config = SSHConfig {
                        host: ssh_host,
                        port: cli.ssh_port,
                        username: ssh_user,
                        auth,
                    };
                    
                    // Extract target from DSN
                    let target_host = kodegen_tools_database::extract_host(dsn)?;
                    let target_port = kodegen_tools_database::extract_port(dsn)?;
                    
                    let tunnel_config = TunnelConfig {
                        target_host,
                        target_port,
                    };
                    
                    Some((ssh_config, tunnel_config))
                } else {
                    None
                };
                
                (Some(dsn.as_str()), ssh_cfg)
            } else {
                (None, None)
            };
            
            #[cfg(not(feature = "database"))]
            let (database_dsn, ssh_config): (Option<&str>, Option<()>) = (None, None);
            
            let routers = common::build_routers::<sse::SseServer>(
                &config_manager,
                &usage_tracker,
                &enabled_categories,
                database_dsn,
                ssh_config,
            ).await?;
            
            let server = sse::SseServer::new(
                routers.tool_router,
                routers.prompt_router,
                usage_tracker,
                config_manager,
                routers.managers,
            );
            
            let server_handle = server.serve_with_tls(addr, cli.tls_config()).await?;
            
            log::info!("SSE server running on {protocol}://{addr}");
            if cli.tls_config().is_some() {
                log::info!("TLS/HTTPS enabled - using encrypted connections");
            }
            log::info!("Press Ctrl+C or send SIGTERM to initiate graceful shutdown");
            
            // Wait for shutdown signal (either SIGINT or SIGTERM)
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                
                let ctrl_c = tokio::signal::ctrl_c();
                let mut sigterm = signal(SignalKind::terminate())?;
                
                tokio::select! {
                    _ = ctrl_c => {
                        log::debug!("Received SIGINT (Ctrl+C)");
                    }
                    _ = sigterm.recv() => {
                        log::debug!("Received SIGTERM");
                    }
                }
            }
            
            #[cfg(not(unix))]
            {
                tokio::signal::ctrl_c().await?;
            }
            
            let timeout = cli.shutdown_timeout();
            log::info!(
                "Shutdown signal received, initiating graceful shutdown (maximum timeout: {timeout:?})"
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
                        "Graceful shutdown timeout ({timeout:?}) elapsed before completion. \
                         Forcing exit. Some in-flight requests may have been interrupted."
                    );
                }
            }
            
            log::info!("SSE server stopped");
        }
    }
    
    Ok(())
}
