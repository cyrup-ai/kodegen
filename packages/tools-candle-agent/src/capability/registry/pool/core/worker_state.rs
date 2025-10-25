// worker_state.rs - Complete worker lifecycle management with state machine

use prometheus::{Counter, HistogramVec, IntGauge};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

/// Worker lifecycle states with atomic transitions
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Spawning = 0,   // Thread created, model loading
    Loading = 1,    // Model weights loading from disk
    Ready = 2,      // Ready to process requests
    Processing = 3, // Currently processing request
    Idle = 4,       // No requests for idle_threshold
    Evicting = 5,   // Shutdown signal sent
    Dead = 6,       // Thread terminated
    Failed = 7,     // Load or runtime failure
}

impl From<u32> for WorkerState {
    fn from(val: u32) -> Self {
        match val {
            0 => WorkerState::Spawning,
            1 => WorkerState::Loading,
            2 => WorkerState::Ready,
            3 => WorkerState::Processing,
            4 => WorkerState::Idle,
            5 => WorkerState::Evicting,
            6 => WorkerState::Dead,
            7 => WorkerState::Failed,
            _ => WorkerState::Dead,
        }
    }
}

/// Unified worker handle with complete lifecycle tracking
pub struct UnifiedWorkerHandle<Req, Resp> {
    pub worker_id: u64,
    pub registry_key: String,
    pub state: Arc<AtomicU32>,
    pub pending_requests: Arc<AtomicU64>,
    pub processed_requests: Arc<AtomicU64>,
    pub failed_requests: Arc<AtomicU64>,
    pub total_latency_us: Arc<AtomicU64>,
    pub last_activity: Arc<AtomicU64>,
    pub spawn_time: Instant,
    pub memory_mb: usize,
    pub cpu_cores: Option<Vec<usize>>, // CPU affinity

    // Channels
    pub request_tx: mpsc::UnboundedSender<Req>,
    pub response_rx: mpsc::UnboundedReceiver<Resp>,
    pub priority_tx: mpsc::UnboundedSender<Req>, // High-priority queue
    pub shutdown_tx: mpsc::UnboundedSender<()>,
    pub health_tx: mpsc::UnboundedSender<HealthCheck>,
    pub health_rx: mpsc::UnboundedReceiver<HealthStatus>,

    // Metrics
    pub metrics: WorkerMetrics,

    // Circuit breaker
    pub circuit_breaker: Arc<CircuitBreaker>,

    // Work stealing (placeholder for future async work stealing implementation)
    pub steal_handle: Option<()>,
}

/// Health check types
#[derive(Debug, Clone)]
pub enum HealthCheck {
    Ping,
    DeepCheck,    // Runs inference to verify model works
    MemoryCheck,  // Checks memory usage
    LatencyCheck, // Checks p99 latency
}

/// Health status response
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub state: WorkerState,
    pub queue_depth: usize,
    pub memory_usage_mb: usize,
    pub p50_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub error_rate: f64,
    pub last_error: Option<String>,
}

/// Worker-specific metrics
pub struct WorkerMetrics {
    pub request_counter: Counter,
    pub error_counter: Counter,
    pub latency_histogram: HistogramVec,
    pub queue_depth_gauge: IntGauge,
    pub memory_usage_gauge: IntGauge,
}

/// Circuit breaker for failure isolation
pub struct CircuitBreaker {
    state: Arc<AtomicU32>,
    failure_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    last_failure: Arc<AtomicU64>,
    config: CircuitBreakerConfig,
}

