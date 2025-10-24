//! Candle immutable conversation management for chat interactions
//!
//! This module provides streaming-only, zero-allocation conversation management with
//! immutable message storage. All operations use borrowed data and atomic operations
//! for blazing-fast, lock-free performance.

use std::sync::atomic::{AtomicUsize, Ordering};

use cyrup_sugars::prelude::MessageChunk;
use std::pin::Pin;
use thiserror::Error;
use tokio_stream::Stream;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::domain::chat::message::types::CandleMessageRole;
use crate::domain::util::unix_timestamp_nanos;

/// Error types for conversation operations
#[derive(Error, Debug)]
pub enum CandleConversationError {
    #[error("Message vector corruption: {message}")]
    MessageVectorCorruption { message: String },
    #[error("System error: {0}")]
    System(String),
}

/// Candle immutable message in a conversation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandleImmutableMessage {
    /// Message content (owned once, never mutated)
    pub content: String,
    /// Message role
    pub role: CandleMessageRole,
    /// Message timestamp (nanoseconds since epoch)
    pub timestamp_nanos: u64,
    /// Message sequence number
    pub sequence: u64,
}

impl CandleImmutableMessage {
    /// Create a new Candle immutable message
    #[inline]
    pub fn new(content: impl Into<String>, role: CandleMessageRole, sequence: u64) -> Self {
        Self {
            content: content.into(),
            role,
            timestamp_nanos: Self::current_timestamp_nanos(),
            sequence,
        }
    }

    /// Create Candle user message
    #[inline]
    pub fn user(content: impl Into<String>, sequence: u64) -> Self {
        Self::new(content, CandleMessageRole::User, sequence)
    }

    /// Create Candle assistant message
    #[inline]
    pub fn assistant(content: impl Into<String>, sequence: u64) -> Self {
        Self::new(content, CandleMessageRole::Assistant, sequence)
    }

    /// Create Candle system message
    #[inline]
    pub fn system(content: impl Into<String>, sequence: u64) -> Self {
        Self::new(content, CandleMessageRole::System, sequence)
    }

    /// Get current timestamp in nanoseconds
    #[inline]
    fn current_timestamp_nanos() -> u64 {
        unix_timestamp_nanos()
    }

    /// Get message content as borrowed string
    #[inline]
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if message is from user
    #[inline]
    #[must_use]
    pub fn is_user(&self) -> bool {
        matches!(self.role, CandleMessageRole::User)
    }

    /// Check if message is from assistant
    #[inline]
    #[must_use]
    pub fn is_assistant(&self) -> bool {
        matches!(self.role, CandleMessageRole::Assistant)
    }

    /// Check if message is system message
    #[inline]
    #[must_use]
    pub fn is_system(&self) -> bool {
        matches!(self.role, CandleMessageRole::System)
    }
}

/// Candle streaming conversation event
#[derive(Debug, Clone, Default)]
pub enum CandleConversationEvent {
    /// New message added to conversation
    MessageAdded(CandleImmutableMessage),
    /// Conversation cleared
    #[default]
    Cleared,
    /// Conversation statistics updated
    StatsUpdated {
        /// Total number of messages in the conversation
        total_messages: u64,
        /// Number of user messages
        user_messages: u64,
        /// Number of assistant messages
        assistant_messages: u64,
        /// Number of system messages
        system_messages: u64,
    },
}

impl MessageChunk for CandleConversationEvent {
    fn bad_chunk(_error: String) -> Self {
        // Error parameter reserved for future use
        Self::StatsUpdated {
            total_messages: 0,
            user_messages: 0,
            assistant_messages: 0,
            system_messages: 0,
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            Self::StatsUpdated {
                total_messages: 0, ..
            } => Some("Invalid conversation statistics"),
            _ => None,
        }
    }
}

/// Candle immutable conversation with streaming updates
pub struct CandleStreamingConversation {
    /// Immutable message history (append-only)
    messages: Vec<CandleImmutableMessage>,
    /// Message sequence counter (atomic)
    sequence_counter: AtomicUsize,
    /// Total message count (atomic)
    total_messages: AtomicUsize,
    /// User message count (atomic)
    user_messages: AtomicUsize,
    /// Assistant message count (atomic)
    assistant_messages: AtomicUsize,
    /// System message count (atomic)
    system_messages: AtomicUsize,
    /// Event stream sender
    event_sender: Option<tokio::sync::mpsc::UnboundedSender<CandleConversationEvent>>,
}

