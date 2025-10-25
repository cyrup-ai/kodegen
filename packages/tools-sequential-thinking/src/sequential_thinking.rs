use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::reasoning::{SequentialThinkingArgs, SequentialThinkingPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// INTERNAL STATE
// ============================================================================

/// Internal representation of a thought
///
/// Stored in `thought_history` and branches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ThoughtData {
    pub thought: String,
    pub thought_number: u32,
    pub total_thoughts: u32,
    pub next_thought_needed: bool,
    pub is_revision: Option<bool>,
    pub revises_thought: Option<u32>,
    pub branch_from_thought: Option<u32>,
    pub branch_id: Option<String>,
    pub needs_more_thoughts: Option<bool>,
}

/// Internal state tracking all thoughts for a single session
///
/// Each session actor task owns an instance of this state directly (no locks!)
#[derive(Debug, Default)]
struct ThinkingState {
    /// Linear history of all thoughts in this session
    thought_history: Vec<ThoughtData>,

    /// Branched thoughts organized by `branch_id`
    branches: HashMap<String, Vec<ThoughtData>>,
}

// ============================================================================
// SESSION COMMAND TYPES (MPSC Actor Pattern)
// ============================================================================

/// Commands sent to session actor task via MPSC
enum SessionCommand {
    /// Add a new thought to this session's history
    AddThought {
        thought: ThoughtData,
        /// Response channel for returning updated state
        respond_to: tokio::sync::oneshot::Sender<SessionResponse>,
    },

    /// Get current session state (for future features)
    GetState {
        respond_to: tokio::sync::oneshot::Sender<SessionStateSnapshot>,
    },

    /// Clear this session's history (for future features)
    Clear {
        respond_to: tokio::sync::oneshot::Sender<()>,
    },
}

/// Response from session actor
#[derive(Debug, Clone, Serialize)]
struct SessionResponse {
    thought_number: u32,
    total_thoughts: u32,
    next_thought_needed: bool,
    branches: Vec<String>,
    thought_history_length: usize,
}

/// Complete session state snapshot (for debugging or persistence)
#[derive(Debug, Clone, Serialize)]
pub struct SessionStateSnapshot {
    pub thought_history: Vec<ThoughtData>,
    pub branches: HashMap<String, Vec<ThoughtData>>,
}

// ============================================================================
// PERSISTENCE TYPES
// ============================================================================

/// Persistence configuration for orphaned sessions
struct PersistenceConfig {
    /// Base directory: $`XDG_CONFIG_HOME/kodegen/sequential_thinking`/
    sessions_dir: PathBuf,

    /// Age before disk cleanup (24 hours)
    cleanup_after: Duration,
}

impl PersistenceConfig {
    fn default() -> Self {
        let base_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kodegen-mcp")
            .join("sequential_thinking");

        Self {
            sessions_dir: base_dir,
            cleanup_after: Duration::from_secs(24 * 60 * 60),
        }
    }
}

/// Commands for persistence background task
enum PersistenceCommand {
    /// Persist a session to disk
    Persist {
        session_id: String,
        snapshot: SessionStateSnapshot,
        created_at: std::time::SystemTime,
        last_activity: std::time::SystemTime,
    },

    /// Delete a session from disk
    Delete { session_id: String },
}

/// Session metadata file (persisted as session.json)
#[derive(Debug, Serialize, Deserialize)]
struct SessionMetadataFile {
    session_id: String,
    created_at: std::time::SystemTime,
    last_activity: std::time::SystemTime,
    total_thoughts: usize,
    branch_ids: Vec<String>,
}

/// Individual thought file (persisted as thought{n}.json)
#[derive(Debug, Serialize, Deserialize)]
struct PersistedThought {
    thought_number: u32,
    thought_data: ThoughtData,
}

// ============================================================================
// SESSION HANDLE
// ============================================================================

/// Handle to a running session actor
#[derive(Clone)]
struct SessionHandle {
    /// Channel to send commands to the session's actor task
    tx: tokio::sync::mpsc::Sender<SessionCommand>,
    /// When this session was created (for potential future runtime reporting)
    created_at: Instant,
    /// Last time a command was processed (used for cleanup)
    last_activity: Arc<RwLock<Instant>>,
}

