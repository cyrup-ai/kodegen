//! SSE session management
//!
//! Handles session lifecycle, tracking, and cleanup for SSE connections.
//! Provides thread-safe session storage and automatic timeout management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tokio::time::interval;
use log::{debug, info, warn};
use uuid::Uuid;

/// Information about an active SSE session
#[derive(Debug, Clone)]
pub struct SseSession {
    /// Unique session identifier
    pub id: String,
    /// When the session was created
    pub created_at: Instant,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Client connection information
    pub client_info: ClientInfo,
}

/// Client connection information
#[derive(Debug, Clone)]
pub struct ClientInfo {
    /// Client IP address
    pub remote_addr: String,
    /// User-Agent header if present
    #[allow(dead_code)] // Used for client tracking and debugging
    pub user_agent: Option<String>,
    /// Client-provided connection info
    #[allow(dead_code)] // Used for client tracking and debugging
    pub connection_id: Option<String>,
}

impl SseSession {
    /// Create a new session with given client info
    pub fn new(client_info: ClientInfo) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            last_activity: now,
            client_info,
        }
    }

    /// Update the last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if the session has timed out
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    /// Get session age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Thread-safe session manager
///
/// Manages active SSE sessions with automatic cleanup and timeout handling.
/// Provides atomic operations for session creation, lookup, and removal.
#[derive(Debug)]
pub struct SessionManager {
    /// Active sessions storage
    sessions: Arc<RwLock<HashMap<String, SseSession>>>,
    /// Maximum number of concurrent sessions
    max_sessions: usize,
    /// Session timeout duration
    timeout: Duration,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(max_sessions: usize, timeout: Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions,
            timeout,
        }
    }

    /// Create a new session
    ///
    /// Returns None if maximum session limit is reached.
    pub async fn create_session(&self, client_info: ClientInfo) -> Option<SseSession> {
        let mut sessions = self.sessions.write().await;

        // Check session limit
        if sessions.len() >= self.max_sessions {
            warn!(
                "Session limit reached ({}/{}), rejecting new session from {}",
                sessions.len(),
                self.max_sessions,
                client_info.remote_addr
            );
            return None;
        }

        let session = SseSession::new(client_info);
        let session_id = session.id.clone();

        sessions.insert(session_id.clone(), session.clone());

        info!(
            "Created new SSE session {} from {} (total: {})",
            session_id,
            session.client_info.remote_addr,
            sessions.len()
        );

        Some(session)
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<SseSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Touch a session to update its last activity
    pub async fn touch_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.touch();
            debug!("Touched session {}", session_id);
            true
        } else {
            false
        }
    }

    /// Remove a session
    #[allow(dead_code)] // Session management API
    pub async fn remove_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            info!(
                "Removed SSE session {} from {} (total: {})",
                session_id,
                session.client_info.remote_addr,
                sessions.len()
            );
            true
        } else {
            false
        }
    }

    /// Get count of active sessions
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Get all session IDs
    #[allow(dead_code)] // Session management API
    pub async fn session_ids(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Clean up expired sessions
    ///
    /// Returns the number of sessions that were cleaned up.
    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();

        // Collect expired session IDs
        let expired_ids: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.is_expired(self.timeout))
            .map(|(id, _)| id.clone())
            .collect();

        // Remove expired sessions
        for session_id in &expired_ids {
            if let Some(session) = sessions.remove(session_id) {
                info!(
                    "Cleaned up expired session {} from {} (age: {:?})",
                    session_id,
                    session.client_info.remote_addr,
                    session.age()
                );
            }
        }

        let cleaned_count = expired_ids.len();
        if cleaned_count > 0 {
            info!(
                "Cleaned up {} expired sessions ({} -> {})",
                cleaned_count,
                initial_count,
                sessions.len()
            );
        }

        cleaned_count
    }

    /// Start a background cleanup task
    ///
    /// Spawns a background task that periodically cleans up expired sessions.
    pub fn start_cleanup_task(&self, cleanup_interval: Duration) -> tokio::task::JoinHandle<()> {
        let sessions = self.sessions.clone();
        let timeout = self.timeout;

        tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);

            loop {
                interval.tick().await;

                // Create a temporary session manager for cleanup
                let temp_manager = SessionManager {
                    sessions: sessions.clone(),
                    max_sessions: 0, // Not used in cleanup
                    timeout,
                };

                let cleaned = temp_manager.cleanup_expired().await;
                if cleaned > 0 {
                    debug!("Background cleanup removed {} expired sessions", cleaned);
                }
            }
        })
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(100, Duration::from_secs(300)) // 100 sessions, 5 minute timeout
    }
}


