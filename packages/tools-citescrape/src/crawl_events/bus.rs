//! Event bus implementation for publishing and subscribing to crawl events
//!
//! This module provides the core event bus functionality with support for
//! metrics, batching, and filtered subscriptions.

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, Notify, broadcast};

use super::config::EventBusConfig;
use super::errors::EventBusError;
use super::metrics::{EventBusMetrics, MetricsSnapshot};
use super::streaming::FilteredReceiver;
use super::types::{BatchPublishResult, CrawlEvent};

/// Event bus for publishing and subscribing to crawl events
#[derive(Debug)]
pub struct CrawlEventBus {
    sender: broadcast::Sender<CrawlEvent>,
    config: Arc<EventBusConfig>,
    metrics: EventBusMetrics,
    shutdown: Arc<Notify>,
    shutdown_flag: Arc<AtomicBool>,
    capacity_notify: Arc<Notify>,
    send_lock: Arc<Mutex<()>>,
    /// Tracks consecutive publish timeout failures for circuit breaker
    consecutive_timeouts: Arc<AtomicUsize>,
    /// Reference count for tracking CrawlEventBus instances (for proper Drop semantics)
    num_instances: Arc<AtomicUsize>,
}

impl CrawlEventBus {
    /// Create a new event bus with the specified capacity
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of events that can be buffered
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let config = EventBusConfig {
            capacity,
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Create a new event bus with custom configuration
    ///
    /// # Arguments
    /// * `config` - Event bus configuration
    #[must_use]
    pub fn with_config(config: EventBusConfig) -> Self {
        let (sender, _) = broadcast::channel(config.capacity);
        let metrics = EventBusMetrics::new();
        let shutdown = Arc::new(Notify::new());
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let capacity_notify = Arc::new(Notify::new());
        let send_lock = Arc::new(Mutex::new(()));
        let consecutive_timeouts = Arc::new(AtomicUsize::new(0));
        let num_instances = Arc::new(AtomicUsize::new(1));
        Self {
            sender,
            config: Arc::new(config),
            metrics,
            shutdown,
            shutdown_flag,
            capacity_notify,
            send_lock,
            consecutive_timeouts,
            num_instances,
        }
    }

    /// Get the current configuration
    #[must_use]
    pub fn config(&self) -> &EventBusConfig {
        &self.config
    }

    /// Get current metrics
    ///
    /// # Consistency Notes
    ///
    /// Returns a reference to the metrics object. Individual counter reads
    /// are atomic, but relationships between counters may be temporarily
    /// inconsistent during concurrent operations. For a consistent view
    /// across all metrics, use `metrics().snapshot()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Individual reads (may be inconsistent)
    /// let published = bus.metrics().get_published();
    /// let dropped = bus.metrics().get_dropped();
    ///
    /// // Consistent snapshot
    /// let snapshot = bus.metrics().snapshot();
    /// assert!(snapshot.events_published >= snapshot.events_dropped);
    /// ```
    #[must_use]
    pub fn metrics(&self) -> &EventBusMetrics {
        &self.metrics
    }

    /// Publish an event to all subscribers
    ///
    /// # Arguments
    /// * `event` - The event to publish
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of active subscribers that received the event
    /// * `Err(EventBusError)` - If publishing failed
    pub async fn publish(&self, event: CrawlEvent) -> Result<usize, EventBusError> {
        if let Ok(subscriber_count) = self.sender.send(event) {
            if self.config.enable_metrics {
                self.metrics.increment_published();
                self.metrics.update_subscriber_count(subscriber_count);

                if subscriber_count == 0 {
                    self.metrics.increment_dropped();
                    log::debug!("Published event but no active subscribers");
                }
            }
            Ok(subscriber_count)
        } else {
            if self.config.enable_metrics {
                self.metrics.increment_failed();
            }
            Err(EventBusError::NoSubscribers)
        }
    }

    /// Publish an event with backpressure control
    ///
    /// Unlike the basic `publish()` method, this method respects the
    /// configured backpressure mode:
    ///
    /// - **`DropOldest`**: Same as `publish()`, never blocks
    /// - **Block**: Waits until space is available (applies backpressure)
    /// - **Error**: Returns `ChannelFull` error if at capacity
    ///
    /// # Arguments
    /// * `event` - The event to publish
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of subscribers that received the event
    /// * `Err(EventBusError::ChannelFull)` - Channel at capacity (Error mode only)
    /// * `Err(EventBusError::NoSubscribers)` - No active subscribers
    ///
    /// # Example with Error Mode
    /// ```
    /// let config = EventBusConfig {
    ///     backpressure_mode: BackpressureMode::Error,
    ///     ..Default::default()
    /// };
    /// let bus = CrawlEventBus::with_config(config);
    ///
    /// match bus.publish_with_backpressure(event).await {
    ///     Ok(count) => log::info!("Published to {} subscribers", count),
    ///     Err(EventBusError::ChannelFull) => {
    ///         log::warn!("Channel full, dropping event or retry later");
    ///     }
    ///     Err(e) => log::error!("Publish failed: {}", e),
    /// }
    /// ```
    ///
    /// # Example with Block Mode
    /// ```
    /// let config = EventBusConfig {
    ///     backpressure_mode: BackpressureMode::Block,
    ///     ..Default::default()
    /// };
    /// let bus = CrawlEventBus::with_config(config);
    ///
    /// // This will wait until space is available
    /// let count = bus.publish_with_backpressure(event).await?;
    /// ```
    pub async fn publish_with_backpressure(
        &self,
        event: CrawlEvent,
    ) -> Result<usize, EventBusError> {
        use super::config::BackpressureMode;

        match self.config.backpressure_mode {
            BackpressureMode::DropOldest => {
                // Delegate to publish() - same behavior, no duplication
                self.publish(event).await
            }

            BackpressureMode::Block => {
                // Circuit breaker: check if we've exceeded timeout threshold
                let timeout_count = self.consecutive_timeouts.load(Ordering::Acquire);
                if timeout_count > 10 {
                    log::warn!(
                        "Circuit breaker opened after {timeout_count} consecutive timeouts, falling back to async mode"
                    );
                    // Fall back to DropOldest mode to prevent complete system hang
                    return self.publish(event).await;
                }

                // Wrap the blocking wait in a 30-second timeout to prevent deadlocks
                let publish_future = async {
                    // Wait until space is available using notification + timeout fallback
                    loop {
                        // Check if we have space
                        if self.sender.len() < self.config.capacity {
                            break;
                        }

                        // Check if bus is shutdown
                        if self.is_shutdown() {
                            return Err(EventBusError::Shutdown);
                        }

                        // Wait for capacity notification OR timeout (5ms fallback)
                        // Timeout ensures we recheck even if notification is missed
                        let _ = tokio::time::timeout(
                            tokio::time::Duration::from_millis(5),
                            self.capacity_notify.notified(),
                        )
                        .await;
                    }

                    // Now publish (should succeed since we have space)
                    if let Ok(subscriber_count) = self.sender.send(event) {
                        if self.config.enable_metrics {
                            self.metrics.increment_published();
                            self.metrics.update_subscriber_count(subscriber_count);

                            if subscriber_count == 0 {
                                self.metrics.increment_dropped();
                            }
                        }

                        // Wake one waiting publisher (if any) now that we've published
                        // This creates a chain where publishers wake each other
                        self.capacity_notify.notify_one();

                        Ok(subscriber_count)
                    } else {
                        if self.config.enable_metrics {
                            self.metrics.increment_failed();
                        }
                        Err(EventBusError::NoSubscribers)
                    }
                };

                // Apply 30-second timeout to prevent indefinite deadlock
                match tokio::time::timeout(Duration::from_secs(30), publish_future).await {
                    Ok(Ok(count)) => {
                        // Success: reset the timeout counter
                        self.consecutive_timeouts.store(0, Ordering::Release);
                        Ok(count)
                    }
                    Ok(Err(e)) => {
                        // Publish failed but not due to timeout
                        Err(e)
                    }
                    Err(_elapsed) => {
                        // Timeout occurred: increment counter and check circuit breaker
                        let new_count =
                            self.consecutive_timeouts.fetch_add(1, Ordering::AcqRel) + 1;

                        if new_count > 10 {
                            // Circuit breaker will trigger on next call
                            log::error!(
                                "Publish timeout #{new_count}: circuit breaker will open on next attempt"
                            );
                        } else {
                            log::warn!(
                                "Publish timeout #{new_count} after 30s waiting for channel capacity"
                            );
                        }

                        Err(EventBusError::PublishTimeout)
                    }
                }
            }

            BackpressureMode::Error => {
                // Acquire lock to serialize check-and-send (eliminates TOCTOU race)
                let _guard = self.send_lock.lock().await;

                // Check and send are now atomic (serialized by mutex)
                if self.sender.len() >= self.config.capacity {
                    return Err(EventBusError::ChannelFull);
                }

                // Send with reserved slot (protected by lock)
                if let Ok(subscriber_count) = self.sender.send(event) {
                    if self.config.enable_metrics {
                        self.metrics.increment_published();
                        self.metrics.update_subscriber_count(subscriber_count);

                        if subscriber_count == 0 {
                            self.metrics.increment_dropped();
                        }
                    }
                    Ok(subscriber_count)
                } else {
                    if self.config.enable_metrics {
                        self.metrics.increment_failed();
                    }
                    Err(EventBusError::NoSubscribers)
                }
                // Lock automatically released when _guard drops
            }
        }
    }

    /// Subscribe to events
    ///
    /// # Returns
    /// A receiver that can be used to listen for events
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<CrawlEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active subscribers
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        let count = self.sender.receiver_count();
        if self.config.enable_metrics {
            self.metrics.update_subscriber_count(count);
        }
        count
    }

