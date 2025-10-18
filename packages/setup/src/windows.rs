//! Windows setup - validates SignTool and certificates

use crate::error::Result;
use crate::config::WindowsSetupConfig;

pub fn show_config() -> Result<()> {
    println!("Windows signing uses Authenticode. Check certificates in certmgr.msc.");
    Ok(())
}

pub fn interactive_setup() -> Result<()> {
    println!("\n🪟 Windows Setup");
    println!("Windows code signing uses Authenticode certificates.");
    println!("\nTo import a certificate:");
    println!("  certutil -user -importpfx code_signing_cert.pfx");
    println!("\nTo view installed certificates:");
    println!("  certmgr.msc");
    Ok(())
}

pub fn setup_from_config(_config: &WindowsSetupConfig, _dry_run: bool, _verbose: bool) -> Result<()> {
    println!("Windows setup from config not yet implemented");
    Ok(())
}
