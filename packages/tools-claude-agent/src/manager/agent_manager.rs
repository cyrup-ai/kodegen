//! Agent session manager for spawning and monitoring multiple Claude agent sessions
//!
//! Provides concurrent session management with circular message buffering, working status
//! detection, and automatic cleanup of completed sessions.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, mpsc, oneshot};
use chrono::Utc;
use uuid::Uuid;

use crate::client::ClaudeSDKClient;
use crate::error::{ClaudeError, Result};
use crate::types::agent::{
    AgentInfo, GetOutputResponse, ListSessionsResponse, TerminateResponse,
};
use crate::types::identifiers::ToolName;
use crate::types::options::ClaudeAgentOptions;
use crate::types::agent::SystemPrompt;

use super::commands::SessionCommand;
use super::session::{AgentSessionInfo, CompletedAgentSession};
use super::background::{spawn_message_collector, CollectorContext};
use super::helpers::extract_last_output_lines;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Time threshold for considering an agent "working" (2 seconds)
const WORKING_THRESHOLD_MS: u64 = 2000;

/// Retention time for completed sessions before cleanup (1 minute)
const COMPLETED_RETENTION_MS: u64 = 60000;

/// Interval for cleanup task execution (1 minute)
const CLEANUP_INTERVAL_SECS: u64 = 60;

// ============================================================================
// REQUEST TYPES
// ============================================================================

/// Request parameters for spawning a new agent session
#[derive(Debug, Clone)]
pub struct SpawnSessionRequest {
    /// Initial prompt to send to the agent
    pub prompt: String,
    /// System prompt configuration
    pub system_prompt: Option<String>,
    /// List of tools that Claude is allowed to use
    pub allowed_tools: Vec<String>,
    /// List of tools that Claude is not allowed to use
    pub disallowed_tools: Vec<String>,
    /// Maximum number of turns before stopping
    pub max_turns: u32,
    /// AI model to use
    pub model: Option<String>,
    /// Working directory for the CLI process
    pub cwd: Option<String>,
    /// Additional directories to add to the context
    pub add_dirs: Vec<String>,
    /// Label for identifying the session
    pub label: String,
}

// ============================================================================
// AGENT MANAGER
// ============================================================================

/// Manager for multiple concurrent Claude agent sessions
///
/// The `AgentManager` coordinates multiple Claude agent sessions, handling:
/// - Session lifecycle (spawn, monitor, terminate)
/// - Message buffering with circular buffers
/// - Working status detection
/// - Automatic cleanup of completed sessions
pub struct AgentManager {
    active_sessions: Arc<Mutex<HashMap<String, AgentSessionInfo>>>,
    completed_sessions: Arc<Mutex<HashMap<String, CompletedAgentSession>>>,
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

impl AgentManager {
    /// Create a new `AgentManager` with background cleanup task
    #[must_use] 
    pub fn new() -> Self {
        let active: Arc<Mutex<HashMap<String, AgentSessionInfo>>> = Arc::new(Mutex::new(HashMap::new()));
        let completed: Arc<Mutex<HashMap<String, CompletedAgentSession>>> = Arc::new(Mutex::new(HashMap::new()));
        
        // Spawn cleanup background task
        let completed_clone = Arc::clone(&completed);
        let cleanup_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(CLEANUP_INTERVAL_SECS)).await;
                
                let mut sessions = completed_clone.lock().await;
                let now = Utc::now();
                