    /// Check if the event bus has any active subscribers
    #[must_use]
    pub fn has_subscribers(&self) -> bool {
        self.subscriber_count() > 0
    }

    /// Get current channel pressure (0.0 to 1.0+)
    ///
    /// Returns the ratio of used capacity to total capacity.
    /// - 0.0 = empty
    /// - 0.5 = half full
    /// - 1.0 = at capacity
    /// - >1.0 = impossible (channel drops oldest events)
    ///
    /// # Example
    /// ```
    /// let bus = CrawlEventBus::new(1000);
    /// // ... publish some events ...
    /// let pressure = bus.pressure();
    /// if pressure > 0.8 {
    ///     log::warn!("Channel is {}% full", pressure * 100.0);
    /// }
    /// ```
    #[must_use]
    pub fn pressure(&self) -> f64 {
        let current = self.sender.len();
        let capacity = self.config.capacity;
        current as f64 / capacity as f64
    }

    /// Check if channel is overloaded
    ///
    /// Returns true if pressure exceeds the configured threshold
    /// (default 0.8 = 80% capacity)
    ///
    /// # Example
    /// ```
    /// if bus.is_overloaded() {
    ///     log::warn!("Event bus overloaded, consider slowing down");
    ///     tokio::time::sleep(Duration::from_millis(10)).await;
    /// }
    /// ```
    #[must_use]
    pub fn is_overloaded(&self) -> bool {
        self.pressure() >= self.config.overload_threshold
    }

