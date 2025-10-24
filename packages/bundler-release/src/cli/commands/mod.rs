//! Command execution functions coordinating all release operations.
//!
//! This module implements the complete release workflow by coordinating
//! all modules and providing comprehensive error handling and user feedback.

// Submodules
mod bundle;
mod cleanup;
mod helpers;
mod preview;
mod release;
mod resume;
mod rollback;
mod runway;
mod status;
mod temp_clone;
mod validate;

use crate::cli::{Args, Command, RuntimeConfig};
use crate::error::Result;

// Import command executors
use bundle::execute_bundle;
use cleanup::execute_cleanup;
use preview::execute_preview;
use release::execute_release;
use resume::execute_resume;
use rollback::execute_rollback;
use status::execute_status;
use validate::execute_validate;

/// Execute the main command based on parsed arguments
pub async fn execute_command(args: Args) -> Result<i32> {
    // Validate arguments
    if let Err(validation_error) = args.validate() {
        // Create output for validation errors (never quiet)
        let output = super::OutputManager::new(false, false);
        output.error(&format!("Invalid arguments: {}", validation_error));
        return Ok(1);
    }

    let config = RuntimeConfig::from(&args);

    // Execute command and handle errors
    match &args.command {
        Command::Release { .. } | Command::Bundle { .. } => {
            // Release and Bundle commands return Result<i32> with exit code
            let result = match &args.command {
                Command::Release { .. } => execute_release(&args, &config).await,
                Command::Bundle { .. } => execute_bundle(&args, &config).await,
                _ => unreachable!(),
            };

            match result {
                Ok(exit_code) => {
                    // Don't print success message here - commands already did
                    Ok(exit_code)
                }
                Err(e) => {
                    config.error_println(&format!(
                        "Command '{}' failed: {}",
                        args.command.name(),
                        e
                    ));

                    // Show recovery suggestions if available
                    if config.is_verbose() {
                        let suggestions = e.recovery_suggestions();
                        if !suggestions.is_empty() {
                            config.println("\n💡 Recovery suggestions:");
                            for suggestion in suggestions {
                                config.println(&format!("  • {}", suggestion));
                            }
                        }
                    }

                    Ok(1)
                }
            }
        }
        _ => {
            // Other commands return Result<()>
            let result = match &args.command {
                Command::Rollback { .. } => execute_rollback(&args, &config).await,
                Command::Resume { .. } => execute_resume(&args, &config).await,
                Command::Status { .. } => execute_status(&args, &config).await,
                Command::Cleanup { .. } => execute_cleanup(&args, &config).await,
                Command::Validate { .. } => execute_validate(&args, &config).await,
                Command::Preview { .. } => execute_preview(&args, &config).await,
                _ => unreachable!(),
            };

            match result {
                Ok(()) => {
                    if !config.is_quiet() {
                        config.success_println(&format!(
                            "Command '{}' completed successfully",
                            args.command.name()
                        ));
                    }
                    Ok(0)
                }
                Err(e) => {
                    config.error_println(&format!(
                        "Command '{}' failed: {}",
                        args.command.name(),
                        e
                    ));

                    // Show recovery suggestions if available
                    if config.is_verbose() {
                        let suggestions = e.recovery_suggestions();
                        if !suggestions.is_empty() {
                            config.println("\n💡 Recovery suggestions:");
                            for suggestion in suggestions {
                                config.println(&format!("  • {}", suggestion));
                            }
                        }
                    }

                    Ok(1)
                }
            }
        }
    }
}
