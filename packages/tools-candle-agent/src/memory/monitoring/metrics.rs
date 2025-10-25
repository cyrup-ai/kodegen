//! Metrics collection and export

use std::collections::HashMap;

use log::{error, warn};
use prometheus::{self, Counter, Gauge, Histogram, HistogramOpts, Registry};

/// Metric types
#[derive(Debug, Clone, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(f64),
    Gauge(f64),
    Histogram(f64),
}

/// Metrics collector
pub struct MetricsCollector {
    /// Registered metrics
    metrics: HashMap<String, Box<dyn Metric>>,
}

impl MetricsCollector {
    /// Create a new collector
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    /// Register a metric
    pub fn register(&mut self, name: String, metric: Box<dyn Metric>) {
        self.metrics.insert(name, metric);
    }

    /// Record a value
    pub fn record(&self, name: &str, value: f64) {
        if let Some(metric) = self.metrics.get(name) {
            metric.record(value);
        }
    }

    /// Get all metrics
    pub fn collect(&self) -> HashMap<String, MetricValue> {
        self.metrics
            .iter()
            .map(|(name, metric)| (name.clone(), metric.value()))
            .collect()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Metric trait
pub trait Metric: Send + Sync {
    /// Record a value
    fn record(&self, value: f64);

    /// Get current value
    fn value(&self) -> MetricValue;

    /// Get metric type
    fn metric_type(&self) -> MetricType;
}

/// Counter metric wrapper
pub struct CounterMetric {
    counter: Counter,
    #[allow(dead_code)]
    registry: Registry,
}

impl CounterMetric {
    pub fn new(name: &str, help: &str) -> Self {
        let registry = Registry::new();
        let counter = Counter::new(name, help).unwrap_or_else(|e| {
            warn!(
                "Failed to create counter metric '{}': {}. Using fallback counter.",
                name, e
            );
            Counter::new("fallback_counter", "Fallback counter metric").unwrap_or_else(|e| {
                // If even the fallback fails, create a minimal counter with timestamp suffix
                warn!("Fallback counter creation failed: {}. Using timestamped minimal counter.", e);
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                Counter::new(format!("minimal_counter_{}", timestamp), "Minimal counter")
                    .unwrap_or_else(|e| {
                        error!("Critical: All counter creation attempts failed: {}. Creating no-op counter.", e);
                        // Return a counter that will work but may not record properly
                        Counter::new("emergency_counter", "Emergency fallback counter")
                            .unwrap_or_else(|_| {
                                // This should virtually never fail, but if it does, we panic with context
                                panic!("Critical system failure: Unable to create any counter metric. This indicates a severe prometheus/metrics system issue.")
                            })
                    })
            })
        });

        if let Err(e) = registry.register(Box::new(counter.clone())) {
            error!("Failed to register counter metric '{}': {}", name, e);
        }

        Self { counter, registry }
    }
}

impl Metric for CounterMetric {
    fn record(&self, value: f64) {
        self.counter.inc_by(value);
    }

    fn value(&self) -> MetricValue {
        MetricValue::Counter(self.counter.get())
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Counter
    }
}

/// Gauge metric wrapper
pub struct GaugeMetric {
    gauge: Gauge,
    #[allow(dead_code)]
    registry: Registry,
}

impl GaugeMetric {
    pub fn new(name: &str, help: &str) -> Self {
        let registry = Registry::new();
        let gauge = Gauge::new(name, help).unwrap_or_else(|e| {
            warn!(
                "Failed to create gauge metric '{}': {}. Using fallback gauge.",
                name, e
            );
            Gauge::new("fallback_gauge", "Fallback gauge metric").unwrap_or_else(|e| {
                // If even the fallback fails, create a minimal gauge with timestamp suffix
                warn!("Fallback gauge creation failed: {}. Using timestamped minimal gauge.", e);
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                Gauge::new(format!("minimal_gauge_{}", timestamp), "Minimal gauge")
                    .unwrap_or_else(|e| {
                        error!("Critical: All gauge creation attempts failed: {}. Creating emergency gauge.", e);
                        // Return a gauge that will work but may not record properly
                        Gauge::new("emergency_gauge", "Emergency fallback gauge")
                            .unwrap_or_else(|_| {
                                // This should virtually never fail, but if it does, we panic with context
                                panic!("Critical system failure: Unable to create any gauge metric. This indicates a severe prometheus/metrics system issue.")
                            })
                    })
            })
        });

        if let Err(e) = registry.register(Box::new(gauge.clone())) {
            error!("Failed to register gauge metric '{}': {}", name, e);
        }

        Self { gauge, registry }
    }
}

impl Metric for GaugeMetric {
    fn record(&self, value: f64) {
        self.gauge.set(value);
    }

    fn value(&self) -> MetricValue {
        MetricValue::Gauge(self.gauge.get())
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Gauge
    }
}

/// Histogram metric wrapper
pub struct HistogramMetric {
    histogram: Histogram,
    #[allow(dead_code)]
    registry: Registry,
}

impl HistogramMetric {
    pub fn new(name: &str, help: &str) -> Self {
        let registry = Registry::new();
        let histogram = Histogram::with_opts(HistogramOpts::new(name, help)).unwrap_or_else(|e| {
            warn!(
                "Failed to create histogram metric '{}': {}. Using fallback histogram.",
                name, e
            );
            Histogram::with_opts(HistogramOpts::new(
                "fallback_histogram",
                "Fallback histogram metric",
            ))
            .unwrap_or_else(|e| {
                // If even the fallback fails, create a minimal histogram with timestamp suffix
                warn!("Fallback histogram creation failed: {}. Using timestamped minimal histogram.", e);
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                Histogram::with_opts(HistogramOpts::new(format!("minimal_histogram_{}", timestamp), "Minimal histogram"))
                    .unwrap_or_else(|e| {
                        error!("Critical: All histogram creation attempts failed: {}. Creating emergency histogram.", e);
                        // Return a histogram that will work but may not record properly
                        Histogram::with_opts(HistogramOpts::new("emergency_histogram", "Emergency fallback histogram"))
                            .unwrap_or_else(|_| {
                                // This should virtually never fail, but if it does, we panic with context
                                panic!("Critical system failure: Unable to create any histogram metric. This indicates a severe prometheus/metrics system issue.")
                            })
                    })
            })
        });

        if let Err(e) = registry.register(Box::new(histogram.clone())) {
            error!("Failed to register histogram metric '{}': {}", name, e);
        }

        Self {
            histogram,
            registry,
        }
    }
}

impl Metric for HistogramMetric {
    fn record(&self, value: f64) {
        self.histogram.observe(value);
    }

    fn value(&self) -> MetricValue {
        // Return the sum for simplicity
        MetricValue::Histogram(self.histogram.get_sample_sum())
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Histogram
    }
}



