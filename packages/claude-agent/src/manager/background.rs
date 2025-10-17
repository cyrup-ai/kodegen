//! Background task spawning for agent sessions
//!
//! Contains functions for spawning background tasks that handle message
//! collection and command processing for agent sessions.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, mpsc};

use crate::client::ClaudeSDKClient;
use crate::types::agent::SerializedMessage;
use crate::types::messages::Message;
use super::commands::SessionCommand;
use super::helpers::serialize_message;

/// Circular buffer capacity for messages
const BUFFER_SIZE: usize = 1000;

/// Spawn a background task to collect messages from an agent session
///
/// This task owns the ClaudeSDKClient and handles:
/// - Processing incoming messages from the Claude API
/// - Handling SendMessage and Shutdown commands via channel
/// - Maintaining a circular buffer of messages
/// - Updating session state (timestamps, turn count, completion status)
///
/// The task runs until the session completes (receives Result message)
/// or encounters an error.
///
/// # Arguments
/// * `client` - The ClaudeSDKClient instance (task takes ownership)
/// * `command_rx` - Channel receiver for session commands
/// * `messages_arc` - Shared message buffer
/// * `last_message_arc` - Shared timestamp of last message
/// * `turn_count_arc` - Shared turn counter
/// * `is_complete_arc` - Shared completion flag
/// * `session_id` - Session identifier for logging
pub(super) fn spawn_message_collector(
    mut client: ClaudeSDKClient,
    mut command_rx: mpsc::UnboundedReceiver<SessionCommand>,
    messages_arc: Arc<Mutex<VecDeque<SerializedMessage>>>,
    last_message_arc: Arc<Mutex<Instant>>,
    turn_count_arc: Arc<Mutex<u32>>,
    is_complete_arc: Arc<Mutex<bool>>,
    session_id: String,
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
                                *last_message_arc.lock().await = Instant::now();
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
                                let mut messages = messages_arc.lock().await;
                                if messages.len() == BUFFER_SIZE {
                                    messages.pop_front();  // Remove oldest
                                }
                                messages.push_back(serialized);
                            }
                            
                            // Update timestamp
                            *last_message_arc.lock().await = Instant::now();
                            
                            // Check for completion
                            if let Message::Result { num_turns, .. } = msg {
                                *turn_count_arc.lock().await = num_turns;
                                *is_complete_arc.lock().await = true;
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("[{}] Message error: {}", session_id, e);
                            *is_complete_arc.lock().await = true;
                            break;
                        }
                    }
                }
            }
        }
    });
}
