//! RAII guard for Docker container cleanup.
//!
//! Ensures containers are properly cleaned up even on panic or error.

/// RAII guard for Docker container cleanup.
///
/// Automatically removes containers when dropped, ensuring cleanup even on panic or error.
/// Follows the same Drop pattern as StateManager in state/manager.rs.
pub(super) struct ContainerGuard {
    pub(super) name: String,
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        // Best-effort cleanup - ignore errors as we're already in error/cleanup path
        // Use std::process::Command (synchronous) instead of tokio::process::Command
        // because Drop is a synchronous method and cannot await futures
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", &self.name])
            .output();
        // Note: This runs `docker rm -f <container-name>` which:
        // - Forcefully removes the container (even if running)
        // - Doesn't fail if container doesn't exist
        // - Cleans up container resources
    }
}
