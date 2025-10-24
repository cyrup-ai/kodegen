//! Live message streamer with lock-free queuing

use atomic_counter::{AtomicCounter, ConsistentCounter};
use crossbeam_skiplist::SkipMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::Stream;
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::super::events::RealTimeEvent;
use super::processing;
use super::results::{ProcessingEvent, StreamingResult, UnsubscribeResult};
use super::stats::StreamingStatistics;
use super::subscriber::StreamSubscriber;
use super::types::LiveUpdateMessage;
use crate::domain::util::unix_timestamp_nanos;

/// Live message streamer with lock-free queuing and atomic statistics
pub struct LiveMessageStreamer {
    /// Message queue for lock-free streaming
    message_queue_tx: mpsc::UnboundedSender<LiveUpdateMessage>,
    message_queue_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<LiveUpdateMessage>>>,
    /// Priority queue for high-priority messages
    priority_queue_tx: mpsc::UnboundedSender<LiveUpdateMessage>,
    priority_queue_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<LiveUpdateMessage>>>,
    /// Active subscribers with zero-allocation keys
    subscribers: Arc<SkipMap<String, Arc<StreamSubscriber>>>,
    /// Event broadcaster for real-time notifications
    event_broadcaster: broadcast::Sender<RealTimeEvent>,
    /// Message counter for statistics
    message_counter: Arc<AtomicUsize>,
    /// Priority message counter
    priority_message_counter: Arc<AtomicUsize>,
    /// Subscriber counter
    subscriber_counter: Arc<ConsistentCounter>,
    /// Total bytes processed
    bytes_processed: Arc<AtomicU64>,
    /// Queue size limit for backpressure
    queue_size_limit: Arc<AtomicUsize>,
    /// Backpressure threshold for warning triggers
    backpressure_threshold: Arc<AtomicUsize>,
    /// Processing rate in messages per second
    processing_rate: Arc<AtomicU64>,
    /// Sequence number generator
    sequence_generator: Arc<AtomicU64>,
    /// Backpressure event counter
    backpressure_events: Arc<AtomicU64>,
    /// Processing task active flag
    processing_active: Arc<AtomicBool>,
}

