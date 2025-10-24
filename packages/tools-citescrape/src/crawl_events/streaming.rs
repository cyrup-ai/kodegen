//! Streaming and filtering functionality for event receivers
//!
//! This module provides filtered event receivers and streaming utilities
//! for selective event consumption.

use std::sync::Arc;
use tokio::sync::broadcast;

use super::errors::EventBusError;
use super::types::CrawlEvent;

/// Filtered event receiver wrapper
pub struct FilteredReceiver<F>
where
    F: Fn(&CrawlEvent) -> bool + Send + Sync + 'static,
{
    receiver: broadcast::Receiver<CrawlEvent>,
    filter: Arc<F>,
}

impl<F> FilteredReceiver<F>
where
    F: Fn(&CrawlEvent) -> bool + Send + Sync + 'static,
{
    pub fn new(receiver: broadcast::Receiver<CrawlEvent>, filter: F) -> Self {
        Self {
            receiver,
            filter: Arc::new(filter),
        }
    }

    /// Receive the next filtered event
    ///
    /// Waits for the next event that passes the filter. Preserves the receiver's
    /// buffered state between calls - no events are lost.
    ///
    /// # Returns
    /// * `Ok(CrawlEvent)` - The next event that passes the filter
    /// * `Err(EventBusError)` - If receiving failed or receiver lagged
    pub async fn recv(&mut self) -> Result<CrawlEvent, EventBusError> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    if (self.filter)(&event) {
                        return Ok(event);
                    }
                    // Continue loop to check next buffered event
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(EventBusError::Shutdown);
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    return Err(EventBusError::ReceiverLagged(skipped));
                }
            }
        }
    }

    /// Try to receive the next filtered event without blocking
    ///
    /// Checks for available events that pass the filter. Does NOT wait if no
    /// matching events are immediately available. Preserves receiver state.
    ///
    /// # Returns
    /// * `Ok(Some(CrawlEvent))` - Event received and passed filter
    /// * `Ok(None)` - No events available or available events don't pass filter
    /// * `Err(EventBusError)` - If receiving failed or receiver lagged
    pub fn try_recv(&mut self) -> Result<Option<CrawlEvent>, EventBusError> {
        loop {
            match self.receiver.try_recv() {
                Ok(event) => {
                    if (self.filter)(&event) {
                        return Ok(Some(event));
                    }
                    // Continue loop to check next buffered event
                    // This is NOT CPU spinning - we're draining the buffer
                }
                Err(broadcast::error::TryRecvError::Empty) => {
                    return Ok(None);
                }
                Err(broadcast::error::TryRecvError::Closed) => {
                    return Err(EventBusError::Shutdown);
                }
                Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                    return Err(EventBusError::ReceiverLagged(skipped));
                }
            }
        }
    }

    /// Check if this receiver will receive specific event types
    ///
    /// # Arguments
    /// * `event` - Test event to check against filter
    #[must_use]
    pub fn would_receive(&self, event: &CrawlEvent) -> bool {
        (self.filter)(event)
    }
}
