//! Session state structures
//!
//! Defines the data structures for tracking active and completed agent sessions.

use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, mpsc};

use super::commands::SessionCommand;
use crate::types::agent::SerializedMessage;

/// Active session data (stored while client is running)
///
/// Holds all state for an active agent session including the command channel
/// for sending messages, circular message buffer, and tracking information.
#[derive(Clone)]
pub(super) struct AgentSessionInfo {
    /// Unique session identifier
    pub session_id: String,

    /// Human-readable label for the session
    pub label: String,

    /// Channel for sending commands to the background task
    pub command_tx: mpsc::UnboundedSender<SessionCommand>,

    /// Circular buffer of messages (FIFO with capacity limit)
    pub messages: Arc<Mutex<VecDeque<SerializedMessage>>>,

    /// When the session was created
    pub created_at: Instant,

    /// Last time a message was received
    pub last_message_at: Arc<Mutex<Instant>>,

    /// Current turn count
    pub turn_count: Arc<Mutex<u32>>,

    /// Maximum turns allowed
    pub max_turns: u32,

    /// Whether the session has completed
    pub is_complete: Arc<Mutex<bool>>,
}

/// Completed session data (retained for final reads before cleanup)
///
/// Once a session completes, it's moved from active to completed state
/// with a snapshot of its final state. These are kept for a retention
/// period before being automatically cleaned up.
pub(super) struct CompletedAgentSession {
    /// Unique session identifier
    pub session_id: String,

    /// Human-readable label for the session
    pub label: String,

    /// Final message buffer snapshot
    pub messages: VecDeque<SerializedMessage>,

    /// Final turn count when completed
    pub final_turn_count: u32,

    /// Total runtime in milliseconds
    pub runtime_ms: u64,

    /// When the session completed (wall-clock time)
    pub completed_at: DateTime<Utc>,
}
