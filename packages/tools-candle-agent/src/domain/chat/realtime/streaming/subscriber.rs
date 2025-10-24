//! Stream subscriber with filtering and statistics

use super::types::{LiveUpdateMessage, MessagePriority};
use crate::domain::util::unix_timestamp_nanos;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

/// Stream subscriber with atomic statistics
#[derive(Debug)]
pub struct StreamSubscriber {
    /// Subscriber ID
    pub id: String,
    /// Session ID filter (None = all sessions)
    pub session_filter: Option<String>,
    /// User ID filter (None = all users)
    pub user_filter: Option<String>,
    /// Minimum priority filter
    pub min_priority: MessagePriority,
    /// Messages received counter
    pub messages_received: Arc<AtomicU64>,
    /// Bytes received counter
    pub bytes_received: Arc<AtomicU64>,
    /// Subscription timestamp
    pub subscribed_at: u64,
    /// Last message timestamp
    pub last_message_at: Arc<AtomicU64>,
    /// Message sender for delivering filtered messages to this subscriber
    pub(crate) message_tx: mpsc::UnboundedSender<LiveUpdateMessage>,
}

impl StreamSubscriber {
    /// Create new stream subscriber
    #[inline]
    pub fn new(id: String, message_tx: mpsc::UnboundedSender<LiveUpdateMessage>) -> Self {
        let now_nanos = unix_timestamp_nanos();

        Self {
            id,
            session_filter: None,
            user_filter: None,
            min_priority: MessagePriority::Normal,
            messages_received: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            subscribed_at: now_nanos,
            last_message_at: Arc::new(AtomicU64::new(now_nanos)),
            message_tx,
        }
    }

    /// Set session filter
    #[inline]
    pub fn with_session_filter(mut self, session_id: String) -> Self {
        self.session_filter = Some(session_id);
        self
    }

    /// Set user filter
    #[inline]
    pub fn with_user_filter(mut self, user_id: String) -> Self {
        self.user_filter = Some(user_id);
        self
    }

    /// Set minimum priority filter
    #[inline]
    pub fn with_min_priority(mut self, priority: MessagePriority) -> Self {
        self.min_priority = priority;
        self
    }

    /// Check if message should be delivered to this subscriber
    pub fn should_receive(&self, message: &LiveUpdateMessage) -> bool {
        // Check session filter
        if let Some(session_filter) = &self.session_filter
            && message.session_id != *session_filter
        {
            return false;
        }

        // Check user filter
        if let Some(user_filter) = &self.user_filter
            && message.user_id != *user_filter
        {
            return false;
        }

        // Check priority filter
        if message.priority < self.min_priority {
            return false;
        }

        true
    }

    /// Record message delivery
    #[inline]
    pub fn record_delivery(&self, message: &LiveUpdateMessage) {
        self.messages_received.fetch_add(1, Ordering::AcqRel);
        self.bytes_received
            .fetch_add(u64::from(message.size_bytes), Ordering::AcqRel);
        self.last_message_at
            .store(unix_timestamp_nanos(), Ordering::Release);
    }

    /// Send message to subscriber's channel
    /// Returns true if sent successfully, false if channel closed
    #[inline]
    pub fn send_message(&self, message: LiveUpdateMessage) -> bool {
        self.message_tx.send(message).is_ok()
    }
}
