use kodegen_simd::similarity::metrics::{MetricsGuard, SimilarityMetrics};
use std::sync::Arc;

#[test]
fn test_metrics_collection() {
    let metrics = Arc::new(SimilarityMetrics::default());

    {
        let _guard = MetricsGuard::new(&metrics, 8);
    }

    let snapshot = metrics.get_metrics();
    assert_eq!(snapshot.total_calculations, 1);
    assert_eq!(snapshot.total_elements_processed, 8);
    assert_eq!(snapshot.average_vector_length(), 8.0);

    {
        let _guard = MetricsGuard::new(&metrics, 4);
    }

    let snapshot = metrics.get_metrics();
    assert_eq!(snapshot.total_calculations, 2);
    assert_eq!(snapshot.total_elements_processed, 12);
    assert_eq!(snapshot.average_vector_length(), 6.0);

    metrics.reset();
    let snapshot = metrics.get_metrics();
    assert_eq!(snapshot.total_calculations, 0);
    assert_eq!(snapshot.total_elements_processed, 0);
    assert_eq!(snapshot.average_vector_length(), 0.0);
}

#[test]
fn test_concurrent_metrics() {
    use std::thread;

    let metrics = Arc::new(SimilarityMetrics::default());

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let metrics = metrics.clone();
            thread::spawn(move || {
                for _ in 0..100 {
                    let _guard = MetricsGuard::new(&metrics, 16);
                }
            })
        })
        .collect();

    for handle in handles {
        if let Err(e) = handle.join() {
            panic!("Thread panicked during metrics test: {:?}", e);
        }
    }

    let snapshot = metrics.get_metrics();
    assert_eq!(snapshot.total_calculations, 1000);
    assert_eq!(snapshot.total_elements_processed, 16000);
    assert_eq!(snapshot.average_vector_length(), 16.0);
}
