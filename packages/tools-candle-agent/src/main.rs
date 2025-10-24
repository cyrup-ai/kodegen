//! Candle Chat CLI - Interactive AI chat application
//!
//! This binary demonstrates the CLI module with model selection, configuration,
//! and interactive chat capabilities.

use log::error;
use rustls::crypto::aws_lc_rs;

use kodegen_candle_agent::cli::{CliArgs, CliRunner};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize env_logger - only show warnings/errors for clean UI
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Warn)
        .init();

    // Initialize Rustls crypto provider for TLS/HTTPS connections
    aws_lc_rs::default_provider()
        .install_default()
        .map_err(|e| format!("Failed to install rustls crypto provider: {:?}", e))?;

    // Initialize Candle performance optimizations
    kodegen_candle_agent::init_candle();

    // Parse CLI arguments
    let args = CliArgs::from_env();

    // Create and run CLI
    let mut runner = match CliRunner::new(args) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to initialize CLI: {}", e);
            std::process::exit(1);
        }
    };

    // Run the interactive chat
    if let Err(e) = runner.run().await {
        error!("CLI error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
