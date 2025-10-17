//! Transport layer for communicating with Claude Code CLI
//!
//! This module provides the transport abstraction and implementations for
//! communicating with the Claude Code CLI process.

pub mod subprocess;

use tokio::sync::mpsc;

use crate::error::Result;

/// Transport trait for communicating with Claude Code
///
/// This trait defines the interface for sending and receiving messages
/// to/from the Claude Code CLI process.
pub trait Transport: Send + Sync {
    /// Connect to the transport
    ///
    /// # Errors
    /// Returns error if connection fails
    fn connect(&mut self) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Write data to the transport
    ///
    /// # Arguments
    /// * `data` - String data to write (typically JSON)
    ///
    /// # Errors
    /// Returns error if write fails or transport is not ready
    fn write(&mut self, data: &str) -> impl std::future::Future<Output = Result<()>> + Send;

    /// End the input stream (close stdin)
    ///
    /// # Errors
    /// Returns error if closing fails
    fn end_input(&mut self) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Read messages from the transport
    ///
    /// Returns a receiver that yields JSON values representing messages from Claude Code.
    /// This method spawns a background task to read messages, allowing concurrent writes.
    /// The receiver will be closed when the transport ends or encounters an error.
    fn read_messages(&mut self) -> mpsc::UnboundedReceiver<Result<serde_json::Value>>;

    /// Check if transport is ready for communication
    fn is_ready(&self) -> bool;

    /// Close the transport and clean up resources
    ///
    /// # Errors
    /// Returns error if cleanup fails
    fn close(&mut self) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub use subprocess::{PromptInput, SubprocessTransport};
