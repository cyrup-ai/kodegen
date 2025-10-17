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
    pub fn new() -> Self {
        Self
    }

    /// Encode an SSE event to wire format
    ///
    /// Produces output according to RFC 6455:
    /// - event: <event_type>
    /// - data: <data_line>
    /// - id: <event_id>
    /// - <empty_line>
    ///
    /// Multiline data is properly handled with multiple data: fields.
    /// Unicode content is preserved with proper UTF-8 encoding.
    pub fn encode(&self, event: &SseEvent) -> String {
        let mut output = String::new();

        // Add event type if present
        if let Some(ref event_type) = event.event_type {
            writeln!(output, "event: {}", event_type).expect("String write cannot fail");
        }

        // Add data field(s) - handle multiline data properly
        for line in event.data.lines() {
            writeln!(output, "data: {}", line).expect("String write cannot fail");
        }

        // Add event ID if present
        if let Some(ref id) = event.id {
            writeln!(output, "id: {}", id).expect("String write cannot fail");
        }

        // Add empty line to terminate the event
        output.push('\n');

        output
    }

    /// Encode multiple events to wire format
    #[allow(dead_code)]
    pub fn encode_multiple(&self, events: &[SseEvent]) -> String {
        events.iter().map(|event| self.encode(event)).collect()
    }

    /// Create a comment line (ignored by SSE parsers)
    ///
    /// Comments start with ':' and are used for keep-alive or debugging.
    #[allow(dead_code)]
    pub fn comment(text: &str) -> String {
        format!(": {}\n\n", text)
    }

    /// Create a keep-alive comment
    #[allow(dead_code)]
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