// ============================================================================
// SESSION ACTOR TASK
// ============================================================================

/// Spawn session actor with optional initial state
///
/// The spawned task exclusively owns the `ThinkingState` for this session.
/// No locks needed within the task since only this task accesses the state.
fn spawn_session_actor_with_state(
    mut rx: tokio::sync::mpsc::Receiver<SessionCommand>,
    disable_logging: bool,
    initial_state: ThinkingState,
) {
    tokio::spawn(async move {
        // Task OWNS the state - no locks needed!
        let mut state = initial_state;

        // Process commands until channel closes
        while let Some(cmd) = rx.recv().await {
            match cmd {
                SessionCommand::AddThought {
                    thought,
                    respond_to,
                } => {
                    // Update state (lock-free - we own it!)
                    state.thought_history.push(thought.clone());

                    // Add to branch if applicable
                    if let (Some(_), Some(branch_id)) =
                        (thought.branch_from_thought, &thought.branch_id)
                    {
                        state
                            .branches
                            .entry(branch_id.clone())
                            .or_default()
                            .push(thought.clone());
                    }

                    // Build response
                    let response = SessionResponse {
                        thought_number: thought.thought_number,
                        total_thoughts: thought.total_thoughts,
                        next_thought_needed: thought.next_thought_needed,
                        branches: state.branches.keys().cloned().collect(),
                        thought_history_length: state.thought_history.len(),
                    };

                    // Log to stderr if enabled
                    if !disable_logging {
                        let formatted = SequentialThinkingTool::format_thought(&thought);
                        let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
                        let mut buffer = bufwtr.buffer();
                        let _ = write!(&mut buffer, "{formatted}");
                        let _ = bufwtr.print(&buffer);
                    }

                    // Send response (ignore if receiver dropped)
                    let _ = respond_to.send(response);

                    // Terminate session if thinking is complete
                    if !thought.next_thought_needed {
                        log::debug!(
                            "Session completed (final thought {}), terminating actor",
                            thought.thought_number
                        );
                        break;
                    }
                }

                SessionCommand::GetState { respond_to } => {
                    let snapshot = SessionStateSnapshot {
                        thought_history: state.thought_history.clone(),
                        branches: state.branches.clone(),
                    };
                    let _ = respond_to.send(snapshot);
                }

                SessionCommand::Clear { respond_to } => {
                    state.thought_history.clear();
                    state.branches.clear();
                    let _ = respond_to.send(());
                    log::debug!("Session cleared, terminating actor");
                    break;
                }
            }
        }
        // Channel closed - session terminated, state automatically cleaned up
        log::debug!("Session actor task terminated, state cleaned up");
    });
}

/// Spawn new session actor with empty state
fn spawn_session_actor(rx: tokio::sync::mpsc::Receiver<SessionCommand>, disable_logging: bool) {
    // Delegate to _with_state with default state
    spawn_session_actor_with_state(rx, disable_logging, ThinkingState::default());
}

// ============================================================================
// TOOL STRUCT (SESSION MANAGER)
// ============================================================================

/// Sequential Thinking tool using MPSC actor pattern for session management
///
/// Each session has an isolated async task that owns its state directly.
/// This eliminates lock contention and provides perfect isolation between users.
#[derive(Clone)]
pub struct SequentialThinkingTool {
    /// Active session handles (only stores channel senders, not state)
    sessions: Arc<RwLock<HashMap<String, SessionHandle>>>,

    /// Whether to disable stderr logging
    /// Controlled by environment variable `DISABLE_THOUGHT_LOGGING=true`
    disable_logging: bool,

    /// Fire-and-forget channel for persistence requests
    persistence_sender: tokio::sync::mpsc::UnboundedSender<PersistenceCommand>,
}

