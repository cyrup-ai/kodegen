// Tests extracted from src/memory/monitoring/metrics.rs

use kodegen_candle_agent::memory::monitoring::metrics::{
    CounterMetric, GaugeMetric, HistogramMetric, Metric, MetricsCollector,
    MetricType, MetricValue,
};

#[test]
fn test_counter_metric_creation() {
    let counter = CounterMetric::new("test_counter", "Test counter metric");
    assert!(matches!(counter.metric_type(), MetricType::Counter));
    
    counter.record(1.0);
    counter.record(2.5);
    
    let value = counter.value();
    match value {
        MetricValue::Counter(val) => assert!(val >= 0.0),
        _ => panic!("Expected Counter value"),
    }
}

#[test]
fn test_gauge_metric_creation() {
    let gauge = GaugeMetric::new("test_gauge", "Test gauge metric");
    assert!(matches!(gauge.metric_type(), MetricType::Gauge));
    
    gauge.record(10.0);
    gauge.record(5.0);
    
    let value = gauge.value();
    match value {
        MetricValue::Gauge(_) => {},
        _ => panic!("Expected Gauge value"),
    }
}

#[test]
fn test_histogram_metric_creation() {
    let histogram = HistogramMetric::new("test_histogram", "Test histogram metric");
    assert!(matches!(histogram.metric_type(), MetricType::Histogram));
    
    histogram.record(1.0);
    histogram.record(2.0);
    histogram.record(3.0);
    
    let value = histogram.value();
    match value {
        MetricValue::Histogram(val) => assert!(val >= 0.0),
        _ => panic!("Expected Histogram value"),
    }
}

#[test]
fn test_metrics_collector() {
    let mut collector = MetricsCollector::new();
    
    collector.register(
        "counter1".to_string(),
        Box::new(CounterMetric::new("counter1", "Counter 1")),
    );
    collector.register(
        "gauge1".to_string(),
        Box::new(GaugeMetric::new("gauge1", "Gauge 1")),
    );
    collector.register(
        "histogram1".to_string(),
        Box::new(HistogramMetric::new("histogram1", "Histogram 1")),
    );
    
    collector.record("counter1", 5.0);
    collector.record("gauge1", 10.0);
    collector.record("histogram1", 2.5);
    
    let metrics = collector.collect();
    assert_eq!(metrics.len(), 3);
    assert!(metrics.contains_key("counter1"));
    assert!(metrics.contains_key("gauge1"));
    assert!(metrics.contains_key("histogram1"));
}
