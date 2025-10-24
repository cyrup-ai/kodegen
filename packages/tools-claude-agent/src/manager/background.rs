//! Background task spawning for agent sessions
//!
//! Contains functions for spawning background tasks that handle message
//! collection and command processing for agent sessions.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, mpsc};

use super::commands::SessionCommand;
use super::helpers::serialize_message;
use crate::client::ClaudeSDKClient;
use crate::types::agent::SerializedMessage;
use crate::types::messages::Message;

/// Circular buffer capacity for messages
const BUFFER_SIZE: usize = 1000;

/// Shared state for message collector task
pub(super) struct CollectorContext {
    pub messages: Arc<Mutex<VecDeque<SerializedMessage>>>,
    pub last_message: Arc<Mutex<Instant>>,
    pub turn_count: Arc<Mutex<u32>>,
    pub is_complete: Arc<Mutex<bool>>,
    pub max_turns: u32,
    pub session_id: String,
}

/// Spawn a background task to collect messages from an agent session
///
/// This task owns the `ClaudeSDKClient` and handles:
/// - Processing incoming messages from the Claude API
/// - Handling `SendMessage` and Shutdown commands via channel
/// - Maintaining a circular buffer of messages
/// - Updating session state (timestamps, turn count, completion status)
///
/// The task runs until the session completes (receives Result message)
/// or encounters an error.
///
/// # Arguments
/// * `client` - The `ClaudeSDKClient` instance (task takes ownership)
/// * `command_rx` - Channel receiver for session commands
/// * `ctx` - Collector context containing shared state
pub(super) fn spawn_message_collector(
    mut client: ClaudeSDKClient,
    mut command_rx: mpsc::UnboundedReceiver<SessionCommand>,
    ctx: CollectorContext,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle commands from other tasks
                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        SessionCommand::SendMessage { prompt, response_tx } => {
                            let result = client.send_message(&prompt).await;
                            if result.is_ok() {
                                *ctx.last_message.lock().await = Instant::now();
                            }
                            let _ = response_tx.send(result);
                        }
                        SessionCommand::Shutdown { response_tx } => {
                            let result = client.close().await;
                            let _ = response_tx.send(result);
                            break;
                        }
                    }
                }
                // Process incoming messages
                Some(msg_result) = client.next_message() => {
                    match msg_result {
                        Ok(msg) => {
                            // Convert Message to SerializedMessage
                            let serialized = serialize_message(&msg);

                            // Push to circular buffer
                            {
                                let mut messages = ctx.messages.lock().await;
                                if messages.len() == BUFFER_SIZE {
                                    messages.pop_front();  // Remove oldest
                                }
                                messages.push_back(serialized);
                            }

                            // Update timestamp
                            *ctx.last_message.lock().await = Instant::now();

                            // Check for completion
                            if let Message::Result { num_turns, .. } = msg {
                                *ctx.turn_count.lock().await = num_turns;

                                // Only mark complete if we've reached ctx.max_turns
                                if num_turns >= ctx.max_turns {
                                    *ctx.is_complete.lock().await = true;
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("[{}] Message error: {}", ctx.session_id, e);
                            *ctx.is_complete.lock().await = true;
                            break;
                        }
                    }
                }
            }
        }
    });
}
