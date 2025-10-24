//! Message processing and distribution

use crossbeam_skiplist::SkipMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::Stream;

use super::results::ProcessingEvent;
use super::subscriber::StreamSubscriber;
use super::types::LiveUpdateMessage;

/// Start the message processing stream
#[allow(clippy::too_many_arguments)]
pub(crate) fn start_processing_stream(
    message_queue_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<LiveUpdateMessage>>>,
    priority_queue_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<LiveUpdateMessage>>>,
    subscribers: Arc<SkipMap<String, Arc<StreamSubscriber>>>,
    message_counter: Arc<AtomicUsize>,
    priority_message_counter: Arc<AtomicUsize>,
    processing_rate: Arc<AtomicU64>,
    processing_active: Arc<AtomicBool>,
) -> Pin<Box<dyn Stream<Item = ProcessingEvent> + Send>> {
    Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
        tokio::spawn(async move {
            let mut messages_processed = 0u64;
            let mut last_rate_check = std::time::Instant::now();

            loop {
                let target_rate = processing_rate.load(Ordering::Acquire);
                let delay_nanos = if target_rate > 0 {
                    1_000_000_000 / target_rate
                } else {
                    1_000_000 // 1ms default
                };

                // Process priority messages first - use timeout to avoid blocking
                let mut message = {
                    let mut rx = priority_queue_rx.lock().await;
                    match tokio::time::timeout(Duration::from_micros(100), rx.recv()).await {
                        Ok(Some(priority_msg)) => {
                            priority_message_counter.fetch_sub(1, Ordering::AcqRel);
                            Some(priority_msg)
                        }
                        _ => None,
                    }
                };

                // If no priority message, check normal queue
                if message.is_none() {
                    let mut rx = message_queue_rx.lock().await;
                    if let Ok(Some(normal_msg)) =
                        tokio::time::timeout(Duration::from_micros(100), rx.recv()).await
                    {
                        message_counter.fetch_sub(1, Ordering::AcqRel);
                        message = Some(normal_msg);
                    }
                }

                if let Some(message) = message {
                    let mut delivered_count = 0u64;
                    let mut total_bytes = 0u64;

                    // Distribute to matching subscribers
                    let mut disconnected = Vec::new();

                    for entry in subscribers.iter() {
                        let subscriber = entry.value();
                        if subscriber.should_receive(&message) {
                            // Actually send the message to subscriber's channel
                            if subscriber.send_message(message.clone()) {
                                subscriber.record_delivery(&message);
                                delivered_count += 1;
                                total_bytes += u64::from(message.size_bytes);
                            } else {
                                // Channel closed - subscriber disconnected
                                disconnected.push(entry.key().clone());
                            }
                        }
                    }

                    // Remove disconnected subscribers
                    for id in disconnected {
                        subscribers.remove(&id);
                    }

                    messages_processed += 1;

                    // Emit processing event
                    let event = ProcessingEvent::MessageProcessed {
                        message_id: message.id,
                        sequence_number: message.sequence_number,
                        delivered_count,
                        total_bytes,
                        priority: message.priority,
                    };
                    let _ = tx.send(event);

                    // Rate limiting with nanosecond precision
                    tokio::time::sleep(Duration::from_nanos(delay_nanos)).await;
                } else {
                    // No messages, sleep briefly
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }

                // Report processing rate periodically
                if last_rate_check.elapsed() >= Duration::from_secs(10) {
                    let elapsed_secs = last_rate_check.elapsed().as_secs_f64();
                    // Use f64 for rate calculation - precision loss is acceptable for metrics
                    let rate = f64::from(u32::try_from(messages_processed).unwrap_or(u32::MAX))
                        / elapsed_secs;

                    let event = ProcessingEvent::RateReport {
                        messages_per_second: rate,
                        messages_processed,
                        active_subscribers: subscribers.len() as u64,
                    };
                    let _ = tx.send(event);

                    messages_processed = 0;
                    last_rate_check = std::time::Instant::now();
                }

                // Check if we should continue
                if !processing_active.load(Ordering::Acquire) {
                    break;
                }
            }
        });
    }))
}