impl LiveMessageStreamer {
    /// Create new live message streamer with lock-free architecture
    #[inline]
    #[must_use]
    pub fn new(
        queue_size_limit: usize,
        backpressure_threshold: usize,
        processing_rate: u64,
    ) -> Self {
        let (message_queue_tx, message_queue_rx) = mpsc::unbounded_channel();
        let (priority_queue_tx, priority_queue_rx) = mpsc::unbounded_channel();
        let (event_broadcaster, _) = broadcast::channel(10000);

        Self {
            message_queue_tx,
            message_queue_rx: Arc::new(tokio::sync::Mutex::new(message_queue_rx)),
            priority_queue_tx,
            priority_queue_rx: Arc::new(tokio::sync::Mutex::new(priority_queue_rx)),
            subscribers: Arc::new(SkipMap::new()),
            event_broadcaster,
            message_counter: Arc::new(AtomicUsize::new(0)),
            priority_message_counter: Arc::new(AtomicUsize::new(0)),
            subscriber_counter: Arc::new(ConsistentCounter::new(0)),
            bytes_processed: Arc::new(AtomicU64::new(0)),
            queue_size_limit: Arc::new(AtomicUsize::new(queue_size_limit)),
            backpressure_threshold: Arc::new(AtomicUsize::new(backpressure_threshold)),
            processing_rate: Arc::new(AtomicU64::new(processing_rate)),
            sequence_generator: Arc::new(AtomicU64::new(0)),
            backpressure_events: Arc::new(AtomicU64::new(0)),
            processing_active: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Send message to normal queue with backpressure detection
    #[inline]
    #[must_use]
    pub fn send_message(&self, mut message: LiveUpdateMessage) -> StreamingResult {
        let current_size = self.message_counter.load(Ordering::Acquire);
        let limit = self.queue_size_limit.load(Ordering::Acquire);
        let threshold = self.backpressure_threshold.load(Ordering::Acquire);

        // Check backpressure limits
        if current_size >= limit {
            self.backpressure_events.fetch_add(1, Ordering::AcqRel);
            return StreamingResult::BackpressureError {
                current_size,
                limit,
                message_id: message.id,
            };
        }

        // Generate sequence number
        let sequence_number = self.sequence_generator.fetch_add(1, Ordering::AcqRel);
        message.sequence_number = sequence_number;

        // Send message
        if self.message_queue_tx.send(message.clone()).is_ok() {
            self.message_counter.fetch_add(1, Ordering::AcqRel);
            self.bytes_processed
                .fetch_add(u64::from(message.size_bytes), Ordering::AcqRel);

            // Broadcast event
            let event = message.to_real_time_event();
            let _ = self.event_broadcaster.send(event);

            // Check if approaching threshold
            if current_size >= threshold {
                self.backpressure_events.fetch_add(1, Ordering::AcqRel);
                StreamingResult::BackpressureWarning {
                    current_size,
                    threshold,
                    message_id: message.id,
                }
            } else {
                StreamingResult::MessageQueued {
                    message_id: message.id,
                    sequence_number,
                    queue_position: current_size,
                }
            }
        } else {
            StreamingResult::BackpressureError {
                current_size,
                limit,
                message_id: message.id,
            }
        }
    }

    /// Send message to priority queue with expedited processing
    #[inline]
    #[must_use]
    pub fn send_priority_message(&self, mut message: LiveUpdateMessage) -> StreamingResult {
        let current_size = self.priority_message_counter.load(Ordering::Acquire);
        let limit = self.queue_size_limit.load(Ordering::Acquire);
        let threshold = self.backpressure_threshold.load(Ordering::Acquire);

        // Check backpressure limits
        if current_size >= limit {
            self.backpressure_events.fetch_add(1, Ordering::AcqRel);
            return StreamingResult::BackpressureError {
                current_size,
                limit,
                message_id: message.id,
            };
        }

        // Generate sequence number
        let sequence_number = self.sequence_generator.fetch_add(1, Ordering::AcqRel);
        message.sequence_number = sequence_number;

        // Send message
        if self.priority_queue_tx.send(message.clone()).is_ok() {
            self.priority_message_counter.fetch_add(1, Ordering::AcqRel);
            self.bytes_processed
                .fetch_add(u64::from(message.size_bytes), Ordering::AcqRel);

            // Broadcast event
            let event = message.to_real_time_event();
            let _ = self.event_broadcaster.send(event);

            // Check if approaching threshold
            if current_size >= threshold {
                self.backpressure_events.fetch_add(1, Ordering::AcqRel);
                StreamingResult::BackpressureWarning {
                    current_size,
                    threshold,
                    message_id: message.id,
                }
            } else {
                StreamingResult::MessageQueued {
                    message_id: message.id,
                    sequence_number,
                    queue_position: current_size,
                }
            }
        } else {
            StreamingResult::BackpressureError {
                current_size,
                limit,
                message_id: message.id,
            }
        }
    }

    /// Subscribe to message stream with optional filters
    #[inline]
    #[must_use]
    pub fn subscribe(
        &self,
        subscriber: StreamSubscriber,
    ) -> Pin<Box<dyn Stream<Item = LiveUpdateMessage> + Send>> {
        // Create channel for this subscriber
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        // Create subscriber with channel (preserve filters from input)
        let subscriber_with_channel = StreamSubscriber {
            id: subscriber.id.clone(),
            session_filter: subscriber.session_filter,
            user_filter: subscriber.user_filter,
            min_priority: subscriber.min_priority,
            messages_received: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            subscribed_at: unix_timestamp_nanos(),
            last_message_at: Arc::new(AtomicU64::new(unix_timestamp_nanos())),
            message_tx,
        };

        let subscriber_arc = Arc::new(subscriber_with_channel);
        let subscriber_id = subscriber_arc.id.clone();

        self.subscribers.insert(subscriber_id, subscriber_arc);
        self.subscriber_counter.inc();

        // Return stream of messages for this subscriber
        Box::pin(UnboundedReceiverStream::new(message_rx))
    }

    /// Unsubscribe from message stream
    #[must_use]
    pub fn unsubscribe(
        &self,
        subscriber_id: &str,
    ) -> Pin<Box<dyn Stream<Item = UnsubscribeResult> + Send>> {
        let subscribers = self.subscribers.clone();
        let subscriber_counter = self.subscriber_counter.clone();
        let id = subscriber_id.to_string();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let result = if subscribers.remove(&id).is_some() {
                // Decrement counter using reset/inc pattern (no .dec() method exists)
                let current = subscriber_counter.get();
                if current > 0 {
                    subscriber_counter.reset();
                    for _ in 0..(current - 1) {
                        subscriber_counter.inc();
                    }
                }

                UnsubscribeResult::Success {
                    subscriber_id: id,
                    remaining_subscribers: subscriber_counter.get() as u64,
                }
            } else {
                UnsubscribeResult::NotFound { subscriber_id: id }
            };

            let _ = tx.send(result);
        }))
    }

    /// Start message processing task
    #[must_use]
    pub fn start_processing(&self) -> Pin<Box<dyn Stream<Item = ProcessingEvent> + Send>> {
        processing::start_processing_stream(
            self.message_queue_rx.clone(),
            self.priority_queue_rx.clone(),
            self.subscribers.clone(),
            self.message_counter.clone(),
            self.priority_message_counter.clone(),
            self.processing_rate.clone(),
            self.processing_active.clone(),
        )
    }

    /// Stop processing task
    #[inline]
    pub fn stop_processing(&self) {
        self.processing_active.store(false, Ordering::Release);
    }

    /// Get current backpressure threshold
    #[inline]
    #[must_use]
    pub fn get_backpressure_threshold(&self) -> usize {
        self.backpressure_threshold.load(Ordering::Acquire)
    }

    /// Update backpressure threshold dynamically
    #[inline]
    pub fn set_backpressure_threshold(&self, threshold: usize) {
        self.backpressure_threshold
            .store(threshold, Ordering::Release);
    }

    /// Get comprehensive streaming statistics
    #[must_use]
    pub fn get_statistics(&self) -> StreamingStatistics {
        StreamingStatistics {
            total_messages: self.message_counter.load(Ordering::Acquire),
            priority_messages: self.priority_message_counter.load(Ordering::Acquire),
            active_subscribers: self.subscriber_counter.get(),
            total_subscribers: self.subscribers.len(),
            bytes_processed: self.bytes_processed.load(Ordering::Acquire),
            queue_size_limit: self.queue_size_limit.load(Ordering::Acquire),
            backpressure_events: self.backpressure_events.load(Ordering::Acquire),
            processing_rate: self.processing_rate.load(Ordering::Acquire),
            processing_active: self.processing_active.load(Ordering::Acquire),
        }
    }

    /// Subscribe to real-time events
    #[inline]
    #[must_use]
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<RealTimeEvent> {
        self.event_broadcaster.subscribe()
    }
}

impl Drop for LiveMessageStreamer {
    fn drop(&mut self) {
        self.stop_processing();
    }
}