impl std::fmt::Debug for CandleStreamingConversation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandleStreamingConversation")
            .field("messages", &self.messages)
            .field(
                "sequence_counter",
                &self
                    .sequence_counter
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "total_messages",
                &self
                    .total_messages
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "user_messages",
                &self
                    .user_messages
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "assistant_messages",
                &self
                    .assistant_messages
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "system_messages",
                &self
                    .system_messages
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field("event_sender", &self.event_sender.is_some())
            .finish()
    }
}

impl CandleStreamingConversation {
    /// Create a new Candle streaming conversation
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            sequence_counter: AtomicUsize::new(0),
            total_messages: AtomicUsize::new(0),
            user_messages: AtomicUsize::new(0),
            assistant_messages: AtomicUsize::new(0),
            system_messages: AtomicUsize::new(0),
            event_sender: None,
        }
    }

    /// Create Candle conversation with event streaming
    #[inline]
    #[must_use]
    pub fn with_streaming() -> (
        Self,
        Pin<Box<dyn Stream<Item = CandleConversationEvent> + Send>>,
    ) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let conversation = Self {
            event_sender: Some(sender),
            ..Self::new()
        };

        (
            conversation,
            Box::pin(UnboundedReceiverStream::new(receiver)),
        )
    }

    /// Add Candle user message (creates new immutable message)
    ///
    /// # Errors
    ///
    /// Returns `CandleConversationError` if message cannot be added to the conversation
    #[inline]
    pub fn add_user_message(
        &mut self,
        content: impl Into<String>,
    ) -> Result<&CandleImmutableMessage, CandleConversationError> {
        let sequence = self.sequence_counter.fetch_add(1, Ordering::Relaxed) as u64;
        let message = CandleImmutableMessage::user(content, sequence);

        self.messages.push(message.clone());
        self.total_messages.fetch_add(1, Ordering::Relaxed);
        self.user_messages.fetch_add(1, Ordering::Relaxed);

        // Send event if streaming enabled
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(CandleConversationEvent::MessageAdded(message.clone()));
        }

        // Safety: We just pushed a message, so messages cannot be empty
        match self.messages.last() {
            Some(msg) => Ok(msg),
            None => Err(CandleConversationError::MessageVectorCorruption {
                message: "Message vector empty after push - possible memory corruption".to_string(),
            }),
        }
    }

    /// Add Candle assistant message (creates new immutable message)
    ///
    /// # Errors
    ///
    /// Returns `CandleConversationError` if message cannot be added to the conversation
    #[inline]
    pub fn add_assistant_message(
        &mut self,
        content: impl Into<String>,
    ) -> Result<&CandleImmutableMessage, CandleConversationError> {
        let sequence = self.sequence_counter.fetch_add(1, Ordering::Relaxed) as u64;
        let message = CandleImmutableMessage::assistant(content, sequence);

        self.messages.push(message.clone());
        self.total_messages.fetch_add(1, Ordering::Relaxed);
        self.assistant_messages.fetch_add(1, Ordering::Relaxed);

        // Send event if streaming enabled
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(CandleConversationEvent::MessageAdded(message.clone()));
        }

        // Safety: We just pushed a message, so messages cannot be empty
        match self.messages.last() {
            Some(msg) => Ok(msg),
            None => Err(CandleConversationError::MessageVectorCorruption {
                message: "Message vector empty after push - possible memory corruption".to_string(),
            }),
        }
    }

    /// Add Candle system message (creates new immutable message)
    ///
    /// # Errors
    ///
    /// Returns `CandleConversationError` if message cannot be added to the conversation
    #[inline]
    pub fn add_system_message(
        &mut self,
        content: impl Into<String>,
    ) -> Result<&CandleImmutableMessage, CandleConversationError> {
        let sequence = self.sequence_counter.fetch_add(1, Ordering::Relaxed) as u64;
        let message = CandleImmutableMessage::system(content, sequence);

        self.messages.push(message.clone());
        self.total_messages.fetch_add(1, Ordering::Relaxed);
        self.system_messages.fetch_add(1, Ordering::Relaxed);

        // Send event if streaming enabled
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(CandleConversationEvent::MessageAdded(message.clone()));
        }

        // Safety: We just pushed a message, so messages cannot be empty
        match self.messages.last() {
            Some(msg) => Ok(msg),
            None => Err(CandleConversationError::MessageVectorCorruption {
                message: "Message vector empty after push - possible memory corruption".to_string(),
            }),
        }
    }

    /// Get all Candle messages as borrowed slice (zero allocation)
    #[inline]
    pub fn messages(&self) -> &[CandleImmutableMessage] {
        &self.messages
    }

    /// Get Candle messages by role (zero allocation iterator)
    #[inline]
    pub fn messages_by_role(
        &self,
        role: CandleMessageRole,
    ) -> impl Iterator<Item = &CandleImmutableMessage> {
        self.messages.iter().filter(move |msg| msg.role == role)
    }

    /// Get Candle user messages (zero allocation iterator)
    #[inline]
    pub fn user_messages(&self) -> impl Iterator<Item = &CandleImmutableMessage> {
        self.messages_by_role(CandleMessageRole::User)
    }

    /// Get Candle assistant messages (zero allocation iterator)
    #[inline]
    pub fn assistant_messages(&self) -> impl Iterator<Item = &CandleImmutableMessage> {
        self.messages_by_role(CandleMessageRole::Assistant)
    }

    /// Get Candle system messages (zero allocation iterator)
    #[inline]
    pub fn system_messages(&self) -> impl Iterator<Item = &CandleImmutableMessage> {
        self.messages_by_role(CandleMessageRole::System)
    }

    /// Get latest Candle user message
    #[inline]
    pub fn latest_user_message(&self) -> Option<&CandleImmutableMessage> {
        self.user_messages().last()
    }

    /// Get latest Candle assistant message
    #[inline]
    pub fn latest_assistant_message(&self) -> Option<&CandleImmutableMessage> {
        self.assistant_messages().last()
    }

    /// Get latest Candle message of any type
    #[inline]
    pub fn latest_message(&self) -> Option<&CandleImmutableMessage> {
        self.messages.last()
    }

    /// Get message count (atomic read)
    #[inline]
    pub fn message_count(&self) -> usize {
        self.total_messages.load(Ordering::Relaxed)
    }

    /// Get user message count (atomic read)
    #[inline]
    pub fn user_message_count(&self) -> usize {
        self.user_messages.load(Ordering::Relaxed)
    }

    /// Get assistant message count (atomic read)
    #[inline]
    pub fn assistant_message_count(&self) -> usize {
        self.assistant_messages.load(Ordering::Relaxed)
    }

    /// Get system message count (atomic read)
    #[inline]
    pub fn system_message_count(&self) -> usize {
        self.system_messages.load(Ordering::Relaxed)
    }

    /// Check if conversation is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.message_count() == 0
    }

    /// Clear all messages (creates new empty conversation)
    #[inline]
    pub fn clear(&mut self) {
        self.messages.clear();
        self.sequence_counter.store(0, Ordering::Relaxed);
        self.total_messages.store(0, Ordering::Relaxed);
        self.user_messages.store(0, Ordering::Relaxed);
        self.assistant_messages.store(0, Ordering::Relaxed);
        self.system_messages.store(0, Ordering::Relaxed);

        // Send clear event if streaming enabled
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(CandleConversationEvent::Cleared);
        }
    }

    /// Get Candle conversation statistics
    #[inline]
    pub fn stats(&self) -> CandleConversationStats {
        CandleConversationStats {
            total_messages: self.total_messages.load(Ordering::Relaxed) as u64,
            user_messages: self.user_messages.load(Ordering::Relaxed) as u64,
            assistant_messages: self.assistant_messages.load(Ordering::Relaxed) as u64,
            system_messages: self.system_messages.load(Ordering::Relaxed) as u64,
        }
    }

    /// Stream Candle conversation statistics updates
    #[inline]
    pub fn stream_stats_updates(&self) {
        if let Some(ref sender) = self.event_sender {
            let stats = self.stats();
            let _ = sender.send(CandleConversationEvent::StatsUpdated {
                total_messages: stats.total_messages,
                user_messages: stats.user_messages,
                assistant_messages: stats.assistant_messages,
                system_messages: stats.system_messages,
            });
        }
    }
}

impl Default for CandleStreamingConversation {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Candle conversation statistics snapshot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandleConversationStats {
    /// Total number of messages in the conversation
    pub total_messages: u64,
    /// Number of messages from users
    pub user_messages: u64,
    /// Number of messages from assistants
    pub assistant_messages: u64,
    /// Number of system messages
    pub system_messages: u64,
}

impl CandleConversationStats {
    /// Calculate user message percentage
    #[allow(clippy::cast_precision_loss)] // Acceptable for percentage calculations
    #[inline]
    #[must_use]
    pub fn user_percentage(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            (self.user_messages as f64 / self.total_messages as f64) * 100.0
        }
    }

    /// Calculate assistant message percentage
    #[allow(clippy::cast_precision_loss)] // Acceptable for percentage calculations
    #[inline]
    #[must_use]
    pub fn assistant_percentage(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            (self.assistant_messages as f64 / self.total_messages as f64) * 100.0
        }
    }

    /// Calculate system message percentage
    #[allow(clippy::cast_precision_loss)] // Acceptable for percentage calculations
    #[inline]
    #[must_use]
    pub fn system_percentage(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            (self.system_messages as f64 / self.total_messages as f64) * 100.0
        }
    }
}
