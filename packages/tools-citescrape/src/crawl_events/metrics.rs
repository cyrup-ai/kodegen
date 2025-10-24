use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Metrics for event bus operations using lock-free atomic operations.
///
/// All counters use `Ordering::SeqCst` for sequential consistency,
/// ensuring snapshot reads are coherent across all fields.
#[derive(Debug, Clone)]
pub struct EventBusMetrics {
    pub events_published: Arc<AtomicU64>,
    pub events_dropped: Arc<AtomicU64>,
    pub events_failed: Arc<AtomicU64>,
    pub active_subscribers: Arc<AtomicUsize>,
    pub peak_subscribers: Arc<AtomicUsize>,
}

impl EventBusMetrics {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events_published: Arc::new(AtomicU64::new(0)),
            events_dropped: Arc::new(AtomicU64::new(0)),
            events_failed: Arc::new(AtomicU64::new(0)),
            active_subscribers: Arc::new(AtomicUsize::new(0)),
            peak_subscribers: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn increment_published(&self) {
        self.events_published.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_dropped(&self) {
        self.events_dropped.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_failed(&self) {
        self.events_failed.fetch_add(1, Ordering::SeqCst);
    }

    pub fn update_subscriber_count(&self, count: usize) {
        self.active_subscribers.store(count, Ordering::SeqCst);
        let _ = self.peak_subscribers.fetch_max(count, Ordering::SeqCst);
    }

    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            events_published: self.events_published.load(Ordering::SeqCst),
            events_dropped: self.events_dropped.load(Ordering::SeqCst),
            events_failed: self.events_failed.load(Ordering::SeqCst),
            active_subscribers: self.active_subscribers.load(Ordering::SeqCst),
            peak_subscribers: self.peak_subscribers.load(Ordering::SeqCst),
        }
    }

    pub fn reset(&self) {
        self.events_published.store(0, Ordering::SeqCst);
        self.events_dropped.store(0, Ordering::SeqCst);
        self.events_failed.store(0, Ordering::SeqCst);
        self.active_subscribers.store(0, Ordering::SeqCst);
        self.peak_subscribers.store(0, Ordering::SeqCst);
    }
}

impl Default for EventBusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MetricsSnapshot {
    pub events_published: u64,
    pub events_dropped: u64,
    pub events_failed: u64,
    pub active_subscribers: usize,
    pub peak_subscribers: usize,
}

impl MetricsSnapshot {
    #[must_use]
    pub fn total_events(&self) -> u64 {
        self.events_published + self.events_dropped + self.events_failed
    }

    #[must_use]
    pub fn success_rate(&self) -> f64 {
        let total = self.total_events();
        if total == 0 {
            return 1.0;
        }
        self.events_published as f64 / total as f64
    }
}
