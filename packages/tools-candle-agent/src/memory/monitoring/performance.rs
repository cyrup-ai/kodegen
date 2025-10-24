//! Performance monitoring and profiling

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Performance monitoring errors
#[derive(Debug, thiserror::Error)]
pub enum PerformanceError {
    /// Recording error
    #[error("Failed to record performance metric: {0}")]
    RecordingError(String),
}

/// Result type for performance operations
pub type PerformanceResult<T> = Result<T, PerformanceError>;

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average response time
    pub avg_response_time_ms: f64,

    /// 95th percentile response time
    pub p95_response_time_ms: f64,

    /// 99th percentile response time
    pub p99_response_time_ms: f64,

    /// Throughput (operations per second)
    pub throughput: f64,

    /// Error rate
    pub error_rate: f64,

    /// Active connections
    pub active_connections: u64,
}

/// Performance monitor
pub struct PerformanceMonitor {
    /// Response times
    response_times: tokio::sync::RwLock<Vec<Duration>>,

    /// Error count
    error_count: std::sync::atomic::AtomicU64,

    /// Total requests
    total_requests: std::sync::atomic::AtomicU64,

    /// Monitor start time
    start_time: Instant,
}

impl PerformanceMonitor {
    /// Create a new monitor
    pub fn new() -> Self {
        Self {
            response_times: tokio::sync::RwLock::new(Vec::new()),
            error_count: std::sync::atomic::AtomicU64::new(0),
            total_requests: std::sync::atomic::AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Record a response time
    pub async fn record_response_time(&self, duration: Duration) -> PerformanceResult<()> {
        let mut times = self.response_times.write().await;
        times.push(duration);

        // Keep only recent times (last 1000)
        let len = times.len();
        if len > 1000 {
            times.drain(0..len - 1000);
        }

        self.total_requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Record an error
    pub fn record_error(&self) {
        self.error_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> PerformanceResult<PerformanceMetrics> {
        let times = self.response_times.read().await;
        let total_requests = self
            .total_requests
            .load(std::sync::atomic::Ordering::Relaxed);
        let error_count = self.error_count.load(std::sync::atomic::Ordering::Relaxed);

        // Calculate average
        let avg_response_time_ms = if times.is_empty() {
            0.0
        } else {
            let sum: Duration = times.iter().sum();
            sum.as_millis() as f64 / times.len() as f64
        };

        // Calculate percentiles
        let (p95, p99) = if times.is_empty() {
            (0.0, 0.0)
        } else {
            let mut sorted_times: Vec<_> = times.iter().map(|d| d.as_millis()).collect();
            sorted_times.sort();

            let p95_index = (sorted_times.len() as f64 * 0.95) as usize;
            let p99_index = (sorted_times.len() as f64 * 0.99) as usize;

            (
                sorted_times.get(p95_index).copied().unwrap_or(0) as f64,
                sorted_times.get(p99_index).copied().unwrap_or(0) as f64,
            )
        };

        // Calculate throughput
        let elapsed_secs = self.start_time.elapsed().as_secs_f64();
        let throughput = if elapsed_secs > 0.0 {
            total_requests as f64 / elapsed_secs
        } else {
            0.0
        };

        // Calculate error rate
        let error_rate = if total_requests > 0 {
            error_count as f64 / total_requests as f64
        } else {
            0.0
        };

        Ok(PerformanceMetrics {
            avg_response_time_ms,
            p95_response_time_ms: p95,
            p99_response_time_ms: p99,
            throughput,
            error_rate,
            active_connections: 0, // Would be tracked separately
        })
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance profiler for code sections
pub struct Profiler {
    /// Active timers
    timers: tokio::sync::RwLock<HashMap<String, Instant>>,

    /// Recorded durations
    durations: tokio::sync::RwLock<HashMap<String, Vec<Duration>>>,
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self {
            timers: tokio::sync::RwLock::new(HashMap::new()),
            durations: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Start a timer
    pub async fn start(&self, name: &str) -> PerformanceResult<()> {
        self.timers
            .write()
            .await
            .insert(name.to_string(), Instant::now());
        Ok(())
    }

    /// Stop a timer
    pub async fn stop(&self, name: &str) -> PerformanceResult<()> {
        let start = self.timers.write().await.remove(name);

        if let Some(start) = start {
            let duration = start.elapsed();
            self.durations
                .write()
                .await
                .entry(name.to_string())
                .or_insert_with(Vec::new)
                .push(duration);
        }
        Ok(())
    }

    /// Get profile report
    pub async fn report(&self) -> PerformanceResult<HashMap<String, ProfileStats>> {
        let durations = self.durations.read().await;

        Ok(durations
            .iter()
            .map(|(name, durations)| {
                let total: Duration = durations.iter().sum();
                let avg = total.as_micros() as f64 / durations.len() as f64;

                (
                    name.clone(),
                    ProfileStats {
                        count: durations.len(),
                        total_us: total.as_micros() as u64,
                        avg_us: avg as u64,
                    },
                )
            })
            .collect())
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Profile statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileStats {
    /// Number of times measured
    pub count: usize,

    /// Total time in microseconds
    pub total_us: u64,

    /// Average time in microseconds
    pub avg_us: u64,
}
