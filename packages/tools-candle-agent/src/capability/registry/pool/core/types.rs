use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::debug;

/// Health check ping sent to worker
#[derive(Debug, Clone, Copy)]
pub struct HealthPing;

/// Health check response from worker
#[derive(Debug, Clone)]
pub struct HealthPong {
    pub worker_id: usize,
    pub timestamp: u64,
    pub queue_depth: usize,
}

/// Configuration for pool behavior
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub request_timeout_secs: u64,      // Default: 21600 (6 hours)
    pub shutdown_timeout_secs: u64,     // Default: 5
    pub maintenance_interval_secs: u64, // Default: 60 (1 minute)
    pub cooldown_idle_minutes: u64,     // Default: 1
    pub max_workers_per_model: usize,   // Default: 4 (adaptive scaling limit)

    // Channel capacities (bounded to prevent OOM)
    pub embed_queue_capacity: usize,       // Default: 100
    pub batch_queue_capacity: usize,       // Default: 50
    pub prompt_queue_capacity: usize,      // Default: 100 (text_to_text)
    pub image_gen_queue_capacity: usize,   // Default: 20  (text_to_image)
    pub vision_queue_capacity: usize,      // Default: 50  (vision)
    pub image_embed_queue_capacity: usize, // Default: 50  (image_embedding)
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            // 6 hour timeout allows for large model downloads (e.g., Llama 70B ~40GB)
            // on slow connections without premature failures. Matches worker spawn timeout.
            request_timeout_secs: 6 * 3600, // 21600 seconds = 6 hours
            shutdown_timeout_secs: 5,
            maintenance_interval_secs: 60,
            cooldown_idle_minutes: 1,
            max_workers_per_model: 4,

            // Channel capacities (bounded to prevent OOM)
            embed_queue_capacity: 100,
            batch_queue_capacity: 50,
            prompt_queue_capacity: 100,
            image_gen_queue_capacity: 20, // Image gen is slower, smaller queue
            vision_queue_capacity: 50,
            image_embed_queue_capacity: 50,
        }
    }
}

/// Per-model latency metrics (thread-safe atomic tracking)
#[derive(Debug, Default)]
pub struct ModelLatencyMetrics {
    pub latency_sum_ms: AtomicU64, // Sum for avg calculation
    pub latency_count: AtomicU64,  // Request count for avg
    pub latency_max_ms: AtomicU64, // Peak latency
    pub latency_min_ms: AtomicU64, // Minimum latency (init to u64::MAX)
}

impl ModelLatencyMetrics {
    pub fn new() -> Self {
        Self {
            latency_sum_ms: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            latency_max_ms: AtomicU64::new(0),
            latency_min_ms: AtomicU64::new(u64::MAX),
        }
    }
}

/// Metrics tracked per pool
#[derive(Debug, Default)]
pub struct PoolMetrics {
    pub total_requests: AtomicUsize,
    pub total_timeouts: AtomicUsize,
    pub total_errors: AtomicUsize,
    pub workers_spawned: AtomicUsize,
    pub workers_evicted: AtomicUsize,
    pub circuit_rejections: AtomicUsize,

    // Per-model latency tracking
    pub per_model_latency: DashMap<String, ModelLatencyMetrics>,
}

