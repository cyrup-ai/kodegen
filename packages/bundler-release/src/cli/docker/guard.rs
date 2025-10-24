//! RAII guard for Docker container cleanup.
//!
//! Ensures containers are properly cleaned up even on panic or error.

use std::time::Duration;
use wait_timeout::ChildExt;

/// RAII guard for Docker container cleanup.
///
/// Automatically removes containers when dropped, ensuring cleanup even on panic or error.
/// Uses bounded timeout to prevent infinite hangs if Docker daemon becomes unresponsive.
pub(super) struct ContainerGuard {
    pub(super) name: String,
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        // Best-effort cleanup with timeout protection
        // We use spawn() + wait_timeout() instead of output() to avoid infinite hangs

        // Attempt to spawn docker command
        let mut child = match std::process::Command::new("docker")
            .args(["rm", "-f", &self.name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => {
                // Can't even spawn docker command (e.g., binary not found)
                // Nothing we can do, give up gracefully
                return;
            }
        };

        // Wait up to 5 seconds for cleanup to complete
        // Docker daemon should respond instantly if alive (just removing a container entry)
        let timeout = Duration::from_secs(5);
        match child.wait_timeout(timeout) {
            Ok(Some(status)) => {
                // Command completed (successfully or with error)
                // Either way, we're done - this is best-effort cleanup
                if !status.success() {
                    // Optional: Log cleanup failure for debugging
                    // We don't panic or propagate error since we're already in cleanup path
                    eprintln!(
                        "Warning: Failed to cleanup container '{}' (exit code: {})",
                        self.name,
                        status.code().unwrap_or(-1)
                    );
                }
            }
            Ok(None) => {
                // Timeout reached - Docker daemon is unresponsive
                // Kill the hanging docker command to prevent zombie process
                let _ = child.kill();
                let _ = child.wait(); // Reap zombie process

                eprintln!(
                    "Warning: Timed out cleaning up container '{}' after {} seconds. \
                     Docker daemon may be down.",
                    self.name,
                    timeout.as_secs()
                );
            }
            Err(_) => {
                // Error while waiting (rare)
                // Try to kill the process to prevent zombie
                let _ = child.kill();
                let _ = child.wait();
            }
        }

        // Note: We deliberately ignore all errors and don't panic
        // Drop must never panic, and we're already in an error/cleanup path
    }
}
