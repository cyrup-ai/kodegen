//! Message parser for Claude Code SDK responses

use crate::error::{ClaudeError, Result};
use crate::types::messages::Message;

/// Parse a JSON value into a typed Message
///
/// # Arguments
/// * `data` - Raw JSON value from CLI output
///
/// # Returns
/// Parsed Message object or error
///
/// # Errors
/// Returns `ClaudeError::MessageParse` if the JSON cannot be parsed into a valid Message
///
/// # Security Note
/// For additional security against deeply nested JSON attacks, consider using
/// `serde_json::Deserializer::with_recursion_limit()` to set explicit depth limits.
/// This is not currently implemented to avoid additional complexity, but the current
/// timeout mechanism on read operations provides some protection against excessive
/// parsing time.
pub fn parse_message(data: serde_json::Value) -> Result<Message> {
    serde_json::from_value(data.clone()).map_err(|e| {
        ClaudeError::message_parse(format!("Failed to parse message: {e}"), Some(data))
    })
}
