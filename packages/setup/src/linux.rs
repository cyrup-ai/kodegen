//! Linux setup - validates build tools

use crate::error::Result;
use crate::config::LinuxSetupConfig;

pub fn show_config() -> Result<()> {
    println!("Linux signing uses GPG. Run 'gpg --list-secret-keys' to see keys.");
    Ok(())
}

pub fn interactive_setup() -> Result<()> {
    println!("\n🐧 Linux Setup");
    println!("Linux code signing uses GPG.");
    println!("\nTo generate a signing key:");
    println!("  gpg --full-generate-key");
    println!("\nTo list existing keys:");
    println!("  gpg --list-secret-keys --keyid-format LONG");
    Ok(())
}

pub fn setup_from_config(_config: &LinuxSetupConfig, _dry_run: bool, _verbose: bool) -> Result<()> {
    println!("Linux setup from config not yet implemented");
    Ok(())
}