impl PoolMetrics {
    /// Record request completion for metrics tracking
    ///
    /// Updates both global and per-model metrics atomically.
    /// Call this after every request completes (success or failure).
    pub fn record_request(&self, registry_key: &str, duration: Duration, success: bool) {
        // Update global counter
        self.total_requests.fetch_add(1, Ordering::Release);

        if !success {
            self.total_errors.fetch_add(1, Ordering::Release);
        }

        // Update per-model latency metrics
        let latency_ms = duration.as_millis() as u64;
        let metrics = self
            .per_model_latency
            .entry(registry_key.to_string())
            .or_default();

        metrics
            .latency_sum_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        metrics.latency_count.fetch_add(1, Ordering::Relaxed);

        // Update max latency (CAS loop for atomicity)
        let mut current_max = metrics.latency_max_ms.load(Ordering::Relaxed);
        while latency_ms > current_max {
            match metrics.latency_max_ms.compare_exchange_weak(
                current_max,
                latency_ms,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    /// Get average latency for a model
    ///
    /// Returns None if no requests processed yet.
    pub fn get_avg_latency(&self, registry_key: &str) -> Option<f64> {
        self.per_model_latency.get(registry_key).and_then(|m| {
            let sum = m.latency_sum_ms.load(Ordering::Acquire);
            let count = m.latency_count.load(Ordering::Acquire);
            if count > 0 {
                Some((sum as f64) / (count as f64))
            } else {
                None
            }
        })
    }

    /// Export all metrics in Prometheus text format
    ///
    /// Returns metrics formatted for Prometheus scraping.
    /// Call this from HTTP /metrics endpoint handler.
    pub async fn get_prometheus_metrics<W>(&self, pool: &super::Pool<W>) -> String
    where
        W: PoolWorkerHandle,
    {
        let mut output = String::with_capacity(4096);

        // Global metrics
        output.push_str("# HELP pool_requests_total Total requests across all models\n");
        output.push_str("# TYPE pool_requests_total counter\n");
        output.push_str(&format!(
            "pool_requests_total {}\n",
            self.total_requests.load(Ordering::Acquire)
        ));

        output.push_str("# HELP pool_errors_total Total errors (timeouts + failures)\n");
        output.push_str("# TYPE pool_errors_total counter\n");
        output.push_str(&format!(
            "pool_errors_total {}\n",
            self.total_errors.load(Ordering::Acquire)
        ));

        output.push_str("# HELP pool_timeouts_total Total request timeouts\n");
        output.push_str("# TYPE pool_timeouts_total counter\n");
        output.push_str(&format!(
            "pool_timeouts_total {}\n",
            self.total_timeouts.load(Ordering::Acquire)
        ));

        output.push_str("# HELP pool_workers_spawned_total Total workers spawned\n");
        output.push_str("# TYPE pool_workers_spawned_total counter\n");
        output.push_str(&format!(
            "pool_workers_spawned_total {}\n",
            self.workers_spawned.load(Ordering::Acquire)
        ));

        output.push_str("# HELP pool_workers_evicted_total Total workers evicted\n");
        output.push_str("# TYPE pool_workers_evicted_total counter\n");
        output.push_str(&format!(
            "pool_workers_evicted_total {}\n",
            self.workers_evicted.load(Ordering::Acquire)
        ));

        output.push_str("# HELP pool_circuit_rejections_total Total circuit breaker rejections\n");
        output.push_str("# TYPE pool_circuit_rejections_total counter\n");
        output.push_str(&format!(
            "pool_circuit_rejections_total {}\n",
            self.circuit_rejections.load(Ordering::Acquire)
        ));

        // Per-model metrics
        output.push_str("# HELP pool_model_requests_total Requests per model\n");
        output.push_str("# TYPE pool_model_requests_total counter\n");
        for entry in self.per_model_latency.iter() {
            let (model, metrics) = (entry.key(), entry.value());
            let count = metrics.latency_count.load(Ordering::Acquire);
            output.push_str(&format!(
                "pool_model_requests_total{{model=\"{}\"}} {}\n",
                model, count
            ));
        }

        output.push_str("# HELP pool_model_latency_avg_ms Average latency per model\n");
        output.push_str("# TYPE pool_model_latency_avg_ms gauge\n");
        for entry in self.per_model_latency.iter() {
            let (model, metrics) = (entry.key(), entry.value());
            let sum = metrics.latency_sum_ms.load(Ordering::Acquire);
            let count = metrics.latency_count.load(Ordering::Acquire);
            if count > 0 {
                output.push_str(&format!(
                    "pool_model_latency_avg_ms{{model=\"{}\"}} {:.2}\n",
                    model,
                    (sum as f64) / (count as f64)
                ));
            }
        }

        output.push_str("# HELP pool_model_latency_max_ms Peak latency per model\n");
        output.push_str("# TYPE pool_model_latency_max_ms gauge\n");
        for entry in self.per_model_latency.iter() {
            let (model, metrics) = (entry.key(), entry.value());
            let max_ms = metrics.latency_max_ms.load(Ordering::Acquire);
            output.push_str(&format!(
                "pool_model_latency_max_ms{{model=\"{}\"}} {}\n",
                model, max_ms
            ));
        }

        output.push_str("# HELP pool_model_workers Active workers per model\n");
        output.push_str("# TYPE pool_model_workers gauge\n");
        for entry in pool.workers().iter() {
            let (model, workers) = (entry.key(), entry.value());
            output.push_str(&format!(
                "pool_model_workers{{model=\"{}\"}} {}\n",
                model,
                workers.len()
            ));
        }

        // Memory metrics
        let memory_stats = pool.memory_governor.get_stats().await;
        output.push_str("# HELP pool_memory_used_mb Memory used by workers\n");
        output.push_str("# TYPE pool_memory_used_mb gauge\n");
        output.push_str(&format!(
            "pool_memory_used_mb {}\n",
            memory_stats.allocated_mb
        ));

        output.push_str("# HELP pool_memory_limit_mb Memory limit\n");
        output.push_str("# TYPE pool_memory_limit_mb gauge\n");
        output.push_str(&format!("pool_memory_limit_mb {}\n", memory_stats.limit_mb));

        output.push_str("# HELP pool_memory_pressure Memory pressure level (0-3)\n");
        output.push_str("# TYPE pool_memory_pressure gauge\n");
        let pressure_value = match memory_stats.pressure {
            super::memory_governor::MemoryPressure::Low => 0,
            super::memory_governor::MemoryPressure::Normal => 1,
            super::memory_governor::MemoryPressure::High => 2,
            super::memory_governor::MemoryPressure::Critical => 3,
        };
        output.push_str(&format!("pool_memory_pressure {}\n", pressure_value));

        output
    }
}

/// Handle to a worker thread (capability-specific channels defined in capabilities/)
#[derive(Debug, Clone)]
pub struct WorkerHandle {
    pub pending_requests: Arc<AtomicUsize>,
    pub last_used: Arc<AtomicU64>,
    pub worker_id: usize,
    pub shutdown_tx: mpsc::UnboundedSender<()>,
    pub per_worker_mb: usize,
    pub health_tx: mpsc::UnboundedSender<HealthPing>,
    pub health_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<HealthPong>>>,

    // NEW: Add state tracking
    pub state: Arc<AtomicU32>, // WorkerState as u32
}

impl WorkerHandle {
    pub fn new(
        worker_id: usize,
        shutdown_tx: mpsc::UnboundedSender<()>,
        per_worker_mb: usize,
        health_tx: mpsc::UnboundedSender<HealthPing>,
        health_rx: mpsc::UnboundedReceiver<HealthPong>,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            pending_requests: Arc::new(AtomicUsize::new(0)),
            last_used: Arc::new(AtomicU64::new(now)),
            worker_id,
            shutdown_tx,
            per_worker_mb,
            health_tx,
            health_rx: Arc::new(tokio::sync::Mutex::new(health_rx)),
            state: Arc::new(AtomicU32::new(0)), // Start in Spawning state
        }
    }

    pub fn touch(&self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_used
            .store(now, std::sync::atomic::Ordering::Release);
    }

    /// Check if worker is alive by sending health ping
    ///
    /// Returns true if worker responds, false otherwise.
    /// False indicates worker thread is dead, stuck, or channel broken.
    ///
    /// Note: Uses try_recv for non-blocking check.
    pub fn is_alive(&self) -> bool {
        // Try to send ping
        if self.health_tx.send(HealthPing).is_err() {
            // Channel broken = worker dead
            return false;
        }

        // Try to receive pong (non-blocking)
        if let Ok(mut rx_guard) = self.health_rx.try_lock() {
            match rx_guard.try_recv() {
                Ok(pong) => {
                    // Update last health check timestamp
                    self.last_used
                        .store(pong.timestamp, std::sync::atomic::Ordering::Release);
                    true
                }
                Err(_) => {
                    // No pong received yet - check timestamp staleness
                    use std::time::{SystemTime, UNIX_EPOCH};

                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    let last_check = self.last_used.load(std::sync::atomic::Ordering::Acquire);
                    let staleness_secs = now.saturating_sub(last_check);

                    // Health check timeout: 6 hours (matches request timeout)
                    // Allows for large model downloads on slow connections
                    // If worker hasn't responded in 6 hours, consider it dead
                    const HEALTH_TIMEOUT_SECS: u64 = 6 * 3600; // 21600 seconds

                    if staleness_secs > HEALTH_TIMEOUT_SECS {
                        // Worker is unresponsive - mark as dead
                        self.set_state(super::worker_state::WorkerState::Dead);
                        false
                    } else {
                        // Worker may be busy processing - give it time
                        true
                    }
                }
            }
        } else {
            // Lock contention - check staleness without lock
            use std::time::{SystemTime, UNIX_EPOCH};

            // First check if state is already Dead/Failed
            let current_state = self.get_state();
            if matches!(
                current_state,
                super::worker_state::WorkerState::Dead | super::worker_state::WorkerState::Failed
            ) {
                return false;
            }

            // Check timestamp staleness (atomic read, no lock needed)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let last_check = self.last_used.load(std::sync::atomic::Ordering::Acquire);
            let staleness_secs = now.saturating_sub(last_check);

            // Health check timeout: 6 hours (matches request timeout)
            const HEALTH_TIMEOUT_SECS: u64 = 6 * 3600; // 21600 seconds

            if staleness_secs > HEALTH_TIMEOUT_SECS {
                self.set_state(super::worker_state::WorkerState::Dead);
                false
            } else {
                true
            }
        }
    }

    /// Get current worker state
    pub fn get_state(&self) -> super::worker_state::WorkerState {
        use super::worker_state::WorkerState;
        let state_val = self.state.load(Ordering::Acquire);
        WorkerState::from(state_val)
    }

    /// Set worker state (atomic)
    pub fn set_state(&self, new_state: super::worker_state::WorkerState) {
        self.state.store(new_state as u32, Ordering::Release);
    }

    /// Check if worker can accept requests
    pub fn can_accept_requests(&self) -> bool {
        use super::worker_state::WorkerState;
        matches!(
            self.get_state(),
            WorkerState::Ready | WorkerState::Processing | WorkerState::Idle
        )
    }

    /// Check if worker should be evicted
    pub fn is_evictable(&self) -> bool {
        use super::worker_state::WorkerState;
        matches!(self.get_state(), WorkerState::Ready | WorkerState::Idle)
    }
}

/// Select worker using Power of Two Choices algorithm (O(1) instead of O(n))
///
/// Algorithm:
/// - 0 workers: None
/// - 1 worker: Return that worker
/// - 2+ workers: Sample 2 random workers, return least loaded
///
/// This achieves O(log log n) load imbalance vs O(log n) for random selection,
/// while maintaining O(1) time complexity vs O(n) for full scan.
///
/// # Performance
/// - 2 atomic loads instead of N
/// - No cache thrashing from full iteration
/// - Scalable to 100+ workers with no degradation
///
/// # Usage
/// Works with any worker handle type that has a `core: WorkerHandle` field:
/// ```ignore
/// let worker = select_worker_power_of_two(&workers, |w| &w.core)?;
/// ```
pub fn select_worker_power_of_two<'a, T, F>(workers: &'a [T], get_core: F) -> Option<&'a T>
where
    F: Fn(&'a T) -> &'a WorkerHandle,
{
    match workers.len() {
        0 => None,
        1 => Some(&workers[0]),
        len => {
            // Sample 2 random indices
            let idx1 = fastrand::usize(..len);
            let mut idx2 = fastrand::usize(..len);

            // Ensure idx2 != idx1 (unlikely but possible)
            while idx2 == idx1 && len > 1 {
                idx2 = fastrand::usize(..len);
            }

            let w1 = &workers[idx1];
            let w2 = &workers[idx2];

            // Compare pending requests (only 2 atomic loads!)
            let core1 = get_core(w1);
            let core2 = get_core(w2);
            let load1 = core1
                .pending_requests
                .load(std::sync::atomic::Ordering::Relaxed);
            let load2 = core2
                .pending_requests
                .load(std::sync::atomic::Ordering::Relaxed);

            // Return least loaded
            if load1 <= load2 { Some(w1) } else { Some(w2) }
        }
    }
}

/// RAII guard that prevents duplicate worker spawning
///
/// Automatically releases spawn lock when dropped, even if panic occurs.
/// Only one thread can hold a SpawnGuard for a given registry_key at a time.
pub struct SpawnGuard {
    flag: Arc<std::sync::atomic::AtomicBool>,
    registry_key: String,
}

impl SpawnGuard {
    pub(crate) fn new(flag: Arc<std::sync::atomic::AtomicBool>, registry_key: String) -> Self {
        Self { flag, registry_key }
    }
}

impl Drop for SpawnGuard {
    fn drop(&mut self) {
        // Release spawn lock when guard is dropped
        self.flag.store(false, std::sync::atomic::Ordering::Release);
        debug!("Released spawn lock for {}", self.registry_key);
    }
}

/// RAII guard that automatically decrements pending_requests counter on drop
///
/// Prevents counter leaks when requests fail or panic. Follows the same
/// pattern as SpawnGuard and AllocationGuard for consistent resource cleanup.
pub(crate) struct PendingRequestsGuard {
    counter: Arc<AtomicUsize>,
}

impl PendingRequestsGuard {
    pub(crate) fn new(counter: &Arc<AtomicUsize>) -> Self {
        Self {
            counter: counter.clone(),
        }
    }
}

impl Drop for PendingRequestsGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Release);
        debug!("Decremented pending_requests via guard");
    }
}

