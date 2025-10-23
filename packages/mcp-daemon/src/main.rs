mod cli;
mod config;
mod control;
mod daemon;
mod ipc;
mod lifecycle;
mod manager;
mod service;
mod signing;
mod state_machine;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
use log::{error, info};
use manager::ServiceManager;

fn main() {
    // Initialize logger with custom format for daemon
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "[{} {} {}:{}] {}",
                buf.timestamp_millis(),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .filter_level(log::LevelFilter::Info)
        .init();

    let rt = tokio::runtime::Runtime::new()
        .expect("FATAL: Failed to create Tokio runtime - daemon cannot start without async runtime");
    if let Err(e) = rt.block_on(real_main()) {
        error!("{e:#}");
        std::process::exit(1);
    }
}

async fn real_main() -> Result<()> {
    let args = cli::Args::parse();

    match args.sub.unwrap_or(cli::Cmd::Run {
        foreground: false,
        config: None,
        system: false,
    }) {
        cli::Cmd::Run {
            foreground,
            config,
            system,
        } => run_daemon(foreground, config, system).await,
        cli::Cmd::Sign {
            binary,
            identity,
            verify,
            show_config,
            self_sign,
        } => handle_sign_command(binary, identity, verify, show_config, self_sign).await,
        cli::Cmd::Status => handle_status(),
        cli::Cmd::Start => handle_start(),
        cli::Cmd::Stop => handle_stop(),
        cli::Cmd::Restart => handle_restart(),
    }
}

async fn run_daemon(
    force_foreground: bool,
    config_path: Option<String>,
    use_system: bool,
) -> Result<()> {
    let should_stay_foreground = force_foreground || daemon::need_foreground();

    if !should_stay_foreground {
        daemon::daemonise(Path::new("/var/run/kodegend.pid"))?;
    }

    // Determine config path based on CLI arguments
    let cfg_path = if let Some(path) = config_path {
        // User specified an explicit config path
        PathBuf::from(path)
    } else if use_system {
        // User wants system-wide config
        PathBuf::from("/etc/kodegend/kodegend.toml")
    } else {
        // Default to user config directory
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("kodegend");
        config_dir.join("kodegend.toml")
    };

    // Load or create default config
    let cfg_str = fs::read_to_string(&cfg_path).or_else(|_| {
        info!("Config not found at {}, using defaults", cfg_path.display());
        Ok::<String, anyhow::Error>(toml::to_string_pretty(&config::ServiceConfig::default())?)
    })?;
    let cfg: config::ServiceConfig = toml::from_str(&cfg_str)?;

    info!("Using config from: {}", cfg_path.display());

    manager::install_signal_handlers();
    let mut mgr = ServiceManager::new(&cfg)?;

    // Start SSE server if enabled
    mgr.start_sse_server(&cfg).await?;

    // Start kodegen SSE server if enabled
    mgr.start_kodegen_sse_server(&cfg).await?;

    daemon::systemd_ready(); // tell systemd we are ready
    info!("kodegen daemon started (pid {})", std::process::id());
    mgr.run().await?;
    info!("kodegen daemon exiting");
    Ok(())
}

async fn handle_sign_command(
    binary: Option<String>,
    identity: Option<String>,
    verify: bool,
    show_config: bool,
    self_sign: bool,
) -> Result<()> {
    // Check if signing is available on this platform
    if !signing::is_signing_available() {
        kodegen_daemon::cli_output::warning("Code signing is not available on this platform");
        return Ok(());
    }

    if show_config {
        let sample = signing::config::create_sample_config()?;
        kodegen_daemon::cli_output::info(&format!("Sample signing configuration:\n\n{sample}"));
        return Ok(());
    }

    // Handle self-signing
    if self_sign {
        kodegen_daemon::cli_output::info("Self-signing current binary...");
        match signing::sign_self() {
            Ok(()) => {
                kodegen_daemon::cli_output::success("Successfully self-signed");
                return Ok(());
            }
            Err(e) => {
                kodegen_daemon::cli_output::error(&format!("Failed to self-sign: {e}"));
                std::process::exit(1);
            }
        }
    }

    let binary_path = if let Some(path) = binary {
        PathBuf::from(path)
    } else {
        std::env::current_exe()?
    };

    if verify {
        // Verify signature
        match signing::verify_signature(&binary_path) {
            Ok(true) => {
                kodegen_daemon::cli_output::success(&format!("{} is properly signed", binary_path.display()));
                Ok(())
            }
            Ok(false) => {
                kodegen_daemon::cli_output::error(&format!(
                    "{} is not signed or signature is invalid",
                    binary_path.display()
                ));
                std::process::exit(1);
            }
            Err(e) => {
                kodegen_daemon::cli_output::error(&format!("Failed to verify signature: {e}"));
                std::process::exit(1);
            }
        }
    } else {
        // Sign the binary
        let mut config = signing::SigningConfig::load()?;
        config.binary_path = binary_path;
        config.output_path = config.binary_path.clone();

        // Override identity if provided
        if let Some(id) = identity {
            match &mut config.platform {
                #[cfg(target_os = "macos")]
                signing::PlatformConfig::MacOS { identity, .. } => *identity = id,
                #[cfg(target_os = "windows")]
                signing::PlatformConfig::Windows { certificate, .. } => *certificate = id,
                #[cfg(target_os = "linux")]
                signing::PlatformConfig::Linux { key_id, .. } => *key_id = Some(id),
                _ => {}
            }
        }

        kodegen_daemon::cli_output::info(&format!("Signing {}...", config.binary_path.display()));

        match signing::sign_binary(&config) {
            Ok(()) => {
                kodegen_daemon::cli_output::success(&format!("Successfully signed {}", config.binary_path.display()));

                // Verify the signature
                if signing::verify_signature(&config.output_path)? {
                    kodegen_daemon::cli_output::success("Signature verified");
                } else {
                    kodegen_daemon::cli_output::error("Signature verification failed");
                }

                Ok(())
            }
            Err(e) => {
                kodegen_daemon::cli_output::error(&format!("Failed to sign binary: {e}"));
                std::process::exit(1);
            }
        }
    }
}

/// Handle status command - check if daemon is running
fn handle_status() -> Result<()> {
    match control::check_status() {
        Ok(true) => {
            println!("kodegend is running");
            std::process::exit(0);
        }
        Ok(false) => {
            println!("kodegend is stopped");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error checking status: {e:#}");
            std::process::exit(1);
        }
    }
}

/// Handle start command - start the daemon service
fn handle_start() -> Result<()> {
    match control::start_daemon() {
        Ok(()) => {
            println!("kodegend started successfully");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Failed to start: {e:#}");
            std::process::exit(1);
        }
    }
}

/// Handle stop command - stop the daemon service
fn handle_stop() -> Result<()> {
    match control::stop_daemon() {
        Ok(()) => {
            println!("kodegend stopped successfully");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Failed to stop: {e:#}");
            std::process::exit(1);
        }
    }
}

/// Handle restart command - restart the daemon service
fn handle_restart() -> Result<()> {
    match control::restart_daemon() {
        Ok(()) => {
            println!("kodegend restarted successfully");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Failed to restart: {e:#}");
            std::process::exit(1);
        }
    }
}
