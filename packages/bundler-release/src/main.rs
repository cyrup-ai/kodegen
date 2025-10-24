//! Cyrup Release - Production-quality release management for Rust workspaces.
//!
//! This binary provides atomic release operations with proper error handling,
//! automatic internal dependency version synchronization, and rollback capabilities.

use kodegen_bundler_release::cli;
use kodegen_bundler_release::cli::OutputManager;
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
                // This needs to handle multi-line values (e.g., APPLE_API_KEY_CONTENT with embedded newlines)
                let mut current_key: Option<String> = None;
                let mut current_value = String::new();

                for line in env_output.lines() {
                    // Check if this line starts a new key=value pair
                    if let Some((key, value)) = line.split_once('=') {
                        // First, save the previous key-value pair if any
                        if let Some(prev_key) = current_key.take() {
                            unsafe {
                                std::env::set_var(prev_key, current_value.trim_end());
                            }
                            current_value.clear();
                        }

                        // Start accumulating the new key-value pair
                        current_key = Some(key.to_string());
                        current_value.push_str(value);
                    } else {
                        // This is a continuation line of a multi-line value
                        if current_key.is_some() {
                            current_value.push('\n');
                            current_value.push_str(line);
                        }
                    }
                }

                // Don't forget the last key-value pair
                if let Some(key) = current_key {
                    unsafe {
                        std::env::set_var(key, current_value.trim_end());
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