/// Trait for capability-specific worker handles
///
/// All worker handles (TextEmbeddingWorkerHandle, TextToTextWorkerHandle, etc.)
/// implement this trait to provide unified access to core WorkerHandle fields.
pub trait PoolWorkerHandle: Send + Sync + 'static {
    /// Access core WorkerHandle (pending_requests, last_used, etc.)
    fn core(&self) -> &WorkerHandle;

    /// Mutable access to core WorkerHandle
    fn core_mut(&mut self) -> &mut WorkerHandle;

    /// Registry key for this worker (model identifier)
    fn registry_key(&self) -> &str;
}

/// Pool-level health status for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct PoolHealth {
    pub status: HealthStatusLevel,
    pub models: Vec<ModelHealth>,
    pub memory: MemoryHealth,
    pub timestamp: u64,
}

/// Health status levels
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum HealthStatusLevel {
    Healthy,   // All models operational
    Degraded,  // Some models at capacity or high load
    Unhealthy, // Models down or critical errors
}

/// Per-model health information
#[derive(Debug, Clone, Serialize)]
pub struct ModelHealth {
    pub registry_key: String,
    pub status: HealthStatusLevel,
    pub workers: WorkerHealthStats,
    pub queue_depth: usize,
    pub avg_latency_ms: Option<f64>,
}

/// Worker statistics for health check
#[derive(Debug, Clone, Serialize)]
pub struct WorkerHealthStats {
    pub total: usize,
    pub busy: usize,
    pub idle: usize,
}

/// Memory health information
#[derive(Debug, Clone, Serialize)]
pub struct MemoryHealth {
    pub used_mb: u64,
    pub limit_mb: u64,
    pub available_mb: u64,
    pub pressure: String,
    pub utilization: f64,
}
