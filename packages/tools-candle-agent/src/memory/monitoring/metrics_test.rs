#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_metric_creation() {
        let counter = CounterMetric::new("test_counter", "Test counter metric");
        assert_eq!(counter.metric_type(), MetricType::Counter);
        
        // Test recording values
        counter.record(1.0);
        counter.record(2.5);
        
        // Should not panic
        let value = counter.value();
        match value {
            MetricValue::Counter(val) => assert!(val >= 0.0),
            _ => panic!("Expected Counter value")}
    }
    
    #[test]
    fn test_gauge_metric_creation() {
        let gauge = GaugeMetric::new("test_gauge", "Test gauge metric");
        assert_eq!(gauge.metric_type(), MetricType::Gauge);
        
        // Test recording values
        gauge.record(10.0);
        gauge.record(5.0);
        
        // Should not panic
        let value = gauge.value();
        match value {
            MetricValue::Gauge(_) => {}, // Value can be any float
            _ => panic!("Expected Gauge value")}
    }
    
    #[test]
    fn test_histogram_metric_creation() {
        let histogram = HistogramMetric::new("test_histogram", "Test histogram metric");
        assert_eq!(histogram.metric_type(), MetricType::Histogram);
        
        // Test recording values
        histogram.record(1.0);
        histogram.record(2.0);
        histogram.record(3.0);
        
        // Should not panic
        let value = histogram.value();
        match value {
            MetricValue::Histogram(val) => assert!(val >= 0.0),
            _ => panic!("Expected Histogram value")}
    }
    
    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();
        
        // Test adding metrics
        collector.register("counter1".to_string(), Box::new(CounterMetric::new("counter1", "Counter 1")));
        collector.register("gauge1".to_string(), Box::new(GaugeMetric::new("gauge1", "Gauge 1")));
        collector.register("histogram1".to_string(), Box::new(HistogramMetric::new("histogram1", "Histogram 1")));
        
        // Test recording values
        collector.record("counter1", 5.0);
        collector.record("gauge1", 10.0);
        collector.record("histogram1", 2.5);
        
        // Test collecting metrics
        let metrics = collector.collect();
        assert_eq!(metrics.len(), 3);
        assert!(metrics.contains_key("counter1"));
        assert!(metrics.contains_key("gauge1"));
        assert!(metrics.contains_key("histogram1"));
    }
}