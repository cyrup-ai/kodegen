//! Realtime Context Updates via File Watching
//!
//! This module provides file watching capabilities for automatic context updates
//! when files change on disk. Uses the `notify` crate for cross-platform file
//! system event monitoring.

use log::{debug, error};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Events representing file system changes
#[derive(Debug, Clone)]
pub enum ContextUpdateEvent {
    /// A file was modified
    FileModified(PathBuf),
    /// A new file was created
    FileCreated(PathBuf),
    /// A file was deleted
    FileDeleted(PathBuf),
}

/// Provider for realtime context updates via file watching
pub struct RealtimeContextProvider {
    watcher: Option<RecommendedWatcher>,
    watched_paths: Vec<PathBuf>,
    event_tx: mpsc::UnboundedSender<ContextUpdateEvent>,
    event_rx: mpsc::UnboundedReceiver<ContextUpdateEvent>,
}

impl RealtimeContextProvider {
    /// Create a new realtime context provider
    #[must_use]
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            watcher: None,
            watched_paths: Vec::new(),
            event_tx,
            event_rx,
        }
    }

    /// Enable realtime updates for specified paths
    ///
    /// Sets up file watching for the given paths. Any modifications, creations,
    /// or deletions within these paths will generate events.
    ///
    /// # Arguments
    /// * `paths` - Paths to watch for changes
    ///
    /// # Errors
    /// Returns an error if:
    /// - File watching cannot be initialized
    /// - Any of the paths cannot be watched (permissions, doesn't exist, etc.)
    pub fn enable_realtime_updates(&mut self, paths: Vec<PathBuf>) -> anyhow::Result<()> {
        let event_tx = self.event_tx.clone();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => match event.kind {
                    EventKind::Modify(_) => {
                        for path in event.paths {
                            let _ = event_tx.send(ContextUpdateEvent::FileModified(path));
                        }
                    }
                    EventKind::Create(_) => {
                        for path in event.paths {
                            let _ = event_tx.send(ContextUpdateEvent::FileCreated(path));
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            let _ = event_tx.send(ContextUpdateEvent::FileDeleted(path));
                        }
                    }
                    _ => {}
                },
                Err(e) => {
                    error!("File watch error: {e:?}");
                }
            })?;

        // Watch all specified paths
        for path in &paths {
            watcher.watch(path, RecursiveMode::Recursive)?;
        }

        self.watcher = Some(watcher);
        self.watched_paths = paths;
        Ok(())
    }

    /// Start event processing loop
    ///
    /// Spawns a background task that processes file system events and calls
    /// the provided callback for each event.
    ///
    /// # Arguments
    /// * `on_update` - Callback function to handle context update events
    ///
    /// # Returns
    /// A `JoinHandle` for the background task, allowing graceful shutdown
    pub fn start_event_loop<F>(&mut self, on_update: F) -> tokio::task::JoinHandle<()>
    where
        F: Fn(ContextUpdateEvent) + Send + 'static,
    {
        // Take ownership of the receiver to move it into the spawned task
        let (tx, mut rx) = mpsc::unbounded_channel();
        std::mem::swap(&mut self.event_rx, &mut rx);
        self.event_tx = tx;

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                debug!("Context update event: {event:?}");
                on_update(event);
            }
        })
    }

    /// Stop watching files
    ///
    /// Stops all file watching and cleans up resources. This is called
    /// automatically when the provider is dropped.
    pub fn stop_watching(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
        }
        self.watched_paths.clear();
    }
}

impl Default for RealtimeContextProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RealtimeContextProvider {
    fn drop(&mut self) {
        self.stop_watching();
    }
}
