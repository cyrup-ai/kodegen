//! Session command protocol for agent communication
//!
//! Defines the command messages that can be sent to agent background tasks
//! via channels for non-blocking communication.

use tokio::sync::oneshot;

use crate::error::Result;

/// Commands that can be sent to an agent background task
///
/// This enum defines the command protocol for communicating with agent
/// sessions via channels, eliminating the need for shared locks on the
/// `ClaudeSDKClient`.
pub(super) enum SessionCommand {
    /// Send a follow-up message to the agent
    SendMessage {
        /// The prompt text to send
        prompt: String,
        /// Channel to send the operation result back
        response_tx: oneshot::Sender<Result<()>>,
    },

    /// Shutdown the agent session gracefully
    Shutdown {
        /// Channel to send the shutdown confirmation back
        response_tx: oneshot::Sender<Result<()>>,
    },
}
