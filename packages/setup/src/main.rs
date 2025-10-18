use clap::Parser;
use anyhow::Result;
use std::path::PathBuf;

mod config;
mod error;
mod apple_api;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

use config::{SetupConfig, PlatformConfig, MacOSSetupConfig, CertificateType};

#[derive(Parser)]
#[command(name = "kodegen-setup")]
#[command(version, about = "Configure code signing for kodegen daemon")]
struct Cli {
    /// Show current configuration
    #[arg(long)]
    show: bool,
    
    /// Interactive mode (prompt for credentials)
    #[arg(long, short = 'i', conflicts_with = "show")]
    interactive: bool,
    
    /// Path to setup config file (TOML)
    #[arg(long, short = 'c', conflicts_with_all = ["interactive", "show"])]
    config: Option<PathBuf>,
    
    /// App Store Connect Issuer ID (macOS)
    #[arg(long, requires_all = ["key_id", "private_key"])]
    issuer_id: Option<String>,
    
    /// App Store Connect Key ID (macOS)
    #[arg(long, requires_all = ["issuer_id", "private_key"])]
    key_id: Option<String>,
    
    /// Path to .p8 private key file (macOS)
    #[arg(long, requires_all = ["issuer_id", "key_id"])]
    private_key: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if cli.show {
        return show_config();
    }
    
    if cli.interactive {
        return run_interactive();
    }
    
    if let Some(config_path) = cli.config {
        return run_from_config(&config_path);
    }
    
    if let (Some(issuer), Some(key), Some(pk)) = 
        (cli.issuer_id, cli.key_id, cli.private_key) {
        return run_from_args(&issuer, &key, &pk);
    }
    
    // Default: interactive
    println!("No mode specified. Running interactive setup...\n");
    run_interactive()
}

fn show_config() -> Result<()> {
    println!("📋 Current Setup Configuration\n");
    
    #[cfg(target_os = "macos")]
    {
        macos::show_config()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        linux::show_config()?;
    }
    
    #[cfg(target_os = "windows")]
    {
        windows::show_config()?;
    }
    
    Ok(())
}

fn run_interactive() -> Result<()> {
    println!("{}", "=".repeat(60));
    println!("🔧 Kodegen Interactive Setup");
    println!("Platform: {}", std::env::consts::OS);
    println!("{}", "=".repeat(60));
    println!();
    
    #[cfg(target_os = "macos")]
    return macos::interactive_setup();
    
    #[cfg(target_os = "linux")]
    return linux::interactive_setup();
    
    #[cfg(target_os = "windows")]
    return windows::interactive_setup();
}

fn run_from_config(config_path: &PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(config_path)?;
    let config: SetupConfig = toml::from_str(&content)?;
    
    #[cfg(target_os = "macos")]
    {
        if let PlatformConfig::MacOS(macos_config) = config.platform {
            return macos::setup_from_config(&macos_config, config.dry_run, config.verbose);
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let PlatformConfig::Linux(linux_config) = config.platform {
            return linux::setup_from_config(&linux_config, config.dry_run, config.verbose);
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let PlatformConfig::Windows(windows_config) = config.platform {
            return windows::setup_from_config(&windows_config, config.dry_run, config.verbose);
        }
    }
    
    anyhow::bail!("Platform mismatch in config file")
}

fn run_from_args(issuer_id: &str, key_id: &str, private_key: &PathBuf) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let config = MacOSSetupConfig {
            issuer_id: issuer_id.to_string(),
            key_id: key_id.to_string(),
            private_key_path: private_key.clone(),
            certificate_type: CertificateType::DeveloperIdApplication,
            common_name: "Kodegen Helper".to_string(),
            keychain: "login.keychain-db".to_string(),
        };
        return macos::setup_from_config(&config, false, false);
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("API credentials only apply to macOS setup")
    }
}