impl Default for SequentialThinkingTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SequentialThinkingTool {
    /// Create a new `SequentialThinkingTool` instance
    ///
    /// Checks the `DISABLE_THOUGHT_LOGGING` environment variable on instantiation.
    #[must_use]
    pub fn new() -> Self {
        let disable_logging = std::env::var("DISABLE_THOUGHT_LOGGING")
            .unwrap_or_default()
            .to_lowercase()
            == "true";

        // Create persistence channel
        let (persistence_sender, persistence_receiver) = tokio::sync::mpsc::unbounded_channel();

        let tool = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            disable_logging,
            persistence_sender: persistence_sender.clone(),
        };

        // Start background persistence processor
        tool.start_persistence_processor(persistence_receiver);

        // Start hourly disk cleanup task
        Self::start_disk_cleanup_task(persistence_sender);

        tool
    }

    /// Generate unique session ID using UUID v4
    fn generate_session_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Get or create a session
    async fn get_or_create_session(
        &self,
        session_id: Option<String>,
    ) -> Result<(String, tokio::sync::mpsc::Sender<SessionCommand>), McpError> {
        // Generate session ID if not provided
        let session_id = match session_id {
            Some(id) => id,
            None => self.generate_session_id(),
        };

        // Check if session exists in memory
        {
            let sessions = self.sessions.read().await;
            if let Some(handle) = sessions.get(&session_id) {
                // Update last activity
                *handle.last_activity.write().await = Instant::now();
                return Ok((session_id, handle.tx.clone()));
            }
        }

        // Try to restore from disk before creating new session
        if let Some(restored_handle) = self.try_restore_session(&session_id).await {
            // Add restored session to active sessions
            let tx = restored_handle.tx.clone();
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), restored_handle);
            return Ok((session_id, tx));
        }

        // Create new session if not found in memory or disk
        let (tx, rx) = tokio::sync::mpsc::channel::<SessionCommand>(100);

        // Spawn actor task
        spawn_session_actor(rx, self.disable_logging);

        // Store handle
        let handle = SessionHandle {
            tx: tx.clone(),
            created_at: Instant::now(),
            last_activity: Arc::new(RwLock::new(Instant::now())),
        };

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), handle);
        }

        Ok((session_id, tx))
    }

    /// Get session state snapshot (for debugging or persistence)
    pub async fn get_session_state(
        &self,
        session_id: &str,
    ) -> Result<SessionStateSnapshot, McpError> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Session not found: {session_id}")))?;

        let (respond_to, rx) = tokio::sync::oneshot::channel();
        let cmd = SessionCommand::GetState { respond_to };

        handle
            .tx
            .send(cmd)
            .await
            .map_err(|_| McpError::Other(anyhow::anyhow!("Session actor terminated")))?;

        rx.await
            .map_err(|_| McpError::Other(anyhow::anyhow!("Failed to receive state")))
    }

    /// Clear a session's history (for starting fresh with same session ID)
    pub async fn clear_session(&self, session_id: &str) -> Result<(), McpError> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Session not found: {session_id}")))?;

        let (respond_to, rx) = tokio::sync::oneshot::channel();
        let cmd = SessionCommand::Clear { respond_to };

        handle
            .tx
            .send(cmd)
            .await
            .map_err(|_| McpError::Other(anyhow::anyhow!("Session actor terminated")))?;

        rx.await
            .map_err(|_| McpError::Other(anyhow::anyhow!("Failed to clear session")))
    }

    /// Get session info including creation time and activity
    pub async fn get_session_info(&self, session_id: &str) -> Result<(Instant, Instant), McpError> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Session not found: {session_id}")))?;

        let created_at = handle.created_at;
        let last_activity = *handle.last_activity.read().await;

        Ok((created_at, last_activity))
    }

    /// Start background task to handle persistence commands
    fn start_persistence_processor(
        &self,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<PersistenceCommand>,
    ) {
        let config = PersistenceConfig::default();

        tokio::spawn(async move {
            // Create base directory once
            if let Err(e) = tokio::fs::create_dir_all(&config.sessions_dir).await {
                log::error!("Failed to create sessions directory: {e}");
            }

            // Process commands until channel closes
            while let Some(cmd) = receiver.recv().await {
                match cmd {
                    PersistenceCommand::Persist {
                        session_id,
                        snapshot,
                        created_at,
                        last_activity,
                    } => {
                        if let Err(e) = Self::persist_session_to_disk(
                            &config,
                            &session_id,
                            &snapshot,
                            created_at,
                            last_activity,
                        )
                        .await
                        {
                            log::error!("Failed to persist session {session_id}: {e}");
                        }
                    }

                    PersistenceCommand::Delete { session_id } => {
                        let session_dir = config.sessions_dir.join(&session_id);
                        if let Err(e) = tokio::fs::remove_dir_all(&session_dir).await {
                            log::debug!("Failed to delete session directory {session_id}: {e}");
                        } else {
                            log::info!("Deleted persisted session: {session_id}");
                        }
                    }
                }
            }

            log::debug!("Persistence processor terminated");
        });
    }

    /// Persist a single session to disk (called by background task)
    async fn persist_session_to_disk(
        config: &PersistenceConfig,
        session_id: &str,
        snapshot: &SessionStateSnapshot,
        created_at: std::time::SystemTime,
        last_activity: std::time::SystemTime,
    ) -> Result<(), anyhow::Error> {
        use anyhow::Context;

        // Create session directory: {sessions_dir}/{session-id}/
        let session_dir = config.sessions_dir.join(session_id);
        tokio::fs::create_dir_all(&session_dir)
            .await
            .context("Failed to create session directory")?;

        // Write session metadata file
        let metadata = SessionMetadataFile {
            session_id: session_id.to_string(),
            created_at,
            last_activity,
            total_thoughts: snapshot.thought_history.len(),
            branch_ids: snapshot.branches.keys().cloned().collect(),
        };
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        tokio::fs::write(session_dir.join("session.json"), metadata_json)
            .await
            .context("Failed to write session.json")?;

        // Write individual thought files: thought1.json, thought2.json, ...
        for (idx, thought) in snapshot.thought_history.iter().enumerate() {
            let persisted = PersistedThought {
                thought_number: thought.thought_number,
                thought_data: thought.clone(),
            };
            let thought_json = serde_json::to_string_pretty(&persisted)?;
            let thought_path = session_dir.join(format!("thought{}.json", idx + 1));
            tokio::fs::write(thought_path, thought_json)
                .await
                .with_context(|| format!("Failed to write thought{}.json", idx + 1))?;
        }

        // Write branch files: branch_{branch_id}_thought{n}.json
        for (branch_id, branch_thoughts) in &snapshot.branches {
            for (idx, thought) in branch_thoughts.iter().enumerate() {
                let persisted = PersistedThought {
                    thought_number: thought.thought_number,
                    thought_data: thought.clone(),
                };
                let thought_json = serde_json::to_string_pretty(&persisted)?;
                let branch_path =
                    session_dir.join(format!("branch_{}_thought{}.json", branch_id, idx + 1));
                tokio::fs::write(branch_path, thought_json)
                    .await
                    .with_context(|| format!("Failed to write branch file for {branch_id}"))?;
            }
        }

        log::info!(
            "Persisted session {} ({} thoughts) to {:?}",
            session_id,
            snapshot.thought_history.len(),
            session_dir
        );

        Ok(())
    }

    /// Attempt to restore session from disk
    /// Returns None if session doesn't exist on disk or restoration fails
    async fn try_restore_session(&self, session_id: &str) -> Option<SessionHandle> {
        let config = PersistenceConfig::default();
        let session_dir = config.sessions_dir.join(session_id);

        // Check if session directory exists (async)
        if !tokio::fs::try_exists(&session_dir).await.unwrap_or(false) {
            return None;
        }

        log::debug!("Attempting to restore session {session_id} from disk");

        // Read session metadata
        let metadata_path = session_dir.join("session.json");
        let metadata_json = tokio::fs::read_to_string(metadata_path).await.ok()?;
        let metadata: SessionMetadataFile = serde_json::from_str(&metadata_json).ok()?;

        // Read all thought files in order
        let mut thought_history = Vec::new();
        for idx in 1..=metadata.total_thoughts {
            let thought_path = session_dir.join(format!("thought{idx}.json"));
            if let Ok(thought_json) = tokio::fs::read_to_string(thought_path).await
                && let Ok(persisted) = serde_json::from_str::<PersistedThought>(&thought_json)
            {
                thought_history.push(persisted.thought_data);
            }
        }

        // Read branch files
        let mut branches = HashMap::new();
        for branch_id in &metadata.branch_ids {
            let mut branch_thoughts = Vec::new();
            let mut idx = 1;
            loop {
                let branch_path = session_dir.join(format!("branch_{branch_id}_thought{idx}.json"));
                match tokio::fs::read_to_string(branch_path).await {
                    Ok(thought_json) => {
                        if let Ok(persisted) =
                            serde_json::from_str::<PersistedThought>(&thought_json)
                        {
                            branch_thoughts.push(persisted.thought_data);
                            idx += 1;
                        } else {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            if !branch_thoughts.is_empty() {
                branches.insert(branch_id.clone(), branch_thoughts);
            }
        }

        log::info!(
            "Restored session {} ({} thoughts, {} branches) from disk",
            session_id,
            thought_history.len(),
            branches.len()
        );

        // Create session actor with restored state
        let (tx, rx) = tokio::sync::mpsc::channel::<SessionCommand>(100);
        let restored_state = ThinkingState {
            thought_history,
            branches,
        };
        spawn_session_actor_with_state(rx, self.disable_logging, restored_state);

        // Calculate original timestamps from metadata
        let created_at_elapsed = metadata.created_at.elapsed().ok()?;
        let created_at = Instant::now()
            .checked_sub(created_at_elapsed)
            .unwrap_or_else(Instant::now);

        let handle = SessionHandle {
            tx,
            created_at,
            last_activity: Arc::new(RwLock::new(Instant::now())), // Reset activity time
        };

        // Delete disk files after successful restoration (session is active again)
        let _ = self.persistence_sender.send(PersistenceCommand::Delete {
            session_id: session_id.to_string(),
        });

        Some(handle)
    }

    /// Start background task to clean up old disk sessions (runs hourly)
    fn start_disk_cleanup_task(
        persistence_sender: tokio::sync::mpsc::UnboundedSender<PersistenceCommand>,
    ) {
        tokio::spawn(async move {
            let config = PersistenceConfig::default();
            let mut interval = tokio::time::interval(Duration::from_secs(60 * 60)); // 1 hour

            loop {
                interval.tick().await;

                log::debug!("Running disk cleanup task");

                // Read all session directories
                let Ok(mut entries) = tokio::fs::read_dir(&config.sessions_dir).await else {
                    continue;
                };

                while let Ok(Some(entry)) = entries.next_entry().await {
                    // Only process directories (session directories)
                    let Ok(file_type) = entry.file_type().await else {
                        continue;
                    };

                    if !file_type.is_dir() {
                        continue;
                    }

                    let path = entry.path();

                    // Read session.json to check age
                    let metadata_path = path.join("session.json");
                    let Ok(metadata_json) = tokio::fs::read_to_string(metadata_path).await else {
                        continue;
                    };

                    let Ok(metadata) = serde_json::from_str::<SessionMetadataFile>(&metadata_json)
                    else {
                        continue;
                    };

                    // Check if session is older than cleanup threshold
                    let age = metadata
                        .last_activity
                        .elapsed()
                        .unwrap_or_else(|_| Duration::from_secs(0));

                    if age > config.cleanup_after {
                        // Send delete command to persistence task
                        log::info!(
                            "Purging old session {} (age: {:.1} hours)",
                            metadata.session_id,
                            age.as_secs_f64() / 3600.0
                        );

                        let _ = persistence_sender.send(PersistenceCommand::Delete {
                            session_id: metadata.session_id,
                        });
                    }
                }
            }
        });
    }

    /// Clean up inactive sessions
    async fn cleanup_sessions(&self, max_age: Duration) {
        let purge_cutoff = Instant::now()
            .checked_sub(max_age)
            .unwrap_or_else(Instant::now);

        let mut sessions = self.sessions.write().await;
        let mut to_persist = Vec::new();

        sessions.retain(|session_id, handle| {
            // Closed channels: session actor terminated, remove immediately
            if handle.tx.is_closed() {
                log::debug!("Removing closed session: {session_id}");
                return false;
            }

            // Check last activity
            let last_activity = handle
                .last_activity
                .try_read()
                .map_or_else(|_| Instant::now(), |t| *t);

            // Old sessions: persist before removal
            if last_activity < purge_cutoff {
                log::debug!("Session {session_id} expired, will persist before removal");
                to_persist.push((session_id.clone(), handle.clone()));
                return false;
            }

            true
        });

        drop(sessions);

        // Persist sessions outside of lock (fire-and-forget)
        for (session_id, handle) in to_persist {
            // Get session state via GetState command
            let (respond_to, rx) = tokio::sync::oneshot::channel();
            if handle
                .tx
                .send(SessionCommand::GetState { respond_to })
                .await
                .is_ok()
                && let Ok(snapshot) = rx.await
            {
                // Convert Instant to SystemTime for persistence
                let created_at_elapsed = handle.created_at.elapsed();
                let created_at = std::time::SystemTime::now()
                    .checked_sub(created_at_elapsed)
                    .unwrap_or_else(std::time::SystemTime::now);

                let last_activity_instant = *handle.last_activity.read().await;
                let last_activity_elapsed = last_activity_instant.elapsed();
                let last_activity = std::time::SystemTime::now()
                    .checked_sub(last_activity_elapsed)
                    .unwrap_or_else(std::time::SystemTime::now);

                // Send to persistence task (fire-and-forget)
                let _ = self.persistence_sender.send(PersistenceCommand::Persist {
                    session_id: session_id.clone(),
                    snapshot,
                    created_at,
                    last_activity,
                });
            }
        }
    }

    /// Start background cleanup task (call once on manager creation)
    /// Pattern from search_manager.rs:565-573
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5 * 60));
            loop {
                interval.tick().await;
                self.cleanup_sessions(Duration::from_secs(30 * 60)).await;
            }
        });
    }

    /// Shutdown the tool gracefully, persisting all active sessions
    ///
    /// Called during server shutdown to ensure no sessions are lost.
    /// Persists all active sessions to disk before terminating.
    pub async fn shutdown(&self) -> Result<(), McpError> {
        log::info!("Shutting down sequential thinking tool, persisting active sessions");

        // Get snapshot of all active sessions
        let sessions = self.sessions.read().await;
        let session_ids: Vec<String> = sessions.keys().cloned().collect();
        drop(sessions);

        log::debug!("Found {} active sessions to persist", session_ids.len());

        // Persist each session
        for session_id in session_ids {
            // Get session state
            if let Ok(snapshot) = self.get_session_state(&session_id).await {
                // Get session handle for timestamps
                let sessions = self.sessions.read().await;
                if let Some(handle) = sessions.get(&session_id) {
                    // Convert Instant → SystemTime (pattern from cleanup_sessions)
                    let created_at_elapsed = handle.created_at.elapsed();
                    let created_at = std::time::SystemTime::now()
                        .checked_sub(created_at_elapsed)
                        .unwrap_or_else(std::time::SystemTime::now);

                    let last_activity_instant = *handle.last_activity.read().await;
                    let last_activity_elapsed = last_activity_instant.elapsed();
                    let last_activity = std::time::SystemTime::now()
                        .checked_sub(last_activity_elapsed)
                        .unwrap_or_else(std::time::SystemTime::now);

                    // Send persistence command (fire-and-forget)
                    let _ = self.persistence_sender.send(PersistenceCommand::Persist {
                        session_id: session_id.clone(),
                        snapshot,
                        created_at,
                        last_activity,
                    });

                    log::debug!("Queued session {} for persistence", session_id);
                }
            }
        }

        // Give persistence task time to process all commands
        // (persistence runs in background, this ensures writes complete)
        tokio::time::sleep(Duration::from_millis(500)).await;

        log::info!("Sequential thinking tool shutdown complete");
        Ok(())
    }

    /// Validate and convert args to `ThoughtData`
    /// Auto-adjusts totalThoughts if thoughtNumber exceeds it
    fn validate_thought(args: SequentialThinkingArgs) -> ThoughtData {
        // Auto-adjust totalThoughts if needed (ensures consistency)
        let total_thoughts = args.total_thoughts.max(args.thought_number);

        ThoughtData {
            thought: args.thought,
            thought_number: args.thought_number,
            total_thoughts,
            next_thought_needed: args.next_thought_needed,
            is_revision: args.is_revision,
            revises_thought: args.revises_thought,
            branch_from_thought: args.branch_from_thought,
            branch_id: args.branch_id,
            needs_more_thoughts: args.needs_more_thoughts,
        }
    }

    /// Format thought for stderr display with ANSI colors
    /// Creates a bordered box with colored prefix based on thought type
    fn format_thought(data: &ThoughtData) -> String {
        let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();

        // Determine the prefix text and color based on thought type
        let (prefix_text, prefix_color, context) = if data.is_revision.unwrap_or(false) {
            let ctx = data
                .revises_thought
                .map(|n| format!(" (revising thought {n})"))
                .unwrap_or_default();
            ("🔄 Revision", Color::Yellow, ctx)
        } else if let Some(branch_from) = data.branch_from_thought {
            let ctx = format!(
                " (from thought {}, ID: {})",
                branch_from,
                data.branch_id.as_deref().unwrap_or("unknown")
            );
            ("🌿 Branch", Color::Green, ctx)
        } else {
            ("💭 Thought", Color::Blue, String::new())
        };

        // Create the header with colored prefix
        let _ = write!(&mut buffer, "\n┌");

        // Calculate border length - we'll build header first to get accurate length
        let header_plain = format!(
            "{prefix_text} {}/{}{context}",
            data.thought_number, data.total_thoughts
        );
        let border_len = header_plain.len().max(data.thought.len()) + 4;
        let border = "─".repeat(border_len);

        let _ = writeln!(&mut buffer, "{border}┐");
        let _ = write!(&mut buffer, "│ ");

        // Write colored prefix
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(prefix_color)));
        let _ = write!(&mut buffer, "{prefix_text}");
        let _ = buffer.reset();

        // Write rest of header
        let _ = writeln!(
            &mut buffer,
            " {}/{}{context} │",
            data.thought_number, data.total_thoughts
        );
        let _ = writeln!(&mut buffer, "├{border}┤");
        let _ = writeln!(&mut buffer, "│ {} │", data.thought);
        let _ = writeln!(&mut buffer, "└{border}┘");

        String::from_utf8_lossy(buffer.as_slice()).to_string()
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for SequentialThinkingTool {
    type Args = SequentialThinkingArgs;
    type PromptArgs = SequentialThinkingPromptArgs;

    fn name() -> &'static str {
        "sequential_thinking"
    }

    fn description() -> &'static str {
        "A detailed tool for dynamic and reflective problem-solving through thoughts.\n\
         This tool helps analyze problems through a flexible thinking process that can adapt and evolve.\n\
         Each thought can build on, question, or revise previous insights as understanding deepens.\n\n\
         When to use this tool:\n\
         - Breaking down complex problems into steps\n\
         - Planning and design with room for revision\n\
         - Analysis that might need course correction\n\
         - Problems where the full scope might not be clear initially\n\
         - Problems that require a multi-step solution\n\
         - Tasks that need to maintain context over multiple steps\n\
         - Situations where irrelevant information needs to be filtered out\n\n\
         Key features:\n\
         - Adjust total_thoughts up or down as you progress\n\
         - Question or revise previous thoughts\n\
         - Add more thoughts even after reaching what seemed like the end\n\
         - Express uncertainty and explore alternative approaches\n\
         - Branch or backtrack (not every thought needs to build linearly)\n\
         - Generate and verify solution hypotheses\n\
         - Repeat the process until satisfied"
    }

    fn read_only() -> bool {
        true // Only tracks internal state, doesn't modify external resources
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate and convert args
        let thought_data = Self::validate_thought(args.clone());

        // Get or create session
        let (session_id, tx) = self.get_or_create_session(args.session_id).await?;

        // Create response channel
        let (respond_to, rx) = tokio::sync::oneshot::channel();

        // Send command to session actor
        let cmd = SessionCommand::AddThought {
            thought: thought_data,
            respond_to,
        };

        tx.send(cmd)
            .await
            .map_err(|_| McpError::Other(anyhow::anyhow!("Session actor terminated")))?;

        // Wait for response
        let response = rx
            .await
            .map_err(|_| McpError::Other(anyhow::anyhow!("Session actor failed to respond")))?;

        // Build JSON response with session ID (snake_case)
        Ok(json!({
            "session_id": session_id,
            "thought_number": response.thought_number,
            "total_thoughts": response.total_thoughts,
            "next_thought_needed": response.next_thought_needed,
            "branches": response.branches,
            "thought_history_length": response.thought_history_length
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![] // No arguments needed for teaching prompt
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use the sequential_thinking tool to solve a complex problem?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The sequential_thinking tool helps you break down complex problems step by step:\n\n\
                     1. Start with initial estimate:\n\
                     sequential_thinking({\n\
                       \"thought\": \"First, I need to understand the problem scope\",\n\
                       \"thought_number\": 1,\n\
                       \"total_thoughts\": 5,\n\
                       \"next_thought_needed\": true\n\
                     })\n\n\
                     2. Continue building:\n\
                     sequential_thinking({\n\
                       \"thought\": \"Now analyzing the core requirements\",\n\
                       \"thought_number\": 2,\n\
                       \"total_thoughts\": 5,\n\
                       \"next_thought_needed\": true\n\
                     })\n\n\
                     3. Revise if needed:\n\
                     sequential_thinking({\n\
                       \"thought\": \"Wait, I need to reconsider my approach\",\n\
                       \"thought_number\": 3,\n\
                       \"total_thoughts\": 6,\n\
                       \"is_revision\": true,\n\
                       \"revises_thought\": 2,\n\
                       \"next_thought_needed\": true\n\
                     })\n\n\
                     4. Branch to explore alternatives:\n\
                     sequential_thinking({\n\
                       \"thought\": \"Alternative approach using pattern X\",\n\
                       \"thought_number\": 4,\n\
                       \"total_thoughts\": 6,\n\
                       \"branch_from_thought\": 2,\n\
                       \"branch_id\": \"alt-pattern-x\",\n\
                       \"next_thought_needed\": true\n\
                     })\n\n\
                     5. Conclude:\n\
                     sequential_thinking({\n\
                       \"thought\": \"Final solution: implement approach Y\",\n\
                       \"thought_number\": 6,\n\
                       \"total_thoughts\": 6,\n\
                       \"next_thought_needed\": false\n\
                     })\n\n\
                     The tool tracks your entire thinking process, allowing you to:\n\
                     - Adjust total_thoughts dynamically as you learn more\n\
                     - Revise earlier thoughts when you discover new information\n\
                     - Branch to explore multiple solution paths\n\
                     - See your complete thought history across all invocations",
                ),
            },
        ])
    }
}

// ============================================================================
// SHUTDOWN HOOK FOR MCP SERVER
// ============================================================================

#[cfg(feature = "server")]
use kodegen_mcp_server_core::ShutdownHook;

#[cfg(feature = "server")]
impl ShutdownHook for SequentialThinkingTool {
    fn shutdown(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), anyhow::Error>> + Send + '_>> {
        Box::pin(async move {
            SequentialThinkingTool::shutdown(self).await
                .map_err(|e| anyhow::anyhow!("Failed to shutdown sequential thinking tool: {e}"))
        })
    }
}
