use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use dashmap::DashMap;
use log::{debug, error, info, warn};
use tokio::fs;
use watchexec::Watchexec;
use watchexec_events::Tag;
use watchexec_signals::Signal;

use crate::ClientConfigPlugin;

/// Simple auto-configuration watcher
pub struct AutoConfigWatcher {
    clients: Vec<Arc<dyn ClientConfigPlugin>>,
    processing_files: Arc<DashMap<PathBuf, ()>>,
    active_tasks: Arc<AtomicUsize>,
}

impl AutoConfigWatcher {
    /// Create a new watcher
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be initialized.
    pub fn new(clients: Vec<Arc<dyn ClientConfigPlugin>>) -> Result<Self> {
        Ok(Self {
            clients,
            processing_files: Arc::new(DashMap::new()),
            active_tasks: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Run the watcher with event-driven file system watching
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be started or encounters critical errors.
    pub async fn run(self) -> Result<()> {
        info!("🔍 Scanning for client installations...");

        // Perform initial scan
        self.perform_initial_scan().await?;

        info!("✅ Initial scan complete. Setting up file watchers...");

        // Build list of all paths to watch
        let watch_paths: Vec<PathBuf> = self
            .clients
            .iter()
            .flat_map(|client| {
                client.watch_paths().into_iter().chain(
                    client
                        .config_paths()
                        .into_iter()
                        .filter_map(|cp| cp.path.parent().map(std::path::Path::to_path_buf)),
                )
            })
            .collect();

        if watch_paths.is_empty() {
            warn!("No paths to watch - exiting");
            return Ok(());
        }

        // Create the watchexec instance with event handler
        let clients = self.clients.clone();
        let processing_files = self.processing_files.clone();
        let active_tasks = self.active_tasks.clone();
        let wx = Watchexec::new(move |mut action| {
            // Extract file system events
            for event in action.events.iter() {
                for tag in &event.tags {
                    if let Tag::Path { path, .. } = tag {
                        // Find which client owns this path
                        for client in &clients {
                            for config_path in client.config_paths() {
                                // Only process the exact config file
                                if config_path.path == *path {
                                    info!(
                                        "📝 Config change detected for {}: {}",
                                        client.client_name(),
                                        path.display()
                                    );

                                    let config_path_clone = config_path.path.clone();

                                    // Check if already processing this file
                                    if processing_files.contains_key(&config_path_clone) {
                                        debug!(
                                            "Already processing {}, skipping duplicate event",
                                            config_path_clone.display()
                                        );
                                        break;
                                    }

                                    // Mark as in-progress
                                    processing_files.insert(config_path_clone.clone(), ());
                                    active_tasks.fetch_add(1, Ordering::SeqCst);

                                    // Process the config file asynchronously
                                    let client_clone = client.clone();
                                    let processing_files_clone = processing_files.clone();
                                    let active_tasks_clone = active_tasks.clone();
                                    tokio::spawn(async move {
                                        let result = Self::process_config_file_static(
                                            client_clone.as_ref(),
                                            &config_path_clone,
                                        )
                                        .await;

                                        // Remove from in-progress when done
                                        processing_files_clone.remove(&config_path_clone);
                                        active_tasks_clone.fetch_sub(1, Ordering::SeqCst);

                                        if let Err(e) = result {
                                            error!("Failed to process config: {e}");
                                        }
                                    });

                                    break; // Found the matching config, no need to check others
                                }
                            }
                        }
                    }
                }
            }

            // Handle shutdown signals
            if action
                .signals()
                .any(|sig| matches!(sig, Signal::Interrupt | Signal::Terminate))
            {
                info!(
                    "🛑 Received shutdown signal, waiting for {} active tasks...",
                    active_tasks.load(Ordering::SeqCst)
                );

                // Wait for all tasks to complete
                while active_tasks.load(Ordering::SeqCst) > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }

                info!("✅ All tasks completed, shutting down");
                action.quit();
            }

            action
        })?;

        // Configure the paths to watch
        info!("👁️  Watching {} directories for changes", watch_paths.len());
        for path in &watch_paths {
            if path.exists() {
                info!("   - {}", path.display());
            }
        }
        wx.config.pathset(watch_paths);

        // Start the watchexec main loop
        let main = wx.main();

        // Run until shutdown
        match main.await {
            Ok(_) => {
                info!("✅ Watcher shut down gracefully");
                Ok(())
            }
            Err(e) => {
                error!("❌ Watcher error: {e}");
                Err(e.into())
            }
        }
    }

    /// Perform the initial scan of all clients
    async fn perform_initial_scan(&self) -> Result<()> {
        for client in &self.clients {
            info!("Checking for {} installation", client.client_name());

            for watch_path in client.watch_paths() {
                if client.is_installed(&watch_path) {
                    info!("Found {} at {}", client.client_name(), watch_path.display());

                    for config_path in client.config_paths() {
                        if let Err(e) = self
                            .process_config_file(client.as_ref(), &config_path.path)
                            .await
                        {
                            error!(
                                "Failed to process config for {}: {}",
                                client.client_name(),
                                e
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Process a single config file (shared implementation)
    async fn process_config_file_impl(client: &dyn ClientConfigPlugin, path: &Path) -> Result<()> {
        // Read existing config if it exists
        let config_content = match fs::read_to_string(path).await {
            Ok(content) => content,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                // Config doesn't exist yet - create it
                let new_config = client.inject_kodegen("{}", client.config_format())?;

                // Ensure directory exists
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).await?;
                }

                // Write new config
                fs::write(path, &new_config).await?;
                info!(
                    "Created KODEGEN.ᴀɪ config for {} at {}",
                    client.client_name(),
                    path.display()
                );

                return Ok(());
            }
            Err(e) => {
                // Propagate real errors (permissions, I/O, etc.)
                return Err(e.into());
            }
        };

        // Check if already configured (fast string search)
        if config_content.contains("kodegen") {
            debug!("KODEGEN.ᴀɪ already configured for {}", client.client_name());
            return Ok(());
        }

        // Inject configuration
        let updated_config = client.inject_kodegen(&config_content, client.config_format())?;

        // Create backup with preserved filename
        let backup_path = {
            let mut bp = path.to_path_buf();
            if let Some(filename) = bp.file_name() {
                let mut new_name = filename.to_os_string();
                new_name.push(".backup");
                bp.set_file_name(new_name);
            }
            bp
        };

        // Fail-fast if backup fails (don't risk data loss)
        fs::copy(path, &backup_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create backup: {e}"))?;

        // Write updated config
        fs::write(path, &updated_config).await?;

        info!(
            "Injected KODEGEN.ᴀɪ config for {} at {}",
            client.client_name(),
            path.display()
        );

        Ok(())
    }

    /// Static version for use in watchexec callback
    async fn process_config_file_static(
        client: &dyn ClientConfigPlugin,
        path: &Path,
    ) -> Result<()> {
        Self::process_config_file_impl(client, path).await
    }

    /// Process a single config file
    async fn process_config_file(
        &self,
        client: &dyn ClientConfigPlugin,
        path: &Path,
    ) -> Result<()> {
        Self::process_config_file_impl(client, path).await
    }
}
