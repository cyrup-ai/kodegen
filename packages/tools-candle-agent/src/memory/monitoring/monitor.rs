//! Monitoring system for mem0-rs
//!
//! Provides Prometheus-based monitoring with comprehensive fallback strategies.

use log::warn;
use prometheus::{CounterVec, Gauge, GaugeVec, Histogram, HistogramVec, Registry};

use super::fallback::{
    create_bulletproof_counter_vec, create_bulletproof_gauge, create_bulletproof_gauge_vec,
    create_bulletproof_histogram, create_bulletproof_histogram_vec,
};

/// Monitoring system for mem0
pub struct Monitor {
    registry: Registry,

    // Counters
    pub memory_operations: CounterVec,
    pub api_requests: CounterVec,
    pub errors: CounterVec,

    // Gauges
    pub active_connections: Gauge,
    pub memory_count: GaugeVec,
    pub cache_size: Gauge,

    // Histograms
    pub operation_duration: HistogramVec,
    pub query_latency: Histogram,
    pub api_latency: HistogramVec,
}

impl Monitor {
    /// Create a new monitor instance
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        // Initialize counters
        let memory_operations = CounterVec::new(
            prometheus::Opts::new("memory_operations_total", "Total memory operations"),
            &["operation", "memory_type"],
        )?;
        registry.register(Box::new(memory_operations.clone()))?;

        let api_requests = CounterVec::new(
            prometheus::Opts::new("api_requests_total", "Total API requests"),
            &["method", "endpoint", "status"],
        )?;
        registry.register(Box::new(api_requests.clone()))?;

        let errors = CounterVec::new(
            prometheus::Opts::new("errors_total", "Total errors"),
            &["error_type", "component"],
        )?;
        registry.register(Box::new(errors.clone()))?;

        // Initialize gauges
        let active_connections = Gauge::new("active_connections", "Number of active connections")?;
        registry.register(Box::new(active_connections.clone()))?;

        let memory_count = GaugeVec::new(
            prometheus::Opts::new("memory_count", "Number of memories by type"),
            &["memory_type", "user_id"],
        )?;
        registry.register(Box::new(memory_count.clone()))?;

        let cache_size = Gauge::new("cache_size_bytes", "Cache size in bytes")?;
        registry.register(Box::new(cache_size.clone()))?;

        // Initialize histograms
        let operation_duration = HistogramVec::new(
            prometheus::HistogramOpts::new("operation_duration_seconds", "Operation duration"),
            &["operation", "memory_type"],
        )?;
        registry.register(Box::new(operation_duration.clone()))?;

        let query_latency = Histogram::with_opts(prometheus::HistogramOpts::new(
            "query_latency_seconds",
            "Query latency",
        ))?;
        registry.register(Box::new(query_latency.clone()))?;

        let api_latency = HistogramVec::new(
            prometheus::HistogramOpts::new("api_latency_seconds", "API endpoint latency"),
            &["method", "endpoint"],
        )?;
        registry.register(Box::new(api_latency.clone()))?;

        Ok(Self {
            registry,
            memory_operations,
            api_requests,
            errors,
            active_connections,
            memory_count,
            cache_size,
            operation_duration,
            query_latency,
            api_latency,
        })
    }

    /// Get the prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Record a memory operation
    pub fn record_memory_operation(&self, operation: &str, memory_type: &str) {
        self.memory_operations
            .with_label_values(&[operation, memory_type])
            .inc();
    }

    /// Record an API request
    pub fn record_api_request(&self, method: &str, endpoint: &str, status: u16) {
        self.api_requests
            .with_label_values(&[method, endpoint, &status.to_string()])
            .inc();
    }

    /// Record an error
    pub fn record_error(&self, error_type: &str, component: &str) {
        self.errors
            .with_label_values(&[error_type, component])
            .inc();
    }
}

impl Monitor {
    /// Create a new monitor instance with safe fallback when Prometheus initialization fails
    ///
    /// This method never panics. If Prometheus metrics cannot be initialized, it returns
    /// a monitor that silently discards all metrics operations without error.
    ///
    /// # Examples
    ///
    /// ```
    /// let monitor = Monitor::new_safe();  // Never panics
    /// monitor.record_operation("test", "test");  // Always works
    /// ```
    pub fn new_safe() -> Self {
        match Self::new() {
            Ok(monitor) => monitor,
            Err(e) => {
                warn!(
                    "Failed to create Prometheus monitor ({}), metrics will be silently discarded",
                    e
                );
                Self::create_disabled_monitor()
            }
        }
    }

    /// Create a disabled monitor that provides a compatible interface but doesn't collect metrics
    ///
    /// This is used as a fallback when Prometheus initialization fails completely.
    /// All metric operations will succeed but do nothing. This method NEVER panics or exits.
    ///
    /// Uses comprehensive fallback strategies to ensure reliability in all scenarios.
    #[inline]
    fn create_disabled_monitor() -> Self {
        use prometheus::Registry;

        // Create minimal registry for unregistered metrics (not used for export)
        let registry = Registry::new();

        // Create metrics using comprehensive fallback strategies
        // These methods are guaranteed to return working metrics or handle all failure cases gracefully
        let memory_operations = create_bulletproof_counter_vec("memory_operations");
        let api_requests = create_bulletproof_counter_vec("api_requests");
        let errors = create_bulletproof_counter_vec("errors");

        let active_connections = create_bulletproof_gauge("active_connections");
        let cache_size = create_bulletproof_gauge("cache_size");

        let memory_count = create_bulletproof_gauge_vec("memory_count");

        let query_latency = create_bulletproof_histogram("query_latency");

        let operation_duration = create_bulletproof_histogram_vec("operation_duration");
        let api_latency = create_bulletproof_histogram_vec("api_latency");

        Self {
            registry,
            memory_operations,
            api_requests,
            errors,
            active_connections,
            memory_count,
            cache_size,
            operation_duration,
            query_latency,
            api_latency,
        }
    }
}
