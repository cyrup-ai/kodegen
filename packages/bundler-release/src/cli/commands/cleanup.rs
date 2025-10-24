//! Cleanup command implementation.
//!
//! Removes release state files and temporary data.

use crate::cli::{Args, Command, RuntimeConfig};
use crate::error::Result;
use crate::state::{create_state_manager_at, has_active_release_at};

/// Execute cleanup command
pub(super) async fn execute_cleanup(args: &Args, config: &RuntimeConfig) -> Result<()> {
    if let Command::Cleanup {
        all,
        older_than,
        yes,
    } = &args.command
    {
        config.verbose_println("Cleaning up state files...");

        if !yes {
            config.println("About to clean up release state files");
            // In a real CLI, you'd prompt for confirmation here
        }

        let state_manager = create_state_manager_at(&config.state_file_path)?;

        if *all || older_than.is_some() {
            state_manager.cleanup_state()?;
            config.success_println("State files cleaned up");
        } else {
            // Just clean up current state
            if has_active_release_at(&config.state_file_path) {
                state_manager.cleanup_state()?;
                config.success_println("Current state file cleaned up");
            } else {
                config.println("No state files to clean up");
            }
        }
    } else {
        unreachable!("execute_cleanup called with non-Cleanup command");
    }

    Ok(())
}