    /// Get current number of events in the channel buffer
    #[must_use]
    pub fn buffer_len(&self) -> usize {
        self.sender.len()
    }

    /// Get remaining capacity before channel is full
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.config.capacity.saturating_sub(self.sender.len())
    }

    /// Create a filtered subscriber that only receives specific event types
    ///
    /// # Arguments
    /// * `filter` - Function that returns true if event should be passed through
    pub fn subscribe_filtered<F>(&self, filter: F) -> FilteredReceiver<F>
    where
        F: Fn(&CrawlEvent) -> bool + Send + Sync + 'static,
    {
        let receiver = self.subscribe();
        FilteredReceiver::new(receiver, filter)
    }

    /// Publish multiple events as a batch
    /// Publish multiple events as a batch with best-effort delivery
    ///
    /// This method publishes all events in the batch regardless of individual failures.
    /// Unlike a transactional approach, partial success is acceptable and fully reported.
    ///
    /// # Best-Effort Semantics
    ///
    /// All events are attempted. Failures (typically due to no active subscribers) don't
    /// stop processing of remaining events. Returns a `BatchPublishResult` with explicit
    /// counts showing exactly how many succeeded vs failed.
    ///
    /// # Arguments
    ///
    /// * `events` - Vector of events to publish
    ///
    /// # Returns
    ///
    /// `BatchPublishResult` with detailed success/failure statistics
    ///
    /// # Example
    ///
    /// ```rust
    /// let events = vec![
    ///     CrawlEvent::page_crawled(...),
    ///     CrawlEvent::page_crawled(...),
    ///     CrawlEvent::crawl_completed(...),
    /// ];
    ///
    /// let result = bus.publish_batch(events).await;
    /// println!("Published {}/{} events to {} subscribers",
    ///          result.published, result.total, result.max_subscribers);
    ///
    /// if result.has_failures() {
    ///     log::warn!("{} events failed (no subscribers)", result.failed);
    /// }
    ///
    /// if result.is_complete() {
    ///     log::info!("All events delivered successfully");
    /// }
    /// ```
    pub async fn publish_batch(&self, events: Vec<CrawlEvent>) -> BatchPublishResult {
        let total = events.len();
        let mut published = 0;
        let mut failed = 0;
        let mut max_subscribers = 0;

        for event in events {
            if let Ok(count) = self.sender.send(event) {
                published += 1;
                max_subscribers = std::cmp::max(max_subscribers, count);

                if self.config.enable_metrics {
                    self.metrics.increment_published();
                    self.metrics.update_subscriber_count(count);
                    if count == 0 {
                        self.metrics.increment_dropped();
                    }
                }
            } else {
                failed += 1;
                if self.config.enable_metrics {
                    self.metrics.increment_failed();
                }
            }
        }

        BatchPublishResult {
            total,
            published,
            failed,
            max_subscribers,
        }
    }

    /// Get detailed metrics report
    ///
    /// Uses a snapshot to ensure all metrics are consistent with each other.
    #[must_use]
    pub fn get_metrics_report(&self) -> String {
        if !self.config.enable_metrics {
            return "Metrics disabled".to_string();
        }

        let snapshot = self.metrics.snapshot();

        format!(
            "Event Bus Metrics:\n\
             - Events Published: {}\n\
             - Events Dropped: {}\n\
             - Events Failed: {}\n\
             - Active Subscribers: {}\n\
             - Peak Subscribers: {}\n\
             - Success Rate: {:.2}%",
            snapshot.events_published,
            snapshot.events_dropped,
            snapshot.events_failed,
            snapshot.active_subscribers,
            snapshot.peak_subscribers,
            self.calculate_success_rate_from_snapshot(&snapshot)
        )
    }

    /// Signal shutdown to all subscribers
    ///
    /// This method is idempotent - calling it multiple times is safe.
    /// All clones of this bus share the same shutdown signal.
    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        self.shutdown.notify_waiters();
        log::debug!("Event bus shutdown signaled");
    }

    /// Wait for shutdown signal
    ///
    /// Subscribers should use this with `tokio::select`! to exit gracefully:
    ///
    /// ```rust
    /// tokio::select! {
    ///     Ok(event) = rx.recv() => { /* handle event */ }
    ///     _ = bus.wait_for_shutdown() => { break; }
    /// }
    /// ```
    pub async fn wait_for_shutdown(&self) {
        self.shutdown.notified().await;
    }

    /// Check if shutdown has been signaled
    ///
    /// Returns true if `shutdown()` has been called on this bus or any of its clones.
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Gracefully shutdown the event bus with proper draining
    ///
    /// This method ensures no events are lost during shutdown:
    ///
    /// 1. **Set shutdown flag** - Prevents new operations from starting
    /// 2. **Publish shutdown event** - Notifies subscribers via event stream
    /// 3. **Wait for subscriber processing** - Gives time for subscribers to drain (500ms)
    /// 4. **Signal shutdown complete** - Wakes waiting tasks
    ///
    /// # Timeouts
    ///
    /// - **Subscriber drain**: 500ms (depends on subscriber processing speed)
    /// - **Total maximum**: 500ms
    ///
    /// If timeouts are exceeded, a warning is logged but shutdown proceeds to prevent hangs.
    ///
    /// # Example
    ///
    /// ```rust
    /// // At end of crawl
    /// bus.shutdown_gracefully(ShutdownReason::CrawlCompleted).await;
    ///
    /// // On error
    /// bus.shutdown_gracefully(ShutdownReason::Error(e.to_string())).await;
    /// ```
    pub async fn shutdown_gracefully(&self, reason: super::types::ShutdownReason) {
        log::info!("Beginning graceful shutdown of event bus: {reason:?}");

        // Phase 1: Set shutdown flag to prevent new operations
        self.shutdown_flag.store(true, Ordering::SeqCst);
        log::debug!("Shutdown flag set");

        // Phase 2: Publish shutdown event
        log::debug!("Publishing shutdown event");
        let event = CrawlEvent::shutdown(reason);
        let _ = self.publish(event).await;

        // Phase 3: Wait for subscribers to process buffered events
        // This is a heuristic - we can't know when subscribers are truly done
        // without explicit acknowledgment, so we use a generous timeout
        log::debug!("Waiting for subscribers to process events");
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Phase 4: Signal final shutdown
        self.shutdown.notify_waiters();

        log::info!("Event bus graceful shutdown complete");
    }

    fn calculate_success_rate_from_snapshot(&self, snapshot: &MetricsSnapshot) -> f64 {
        let total = snapshot.events_published;
        if total == 0 {
            return 100.0;
        }
        let failed = snapshot.events_failed;
        ((total - failed) as f64 / total as f64) * 100.0
    }
}

impl Default for CrawlEventBus {
    fn default() -> Self {
        Self::with_config(EventBusConfig::default())
    }
}

impl Clone for CrawlEventBus {
    fn clone(&self) -> Self {
        // Increment instance count (follows tokio's broadcast::Sender pattern)
        self.num_instances.fetch_add(1, Ordering::Relaxed);
        Self {
            sender: self.sender.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            shutdown: self.shutdown.clone(),
            shutdown_flag: self.shutdown_flag.clone(),
            capacity_notify: self.capacity_notify.clone(),
            send_lock: self.send_lock.clone(),
            consecutive_timeouts: self.consecutive_timeouts.clone(),
            num_instances: self.num_instances.clone(),
        }
    }
}

impl Drop for CrawlEventBus {
    fn drop(&mut self) {
        // Only shutdown when the LAST instance is dropped (follows tokio's pattern)
        // fetch_sub returns the value BEFORE decrementing
        if 1 == self.num_instances.fetch_sub(1, Ordering::AcqRel) {
            // This was the last instance - trigger shutdown
            self.shutdown_flag.store(true, Ordering::SeqCst);
            self.shutdown.notify_waiters();
            log::trace!("Event bus dropped (last instance), shutdown signal sent");
        }
    }
}
