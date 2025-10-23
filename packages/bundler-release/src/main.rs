//! Cyrup Release - Production-quality release management for Rust workspaces.
//!
//! This binary provides atomic release operations with proper error handling,
//! automatic internal dependency version synchronization, and rollback capabilities.

use kodegen_bundler_release::cli::OutputManager;
use kodegen_bundler_release::cli;
use std::process;

#[tokio::main]
async fn main() {
    env_logger::init();

    // Source ~/.zshrc to load environment variables (APPLE_CERTIFICATE, etc.)
    // This is critical for code signing to work properly
    if let Some(home) = dirs::home_dir() {
        let zshrc = home.join(".zshrc");
        if zshrc.exists() {
            // Run shell to source .zshrc and export all environment variables
            if let Ok(output) = std::process::Command::new("zsh")
                .arg("-c")
                .arg(format!("source {} && env", zshrc.display()))
                .output()
                && output.status.success()
            {
                let env_output = String::from_utf8_lossy(&output.stdout);
                // Parse and set environment variables
                for line in env_output.lines() {
                    if let Some((key, value)) = line.split_once('=') {
                        // SAFETY: We're setting environment variables from ~/.zshrc
                        // This is necessary for code signing to access APPLE_CERTIFICATE env vars
                        unsafe {
                            std::env::set_var(key, value);
                        }
                    }
                }
            }
        }
    }

    match cli::run().await {
        Ok(exit_code) => {
            process::exit(exit_code);
        }
        Err(e) => {
            // Create output manager for error display (never quiet for fatal errors)
            let output = OutputManager::new(false, false);
            output.error(&format!("Fatal error: {e}"));
            
            // Show recovery suggestions for critical errors
            let suggestions = e.recovery_suggestions();
            if !suggestions.is_empty() {
                output.println("\n💡 Recovery suggestions:");
                for suggestion in suggestions {
                    output.indent(&suggestion);
                }
            }
            
            process::exit(1);
        }
    }
}