#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u64,
    pub success_threshold: u64,
    pub timeout: Duration,
    pub half_open_requests: u64,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(AtomicU32::new(0)), // 0=closed, 1=open, 2=half-open
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            last_failure: Arc::new(AtomicU64::new(0)),
            config,
        }
    }

    pub fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        let success = self.success_count.load(Ordering::Relaxed);

        // Close circuit if enough successes in half-open state
        if self.state.load(Ordering::Acquire) == 2 && success >= self.config.success_threshold {
            self.state.store(0, Ordering::Release);
            self.failure_count.store(0, Ordering::Release);
        }
    }

    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        let failures = self.failure_count.load(Ordering::Relaxed);

        if failures >= self.config.failure_threshold {
            self.state.store(1, Ordering::Release); // Open circuit
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_else(|e| {
                    log::warn!("System clock before Unix epoch ({}), using epoch time as fallback", e);
                    std::time::Duration::from_secs(0)
                })
                .as_secs();
            self.last_failure.store(timestamp, Ordering::Release);
        }
    }

    pub fn can_request(&self) -> bool {
        match self.state.load(Ordering::Acquire) {
            0 => true, // Closed - allow all
            1 => {
                // Open - check timeout
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_else(|e| {
                        log::warn!("System clock before Unix epoch ({}), using epoch time as fallback", e);
                        std::time::Duration::from_secs(0)
                    })
                    .as_secs();
                let last = self.last_failure.load(Ordering::Relaxed);

                if now - last > self.config.timeout.as_secs() {
                    // Try half-open
                    self.state.store(2, Ordering::Release);
                    self.success_count.store(0, Ordering::Release);
                    true
                } else {
                    false
                }
            }
            2 => {
                // Half-open - limited requests
                self.success_count.load(Ordering::Relaxed) < self.config.half_open_requests
            }
            _ => false,
        }
    }
}

impl<Req, Resp> UnifiedWorkerHandle<Req, Resp> {
    /// Transition to new state with validation
    pub fn transition_state(&self, new_state: WorkerState) -> Result<(), String> {
        let current = WorkerState::from(self.state.load(Ordering::Acquire));

        // Validate state transition
        let valid = match (current, new_state) {
            (WorkerState::Spawning, WorkerState::Loading) => true,
            (WorkerState::Loading, WorkerState::Ready) => true,
            (WorkerState::Loading, WorkerState::Failed) => true,
            (WorkerState::Ready, WorkerState::Processing) => true,
            (WorkerState::Processing, WorkerState::Ready) => true,
            (WorkerState::Ready, WorkerState::Idle) => true,
            (WorkerState::Idle, WorkerState::Ready) => true,
            (WorkerState::Idle, WorkerState::Evicting) => true,
            (_, WorkerState::Evicting) => true, // Can always evict
            (WorkerState::Evicting, WorkerState::Dead) => true,
            (WorkerState::Failed, WorkerState::Dead) => true,
            _ => false,
        };

        if valid {
            self.state.store(new_state as u32, Ordering::Release);
            self.update_activity();
            Ok(())
        } else {
            Err(format!(
                "Invalid transition: {:?} -> {:?}",
                current, new_state
            ))
        }
    }

    #[inline]
    pub fn update_activity(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|e| {
                log::warn!("System clock before Unix epoch ({}), using epoch time as fallback", e);
                std::time::Duration::from_secs(0)
            })
            .as_secs();
        self.last_activity.store(now, Ordering::Release);
    }

    pub fn is_alive(&self) -> bool {
        let state = WorkerState::from(self.state.load(Ordering::Acquire));
        !matches!(
            state,
            WorkerState::Dead | WorkerState::Failed | WorkerState::Evicting
        )
    }

    pub fn can_accept_requests(&self) -> bool {
        let state = WorkerState::from(self.state.load(Ordering::Acquire));
        matches!(state, WorkerState::Ready | WorkerState::Processing)
            && self.circuit_breaker.can_request()
    }

    pub fn get_load_score(&self) -> f64 {
        let pending = self.pending_requests.load(Ordering::Relaxed) as f64;
        let error_rate = self.calculate_error_rate();
        let latency = self.get_avg_latency_ms();

        // Weighted load score (lower is better)
        pending * 1.0 + error_rate * 100.0 + latency * 0.1
    }

    fn calculate_error_rate(&self) -> f64 {
        let total = self.processed_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);

        if total == 0 {
            0.0
        } else {
            (failed as f64) / (total as f64)
        }
    }

    fn get_avg_latency_ms(&self) -> f64 {
        let total = self.processed_requests.load(Ordering::Relaxed);
        let latency_us = self.total_latency_us.load(Ordering::Relaxed);

        if total == 0 {
            0.0
        } else {
            (latency_us as f64) / (total as f64) / 1000.0
        }
    }
}
