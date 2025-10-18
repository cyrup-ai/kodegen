//! Control protocol implementation for bidirectional communication
//!
//! This module provides the protocol handler and message types for the control
//! protocol used in bidirectional communication with Claude Code CLI.
//!
//! # Overview
//!
//! The control protocol enables:
//! - Request/response communication
//! - Hook invocations from CLI to SDK
//! - Permission requests from CLI to SDK
//! - Protocol initialization and capability negotiation
//!
//! # Example: Basic Protocol Usage
//!
//! ```rust
//! use kodegen_claude_agent::control::ProtocolHandler;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let handler = ProtocolHandler::new();
//!
//! // Create an initialization request
//! let init_req = handler.create_init_request();
//! assert_eq!(init_req.protocol_version, "1.0");
//!
//! // After receiving init response, mark as initialized
//! handler.set_initialized(true);
//!
//! // Create control requests
//! let interrupt_req = handler.create_interrupt_request();
//! let msg_req = handler.create_send_message_request("Hello!".to_string());
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Handling Hook Events
//!
//! ```rust
//! use kodegen_claude_agent::control::ProtocolHandler;
//! use tokio::sync::mpsc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut handler = ProtocolHandler::new();
//!
//! // Set up hook channel
//! let (hook_tx, mut hook_rx) = mpsc::unbounded_channel();
//! handler.set_hook_channel(hook_tx);
//!
//! // When a hook event arrives, it will be sent to hook_rx
//! // You can then process it and send a response
//! tokio::spawn(async move {
//!     while let Some((hook_id, event, event_data)) = hook_rx.recv().await {
//!         log::info!("Received hook: {} {:?} with data: {:?}", hook_id, event, event_data);
//!         // Process hook and create response...
//!     }
//! });
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Serialization
//!
//! ```rust
//! use kodegen_claude_agent::control::{ControlMessage, ProtocolHandler};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let handler = ProtocolHandler::new();
//! let request = handler.create_interrupt_request();
//! let message = ControlMessage::Request(request);
//!
//! // Serialize to JSON
//! let json = handler.serialize_message(&message)?;
//! assert!(json.ends_with('\n'));
//!
//! // Deserialize from JSON
//! let parsed = handler.deserialize_message(json.trim())?;
//! # Ok(())
//! # }
//! ```

mod capabilities;
mod handler;
mod messages;

// Re-export public types
pub use capabilities::{ClientCapabilities, ServerCapabilities};
pub use handler::ProtocolHandler;
pub use messages::{ControlMessage, ControlRequest, ControlResponse, InitRequest, InitResponse};
