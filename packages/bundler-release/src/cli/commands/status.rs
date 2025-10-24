//! Status command implementation.
//!
//! Displays the current state of an active release.

use crate::cli::{Args, Command, RuntimeConfig};
use crate::error::{ReleaseError, Result};
use crate::state::{create_state_manager_at, has_active_release_at};

/// Execute status command
pub(super) async fn execute_status(args: &Args, config: &RuntimeConfig) -> Result<()> {
    if let Command::Status {
        detailed,
        history: _,
        json,
    } = &args.command
    {
        config.verbose_println("Checking release status...");

        if !has_active_release_at(&config.state_file_path) {
            if *json {
                println!("{{\"status\": \"no_active_release\"}}");
            } else {
                config.println("No active release found");
            }
            return Ok(());
        }

        // Load release state
        let mut state_manager = create_state_manager_at(&config.state_file_path)?;
        let load_result = state_manager.load_state().await?;
        let release_state = load_result.state;

        if *json {
            let json_output =
                serde_json::to_string_pretty(&release_state).map_err(ReleaseError::Json)?;
            println!("{}", json_output);
        } else {
            config.println(&format!("📊 {}", release_state.summary()));

            if *detailed {
                config.println(&format!("Release ID: {}", release_state.release_id));
                config.println(&format!("Started: {}", release_state.started_at));
                config.println(&format!("Updated: {}", release_state.updated_at));
                config.println(&format!(
                    "Elapsed: {}",
                    release_state.elapsed_time().num_seconds()
                ));

                if !release_state.checkpoints.is_empty() {
                    config.println("\nCheckpoints:");
                    for checkpoint in &release_state.checkpoints {
                        config
                            .println(&format!("  ✓ {} ({:?})", checkpoint.name, checkpoint.phase));
                    }
                }

                if !release_state.errors.is_empty() {
                    config.println("\nErrors:");
                    for error in &release_state.errors {
                        let recoverable = if error.recoverable {
                            "recoverable"
                        } else {
                            "critical"
                        };
                        config.println(&format!("  ❌ {} ({})", error.message, recoverable));
                    }
                }
            }
        }
    } else {
        unreachable!("execute_status called with non-Status command");
    }

    Ok(())
}
