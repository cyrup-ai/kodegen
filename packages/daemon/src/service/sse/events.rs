//! SSE event types and structures
//!
//! Defines the event types and data structures used in the SSE transport
//! according to the MCP SSE specification and RFC 6455.

use std::fmt;

use serde::{Deserialize, Serialize};

/// SSE event types as defined in the MCP SSE specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    /// Sent when SSE connection is established, contains the messages endpoint URL
    Endpoint,
    /// Contains a JSON-RPC message
    Message,
    /// Keep-alive ping event
    Ping,
    /// Error event for protocol or server errors
    Error,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::Endpoint => write!(f, "endpoint"),
            EventType::Message => write!(f, "message"),
            EventType::Ping => write!(f, "ping"),
            EventType::Error => write!(f, "error"),
        }
    }
}

/// Server-Sent Event structure
///
/// Represents a single SSE event with optional event type, data payload,
/// and event ID for resumability support.
#[derive(Debug, Clone, PartialEq)]
pub struct SseEvent {
    /// Optional event type (maps to SSE "event:" field)
    pub event_type: Option<EventType>,
    /// Event data payload (maps to SSE "data:" field)
    pub data: String,
    /// Optional event ID for resumability (maps to SSE "id:" field)
    pub id: Option<String>,
}

impl SseEvent {
    /// Create a new SSE event with given type and data
    pub fn new(event_type: EventType, data: impl Into<String>) -> Self {
        Self {
            event_type: Some(event_type),
            data: data.into(),
            id: None,
        }
    }

    /// Create a new SSE event with data only (no event type)
    #[allow(dead_code)]
    pub fn data_only(data: impl Into<String>) -> Self {
        Self {
            event_type: None,
            data: data.into(),
            id: None,
        }
    }

    /// Create an endpoint event with the messages URL for a session
    pub fn endpoint(session_id: &str, base_url: &str) -> Self {
        let messages_url = format!("{}/messages?session_id={}", base_url, session_id);
        Self::new(EventType::Endpoint, messages_url)
    }

    /// Create a message event with JSON-RPC payload
    #[allow(dead_code)]
    pub fn message(json_rpc: impl Into<String>) -> Self {
        Self::new(EventType::Message, json_rpc)
    }

    /// Create a ping event with timestamp
    pub fn ping(timestamp: impl Into<String>) -> Self {
        Self::new(EventType::Ping, timestamp)
    }

    /// Create an error event with error message
    #[allow(dead_code)]
    pub fn error(error_msg: impl Into<String>) -> Self {
        Self::new(EventType::Error, error_msg)
    }

    /// Set the event ID for resumability
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Check if this is a ping event
    #[allow(dead_code)]
    pub fn is_ping(&self) -> bool {
        matches!(self.event_type, Some(EventType::Ping))
    }

    /// Check if this is an error event
    #[allow(dead_code)]
    pub fn is_error(&self) -> bool {
        matches!(self.event_type, Some(EventType::Error))
    }
}

