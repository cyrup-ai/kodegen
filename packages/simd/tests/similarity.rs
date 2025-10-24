use approx::assert_relative_eq;
use kodegen_simd::similarity::{active_implementation, cosine_similarity, metrics, reset_metrics};

#[test]
fn test_cosine_similarity() {
    // Test with simple vectors
    let a = [1.0, 2.0, 3.0];
    let b = [4.0, 5.0, 6.0];

    let result = cosine_similarity(&a, &b);
    let expected = 0.974_631_8; // Precomputed value
    assert_relative_eq!(result, expected, epsilon = 1e-6);

    // Test with orthogonal vectors
    let a = [1.0, 0.0];
    let b = [0.0, 1.0];
    assert_relative_eq!(cosine_similarity(&a, &b), 0.0);

    // Test with identical vectors
    let a = [1.0, 2.0, 3.0];
    assert_relative_eq!(cosine_similarity(&a, &a), 1.0);
}

#[test]
fn test_metrics() {
    reset_metrics();

    let a = [1.0, 2.0, 3.0];
    let b = [4.0, 5.0, 6.0];

    // First call
    cosine_similarity(&a, &b);
    let metrics_snapshot = metrics();
    assert_eq!(metrics_snapshot.total_calculations, 1);
    assert_eq!(metrics_snapshot.total_elements_processed, 3);

    // Second call
    cosine_similarity(&a, &b);
    let metrics_snapshot = metrics();
    assert_eq!(metrics_snapshot.total_calculations, 2);
    assert_eq!(metrics_snapshot.total_elements_processed, 6);

    // Reset and verify
    reset_metrics();
    let metrics_snapshot = metrics();
    assert_eq!(metrics_snapshot.total_calculations, 0);
    assert_eq!(metrics_snapshot.total_elements_processed, 0);
}

#[test]
fn test_implementation_name() {
    // Just verify it returns something
    assert!(!active_implementation().is_empty());
}
