//! Helper functions for message processing
//!
//! Pure functions for converting and extracting data from messages.

use chrono::Utc;
use std::collections::VecDeque;

use crate::types::agent::SerializedMessage;
use crate::types::messages::{ContentBlock, Message};

/// Convert a Message enum to `SerializedMessage` for storage
///
/// Serializes the entire message to JSON and extracts metadata like
/// message type and turn number for efficient querying.
///
/// # Arguments
/// * `msg` - The message to serialize
///
/// # Returns
/// A `SerializedMessage` with type, content, turn, and timestamp
pub(super) fn serialize_message(msg: &Message) -> SerializedMessage {
    let (message_type, turn) = match msg {
        Message::User { .. } => ("user".to_string(), 0),
        Message::Assistant { .. } => ("assistant".to_string(), 0),
        Message::System { subtype, .. } => (format!("system_{subtype}"), 0),
        Message::Result { num_turns, .. } => ("result".to_string(), *num_turns),
        Message::StreamEvent { .. } => ("stream_event".to_string(), 0),
    };

    // Serialize the entire message to JSON, falling back to Null on error
    let content = serde_json::to_value(msg).unwrap_or(serde_json::Value::Null);

    SerializedMessage {
        message_type,
        content,
        turn,
        timestamp: Utc::now(),
    }
}

/// Extract last N lines of text from assistant messages
///
/// Scans messages in reverse chronological order, filters for assistant
/// messages, and extracts text from Text and Thinking content blocks.
///
/// # Arguments
/// * `messages` - The message buffer to scan
/// * `n` - Maximum number of text lines to extract
///
/// # Returns
/// Vec of text strings from most recent assistant messages (up to N lines)
pub(super) fn extract_last_output_lines(
    messages: &VecDeque<SerializedMessage>,
    n: usize,
) -> Vec<String> {
    messages
        .iter()
        .rev() // Start from most recent
        .filter(|msg| msg.message_type == "assistant")
        .flat_map(|msg| {
            // Try to deserialize as Message and extract text from content blocks
            if let Ok(Message::Assistant { message, .. }) =
                serde_json::from_value::<Message>(msg.content.clone())
            {
                message
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } => Some(text.clone()),
                        ContentBlock::Thinking { thinking, .. } => Some(thinking.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![]
            }
        })
        .take(n)
        .collect()
}
