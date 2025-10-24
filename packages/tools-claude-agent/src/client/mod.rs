//! `ClaudeSDKClient` for bidirectional communication
//!
//! This module provides the main client for interactive, stateful conversations
//! with Claude Code, including support for:
//! - Bidirectional messaging (no lock contention)
//! - Interrupts and control flow
//! - Hook and permission callbacks
//! - Conversation state management
//!
//! # Architecture
//!
//! The client uses a lock-free architecture for reading and writing:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                   ClaudeSDKClient                        │
//! │                                                          │
//! │  ┌──────────────────┐        ┌──────────────────┐      │
//! │  │  Message Reader  │        │  Control Writer  │      │
//! │  │  Background Task │        │  Background Task │      │
//! │  │                  │        │                  │      │
//! │  │ • Gets receiver  │        │ • Locks per-write│      │
//! │  │   once           │        │ • No blocking    │      │
//! │  │ • No lock held   │        │                  │      │
//! │  │   while reading  │        │                  │      │
//! │  └────────┬─────────┘        └────────┬─────────┘      │
//! │           │                           │                 │
//! │           │    ┌──────────────┐      │                 │
//! │           └───→│  Transport   │←─────┘                 │
//! │                │  (Arc<Mutex>)│                         │
//! │                └──────────────┘                         │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! **Key Design Points:**
//! - Transport returns an owned `UnboundedReceiver` (no lifetime issues)
//! - Reader task gets receiver once, then releases transport lock
//! - Writer task locks transport briefly for each write operation
//! - No contention: reader never blocks writer, writer never blocks reader
//!
//! # Example: Basic Usage
//!
//! ```no_run
//! use kodegen_claude_agent::{ClaudeSDKClient, ClaudeAgentOptions, Message};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::default();
//! let mut client = ClaudeSDKClient::new(options, None).await?;
//!
//! // Send a message
//! client.send_message("Hello, Claude!").await?;
//!
//! // Read responses
//! while let Some(message) = client.next_message().await {
//!     match message? {
//!         Message::Assistant { message, .. } => {
//!             log::info!("Response: {:?}", message.content);
//!         }
//!         Message::Result { .. } => break,
//!         _ => {}
//!     }
//! }
//!
//! client.close().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Concurrent Operations
//!
//! ```no_run
//! use kodegen_claude_agent::{ClaudeSDKClient, ClaudeAgentOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::default();
//! let mut client = ClaudeSDKClient::new(options, None).await?;
//!
//! // Send first message
//! client.send_message("First question").await?;
//!
//! // Can send another message while reading responses
//! // No blocking due to lock-free architecture
//! tokio::spawn(async move {
//!     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
//!     client.send_message("Second question").await
//! });
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Interrupt
//!
//! ```no_run
//! use kodegen_claude_agent::{ClaudeSDKClient, ClaudeAgentOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::default();
//! let mut client = ClaudeSDKClient::new(options, None).await?;
//!
//! client.send_message("Write a long essay").await?;
//!
//! // After some time, interrupt the response
//! tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
//! client.interrupt().await?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Hooks and Permissions
//!
//! ```no_run
//! use kodegen_claude_agent::{ClaudeSDKClient, ClaudeAgentOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::default();
//! let mut client = ClaudeSDKClient::new(options, None).await?;
//!
//! // Take receivers to handle hooks and permissions
//! let mut hook_rx = client.take_hook_receiver()
//!     .ok_or("Hook receiver already taken")?;
//! let mut perm_rx = client.take_permission_receiver()
//!     .ok_or("Permission receiver already taken")?;
//!
//! // Handle hook events
//! tokio::spawn(async move {
//!     while let Some((hook_id, event, event_data)) = hook_rx.recv().await {
//!         log::info!("Hook: {} {:?} with data: {:?}", hook_id, event, event_data);
//!         // Respond to hook...
//!     }
//! });
//!
//! // Handle permission requests
//! tokio::spawn(async move {
//!     while let Some((req_id, request)) = perm_rx.recv().await {
//!         log::info!("Permission: {:?}", request);
//!         // Respond to permission...
//!     }
//! });
//!
//! # Ok(())
//! # }
//! ```

mod client_impl;
mod tasks;

use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use crate::control::ProtocolHandler;
use crate::error::Result;
use crate::hooks::HookManager;
use crate::permissions::PermissionManager;
use crate::transport::SubprocessTransport;
use crate::types::hooks::HookEvent;
use crate::types::identifiers::RequestId;
use crate::types::messages::Message;
use crate::types::permissions::PermissionRequest;

/// Client for bidirectional communication with Claude Code
///
/// `ClaudeSDKClient` provides interactive, stateful conversations with
/// support for interrupts, hooks, and permission callbacks.
///
/// # Examples
///
/// ```no_run
/// use kodegen_claude_agent::{ClaudeSDKClient, ClaudeAgentOptions};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::default();
///     let mut client = ClaudeSDKClient::new(options, None).await?;
///
///     client.send_message("Hello, Claude!").await?;
///
///     while let Some(message) = client.next_message().await {
///         log::info!("{:?}", message?);
///     }
///
///     Ok(())
/// }
/// ```
pub struct ClaudeSDKClient {
    /// Transport layer
    transport: Arc<Mutex<SubprocessTransport>>,
    /// Control protocol handler
    protocol: Arc<Mutex<ProtocolHandler>>,
    /// Message stream receiver
    message_rx: mpsc::UnboundedReceiver<Result<Message>>,
    /// Control message sender
    control_tx: mpsc::UnboundedSender<crate::control::ControlRequest>,
    /// Hook event receiver (if not using automatic handler)
    hook_rx: Option<mpsc::UnboundedReceiver<(String, HookEvent, serde_json::Value)>>,
    /// Permission request receiver (if not using automatic handler)
    permission_rx: Option<mpsc::UnboundedReceiver<(RequestId, PermissionRequest)>>,
    /// Hook manager for automatic hook handling (kept alive for background tasks)
    #[allow(dead_code)]
    // APPROVED BY DAVID MAPLE on 2025-10-14: Required to keep Arc alive for background tasks
    hook_manager: Option<Arc<Mutex<HookManager>>>,
    /// Permission manager for automatic permission handling (kept alive for background tasks)
    #[allow(dead_code)]
    // APPROVED BY DAVID MAPLE on 2025-10-14: Required to keep Arc alive for background tasks
    permission_manager: Option<Arc<Mutex<PermissionManager>>>,
}
