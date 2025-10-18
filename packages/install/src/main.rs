mod install;
mod signing;

use clap::Parser;
use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kodegen-install")]
#[command(version, about = "Install kodegen daemon as a system service")]
struct Cli {
    /// Path to kodegend binary to install
    #[arg(long, default_value = "./target/release/kodegend")]
    binary: PathBuf,

    /// Install system-wide (requires sudo/admin)
    #[arg(long)]
    system_wide: bool,

    /// Don't start service after install
    #[arg(long)]
    no_start: bool,

    /// Show what would be done without doing it
    #[arg(long)]
    dry_run: bool,

    /// Uninstall instead of install
    #[arg(long)]
    uninstall: bool,

    /// Sign the daemon binary after installation
    #[arg(long)]
    sign: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = Cli::parse();

    if cli.uninstall {
        return run_uninstall(&cli).await;
    }

    run_install(&cli).await
}

async fn run_install(cli: &Cli) -> Result<()> {
    println!("🔧 Kodegen Daemon Installation");
    println!("Platform: {}\n", std::env::consts::OS);

    // Verify binary exists
    if !cli.binary.exists() {
        anyhow::bail!("Binary not found: {}", cli.binary.display());
    }

    // Get config path
    let config_path = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("kodegen")
        .join("config.toml");

    // Call the actual installation logic
    install::config::install_kodegen_daemon(cli.binary.clone(), config_path, cli.sign)
        .await
        .context("Installation failed")?;

    println!("✅ Installation completed successfully!");
    Ok(())
}

async fn run_uninstall(_cli: &Cli) -> Result<()> {
    println!("🗑️  Kodegen Daemon Uninstallation\n");

    // Call the actual uninstallation logic
    install::uninstall::uninstall_kodegen_daemon()
        .await
        .context("Uninstallation failed")?;

    println!("✅ Uninstallation completed successfully!");
    Ok(())
}