                // Remove sessions older than retention period
                sessions.retain(|_id, session| {
                    let age_ms = now.signed_duration_since(session.completed_at).num_milliseconds() as u64;
                    age_ms < COMPLETED_RETENTION_MS
                });
            }
        });
        
        Self {
            active_sessions: active,
            completed_sessions: completed,
            cleanup_handle: Some(cleanup_handle),
        }
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AgentManager {
    fn drop(&mut self) {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}

impl AgentManager {
    /// Spawn a new Claude agent session
    ///
    /// Creates a new `ClaudeSDKClient` with the specified options, sends the initial prompt,
    /// and spawns a background task to collect messages into a circular buffer.
    ///
    /// Returns the session ID for subsequent operations.
    pub async fn spawn_session(&self, request: SpawnSessionRequest) -> Result<String> {
        // Generate unique session ID
        let session_id = Uuid::new_v4().to_string();

        // Build ClaudeAgentOptions
        let options = ClaudeAgentOptions {
            allowed_tools: request.allowed_tools.into_iter().map(ToolName::from).collect(),
            disallowed_tools: request.disallowed_tools.into_iter().map(ToolName::from).collect(),
            system_prompt: request.system_prompt.map(SystemPrompt::from),
            max_turns: Some(request.max_turns),
            model: request.model,
            cwd: request.cwd.map(PathBuf::from),
            add_dirs: request.add_dirs.into_iter().map(PathBuf::from).collect(),
            ..Default::default()
        };

        // Create client
        let mut client = ClaudeSDKClient::new(options, None).await?;

        // Send initial prompt
        client.send_message(&request.prompt).await?;
        
        // Create command channel
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        
        // Create shared state for background task
        let messages_arc = Arc::new(Mutex::new(VecDeque::with_capacity(1000)));
        let last_message_arc = Arc::new(Mutex::new(Instant::now()));
        let turn_count_arc = Arc::new(Mutex::new(0));
        let is_complete_arc = Arc::new(Mutex::new(false));
        
        // Create session info
        let session_info = AgentSessionInfo {
            session_id: session_id.clone(),
            label: request.label,
            command_tx: command_tx.clone(),
            messages: Arc::clone(&messages_arc),
            created_at: Instant::now(),
            last_message_at: Arc::clone(&last_message_arc),
            turn_count: Arc::clone(&turn_count_arc),
            max_turns: request.max_turns,
            is_complete: Arc::clone(&is_complete_arc),
        };
        
        // Store in active sessions
        self.active_sessions.lock().await.insert(session_id.clone(), session_info);
        
        // Spawn background message collector
        let ctx = CollectorContext {
            messages: messages_arc,
            last_message: last_message_arc,
            turn_count: turn_count_arc,
            is_complete: is_complete_arc,
            max_turns: request.max_turns,
            session_id: session_id.clone(),
        };
        spawn_message_collector(client, command_rx, ctx);
        
        Ok(session_id)
    }

    /// Get information about a specific agent session
    ///
    /// Returns session details including working status, turn count, runtime, and
    /// a preview of recent output. Checks active sessions first, then completed sessions.
    pub async fn get_session_info(&self, session_id: &str) -> Result<AgentInfo> {
        // Check active sessions first
        let active = self.active_sessions.lock().await;
        if let Some(session) = active.get(session_id) {
            let runtime_ms = session.created_at.elapsed().as_millis() as u64;
            let turn_count = *session.turn_count.lock().await;
            let is_complete = *session.is_complete.lock().await;
            let messages = session.messages.lock().await;
            let message_count = messages.len();
            let last_output = extract_last_output_lines(&messages, 3);
            
            // Calculate working status
            let working = if is_complete {
                false
            } else {
                let last_msg_time = *session.last_message_at.lock().await;
                let elapsed_ms = last_msg_time.elapsed().as_millis() as u64;
                elapsed_ms < WORKING_THRESHOLD_MS
            };
            
            return Ok(AgentInfo {
                session_id: session.session_id.clone(),
                label: session.label.clone(),
                working,
                turn_count,
                max_turns: session.max_turns,
                runtime_ms,
                message_count,
                is_complete,
                last_output,
                completion_time: None,
            });
        }
        drop(active);
        
        // Check completed sessions
        let completed = self.completed_sessions.lock().await;
        if let Some(session) = completed.get(session_id) {
            let last_output = extract_last_output_lines(&session.messages, 3);
            
            return Ok(AgentInfo {
                session_id: session.session_id.clone(),
                label: session.label.clone(),
                working: false,
                turn_count: session.final_turn_count,
                max_turns: 0,
                runtime_ms: session.runtime_ms,
                message_count: session.messages.len(),
                is_complete: true,
                last_output,
                completion_time: Some(session.completed_at),
            });
        }
        
        Err(ClaudeError::SessionNotFound(session_id.to_string()))
    }

    /// Get paginated output from an agent session
    ///
    /// Supports offset/length pagination:
    /// - offset >= 0: Start from line N, take `length` messages
    /// - offset < 0: Tail mode - take last |offset| messages
    ///
    /// Returns messages, working status, and pagination metadata.
    pub async fn get_output(
        &self,
        session_id: &str,
        offset: i64,
        length: usize,
    ) -> Result<GetOutputResponse> {
        // Try active sessions first
        let active = self.active_sessions.lock().await;
        if let Some(session) = active.get(session_id) {
            let messages = session.messages.lock().await;
            let turn_count = *session.turn_count.lock().await;
            let is_complete = *session.is_complete.lock().await;
            let max_turns = session.max_turns;
            
            // Calculate working status
            let working = if is_complete {
                false
            } else {
                let last_msg_time = *session.last_message_at.lock().await;
                let elapsed_ms = last_msg_time.elapsed().as_millis() as u64;
                elapsed_ms < WORKING_THRESHOLD_MS
            };
            
            // Handle pagination
            let output = paginate_messages(&messages, offset, length);
            let total_messages = messages.len();
            let messages_returned = output.len();
            let has_more = calculate_has_more(offset, messages_returned, total_messages);
            
            return Ok(GetOutputResponse {
                session_id: session_id.to_string(),
                working,
                output,
                total_messages,
                messages_returned,
                is_complete,
                turn_count,
                max_turns,
                has_more,
            });
        }
        drop(active);
        
        // Try completed sessions
        let completed = self.completed_sessions.lock().await;
        if let Some(session) = completed.get(session_id) {
            let output = paginate_messages(&session.messages, offset, length);
            let total_messages = session.messages.len();
            let messages_returned = output.len();
            let has_more = calculate_has_more(offset, messages_returned, total_messages);
            
            return Ok(GetOutputResponse {
                session_id: session_id.to_string(),
                working: false,
                output,
                total_messages,
                messages_returned,
                is_complete: true,
                turn_count: session.final_turn_count,
                max_turns: 0,
                has_more,
            });
        }
        
        Err(ClaudeError::SessionNotFound(session_id.to_string()))
    }

    /// Check if an agent session is actively working
    ///
    /// Returns true if the agent has received a message within the working threshold
    /// and the session is not complete.
    pub async fn is_working(&self, session_id: &str) -> Result<bool> {
        let active = self.active_sessions.lock().await;
        if let Some(session) = active.get(session_id) {
            if *session.is_complete.lock().await {
                return Ok(false);
            }
            
            let last_msg_time = *session.last_message_at.lock().await;
            let elapsed_ms = last_msg_time.elapsed().as_millis() as u64;
            
            Ok(elapsed_ms < WORKING_THRESHOLD_MS)
        } else {
            Ok(false)
        }
    }

    /// Send a follow-up message to an active agent session
    ///
    /// Only works for active, non-completed sessions that haven't reached `max_turns`.
    pub async fn send_message(&self, session_id: &str, prompt: &str) -> Result<()> {
        let active = self.active_sessions.lock().await;
        let session = active.get(session_id)
            .ok_or_else(|| ClaudeError::SessionNotFound(session_id.to_string()))?;
        
        if *session.is_complete.lock().await {
            return Err(ClaudeError::SessionComplete(session_id.to_string()));
        }
        
        if *session.turn_count.lock().await >= session.max_turns {
            return Err(ClaudeError::SessionComplete(session_id.to_string()));
        }
        
        let (response_tx, response_rx) = oneshot::channel();
        let cmd = SessionCommand::SendMessage {
            prompt: prompt.to_string(),
            response_tx,
        };
        
        session.command_tx.send(cmd)
            .map_err(|_| ClaudeError::SessionComplete(session_id.to_string()))?;
        
        response_rx.await
            .map_err(|_| ClaudeError::SessionComplete(session_id.to_string()))??;
        
        Ok(())
    }

    /// Terminate an agent session gracefully
    ///
    /// Closes the client connection, moves the session to completed state, and returns
    /// final statistics. The session will be retained for `COMPLETED_RETENTION_MS` before cleanup.
    pub async fn terminate_session(&self, session_id: &str) -> Result<TerminateResponse> {
        let mut active = self.active_sessions.lock().await;
        let session = active.remove(session_id)
            .ok_or_else(|| ClaudeError::SessionNotFound(session_id.to_string()))?;
        drop(active);
        
        let (response_tx, response_rx) = oneshot::channel();
        let cmd = SessionCommand::Shutdown { response_tx };
        
        if session.command_tx.send(cmd).is_ok() {
            let _ = response_rx.await;
        }
        
        let runtime_ms = session.created_at.elapsed().as_millis() as u64;
        let messages = session.messages.lock().await;
        let total_messages = messages.len();
        let final_turn_count = *session.turn_count.lock().await;
        
        let completed = CompletedAgentSession {
            session_id: session.session_id.clone(),
            label: session.label.clone(),
            messages: messages.clone(),
            final_turn_count,
            runtime_ms,
            completed_at: Utc::now(),
        };
        
        self.completed_sessions.lock().await
            .insert(session_id.to_string(), completed);
        
        Ok(TerminateResponse {
            session_id: session_id.to_string(),
            success: true,
            final_turn_count,
            total_messages,
            runtime_ms,
        })
    }

    /// List all agent sessions
    ///
    /// Returns information about active and (optionally) completed sessions, sorted by
    /// working status (working first) and then by runtime (most recent first).
    pub async fn list_sessions(
        &self,
        include_completed: bool,
        last_output_lines: usize,
    ) -> Result<ListSessionsResponse> {
        let mut agents = Vec::new();
        
        // Collect active sessions
        let active = self.active_sessions.lock().await;
        for (_, session) in active.iter() {
            let runtime_ms = session.created_at.elapsed().as_millis() as u64;
            let turn_count = *session.turn_count.lock().await;
            let is_complete = *session.is_complete.lock().await;
            let messages = session.messages.lock().await;
            let message_count = messages.len();
            let last_output = extract_last_output_lines(&messages, last_output_lines);
            
            let working = if is_complete {
                false
            } else {
                let last_msg_time = *session.last_message_at.lock().await;
                let elapsed_ms = last_msg_time.elapsed().as_millis() as u64;
                elapsed_ms < WORKING_THRESHOLD_MS
            };
            
            agents.push(AgentInfo {
                session_id: session.session_id.clone(),
                label: session.label.clone(),
                working,
                turn_count,
                max_turns: session.max_turns,
                runtime_ms,
                message_count,
                is_complete,
                last_output,
                completion_time: None,
            });
        }
        
        let total_active = agents.len();
        drop(active);
        
        // Collect completed sessions if requested
        let mut total_completed = 0;
        if include_completed {
            let completed = self.completed_sessions.lock().await;
            total_completed = completed.len();
            
            for (_, session) in completed.iter() {
                let last_output = extract_last_output_lines(&session.messages, last_output_lines);
                
                agents.push(AgentInfo {
                    session_id: session.session_id.clone(),
                    label: session.label.clone(),
                    working: false,
                    turn_count: session.final_turn_count,
                    max_turns: 0,
                    runtime_ms: session.runtime_ms,
                    message_count: session.messages.len(),
                    is_complete: true,
                    last_output,
                    completion_time: Some(session.completed_at),
                });
            }
        }
        
        agents.sort_by(|a, b| {
            match (a.working, b.working) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.runtime_ms.cmp(&a.runtime_ms),
            }
        });
        
        Ok(ListSessionsResponse {
            agents,
            total_active,
            total_completed,
        })
    }
}

// ============================================================================
// PRIVATE HELPER FUNCTIONS
// ============================================================================

/// Paginate messages based on offset and length
fn paginate_messages(
    messages: &VecDeque<crate::types::agent::SerializedMessage>,
    offset: i64,
    length: usize,
) -> Vec<crate::types::agent::SerializedMessage> {
    if offset >= 0 {
        let start = offset as usize;
        messages.iter()
            .skip(start)
            .take(length)
            .cloned()
            .collect()
    } else {
        let tail_count = (-offset) as usize;
        messages.iter()
            .rev()
            .take(tail_count)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

/// Calculate if there are more messages available for pagination
fn calculate_has_more(offset: i64, messages_returned: usize, total_messages: usize) -> bool {
    if offset >= 0 {
        (offset as usize + messages_returned) < total_messages
    } else {
        false
    }
}
