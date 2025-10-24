use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

/// Container for managers that require explicit shutdown
///
/// Category servers populate this based on what managers their tools use.
/// The core server handles calling shutdown() on graceful termination.
#[derive(Default)]
pub struct Managers {
    shutdown_hooks: Vec<Box<dyn ShutdownHook>>,
}

/// Trait for components that need graceful shutdown
///
/// Example implementations:
/// - BrowserManager::shutdown() - closes Chrome processes
/// - TunnelGuard::shutdown() - closes SSH tunnels
/// - SearchManager::shutdown() - cancels background search tasks
pub trait ShutdownHook: Send + Sync {
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

impl Managers {
    pub fn new() -> Self {
        Self {
            shutdown_hooks: Vec::new(),
        }
    }

    /// Register a component that needs shutdown
    ///
    /// Example usage in category server:
    /// ```
    /// let browser_manager = Arc::new(BrowserManager::new());
    /// managers.register(browser_manager.clone());
    /// ```
    pub fn register<H: ShutdownHook + 'static>(&mut self, hook: H) {
        self.shutdown_hooks.push(Box::new(hook));
    }

    /// Shutdown all registered managers gracefully
    ///
    /// Called automatically by core server before exit.
    /// Logs warnings for individual failures but continues shutdown.
    pub async fn shutdown(&self) -> Result<()> {
        for (i, hook) in self.shutdown_hooks.iter().enumerate() {
            if let Err(e) = hook.shutdown().await {
                log::warn!("Failed to shutdown manager {}: {}", i, e);
            }
        }
        Ok(())
    }
}
