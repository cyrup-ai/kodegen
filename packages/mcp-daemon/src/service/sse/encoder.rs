//! SSE wire format encoder
//!
//! Implements Server-Sent Events encoding according to RFC 6455 specification.
//! Handles proper field formatting, multiline data, and Unicode encoding.

use std::fmt::Write;

use super::events::SseEvent;

/// SSE encoder for converting events to wire format
///
/// Implements the Server-Sent Events protocol as specified in RFC 6455.
/// Handles proper field formatting, Unicode encoding, and multiline data.
#[derive(Debug, Default, Clone)]
pub struct SseEncoder;

impl SseEncoder {
    /// Create a new SSE encoder
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Encode an SSE event to wire format
    ///
    /// Produces output according to RFC 6455:
    /// - event: <`event_type`>
    /// - data: <`data_line`>
    /// - id: <`event_id`>
    /// - <`empty_line`>
    ///
    /// Multiline data is properly handled with multiple data: fields.
    /// Unicode content is preserved with proper UTF-8 encoding.
    #[must_use]
    pub fn encode(&self, event: &SseEvent) -> String {
        let mut output = String::new();

        // Add event type if present
        if let Some(ref event_type) = event.event_type {
            // Writing to String is infallible - the write! family always succeeds for String
            let _ = writeln!(output, "event: {event_type}");
        }

        // Add data field(s) - handle multiline data properly
        for line in event.data.lines() {
            // Writing to String is infallible - the write! family always succeeds for String
            let _ = writeln!(output, "data: {line}");
        }

        // Add event ID if present
        if let Some(ref id) = event.id {
            // Writing to String is infallible - the write! family always succeeds for String
            let _ = writeln!(output, "id: {id}");
        }

        // Add empty line to terminate the event
        output.push('\n');

        output
    }

    /// Encode multiple events to wire format
    #[allow(dead_code)]
    #[must_use]
    pub fn encode_multiple(&self, events: &[SseEvent]) -> String {
        events.iter().map(|event| self.encode(event)).collect()
    }

    /// Create a comment line (ignored by SSE parsers)
    ///
    /// Comments start with ':' and are used for keep-alive or debugging.
    #[allow(dead_code)]
    #[must_use]
    pub fn comment(text: &str) -> String {
        format!(": {text}\n\n")
    }

    /// Create a keep-alive comment
    #[allow(dead_code)]
    #[must_use]
    pub fn keep_alive() -> String {
        Self::comment("keep-alive")
    }
}

/// Helper function to escape data for SSE format
///
/// While SSE doesn't require extensive escaping like XML/HTML,
/// we ensure proper line handling and Unicode preservation.
#[allow(dead_code)]
fn escape_sse_data(data: &str) -> String {
    // SSE data doesn't need escaping except for proper line handling
    // Unicode is preserved as-is since SSE is UTF-8
    data.to_string()
}
