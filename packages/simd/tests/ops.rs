use float_eq::assert_float_eq;
use kodegen_simd::ops::{argmax, scale_temperature, softmax};

#[test]
fn test_temperature_scale_empty() {
    let mut logits: Vec<f32> = vec![];
    let result = scale_temperature(&mut logits, 1.0);
    assert!(result.is_err(), "Expected error for empty logits");
}

#[test]
fn test_temperature_scale_invalid_temperature() {
    let mut logits = vec![1.0, 2.0];
    let result = scale_temperature(&mut logits, 0.0);
    assert!(
        result.is_err(),
        "Expected error for non-positive temperature"
    );
}

#[test]
fn test_temperature_scale_single_element() {
    let mut logits = vec![5.0];
    scale_temperature(&mut logits, 2.0).expect("Temperature scaling failed");
    assert_float_eq!(logits[0], 2.5, abs <= 1e-6);
}

#[test]
fn test_temperature_scale_multiple_elements() {
    let mut logits = vec![1.0, 2.0, 3.0];
    scale_temperature(&mut logits, 0.5).expect("Temperature scaling failed");
    assert_float_eq!(logits[0], 2.0, abs <= 1e-6);
    assert_float_eq!(logits[1], 4.0, abs <= 1e-6);
    assert_float_eq!(logits[2], 6.0, abs <= 1e-6);
}

#[test]
fn test_temperature_scale_large_vector() {
    let mut logits: Vec<f32> = (0..1024).map(|i| i as f32).collect();
    let temperature = 1.5;
    scale_temperature(&mut logits, temperature).expect("Temperature scaling failed");
    for (i, &val) in logits.iter().enumerate() {
        assert_float_eq!(val, (i as f32) / temperature, rel <= 1e-6);
    }
}

#[test]
fn test_softmax_empty() {
    let logits: Vec<f32> = vec![];
    let result = softmax(&logits).expect("Softmax failed");
    assert!(result.is_empty(), "Expected empty result for empty input");
}

#[test]
fn test_softmax_single_element() {
    let logits = vec![5.0];
    let result = softmax(&logits).expect("Softmax failed");
    assert_eq!(result.len(), 1);
    assert_float_eq!(result[0], 1.0, abs <= 1e-6);
}

#[test]
fn test_softmax_two_elements() {
    let logits = vec![1.0, 1.0];
    let result = softmax(&logits).expect("Softmax failed");
    assert_float_eq!(result[0], 0.5, abs <= 1e-6);
    assert_float_eq!(result[1], 0.5, abs <= 1e-6);
}

#[test]
fn test_softmax_multiple_elements() {
    let logits = vec![1.0, 2.0, 3.0];
    let result = softmax(&logits).expect("Softmax failed");
    let sum: f32 = result.iter().sum();
    assert_float_eq!(sum, 1.0, abs <= 1e-5);
    assert_float_eq!(
        result[0],
        (1.0f32.exp() / (1.0f32.exp() + 2.0f32.exp() + 3.0f32.exp())),
        abs <= 1e-5
    );
    assert_float_eq!(
        result[1],
        (2.0f32.exp() / (1.0f32.exp() + 2.0f32.exp() + 3.0f32.exp())),
        abs <= 1e-5
    );
    assert_float_eq!(
        result[2],
        (3.0f32.exp() / (1.0f32.exp() + 2.0f32.exp() + 3.0f32.exp())),
        abs <= 1e-5
    );
}

#[test]
fn test_softmax_large_vector() {
    let logits: Vec<f32> = (0..1024).map(|i| (i % 100) as f32).collect();
    let result = softmax(&logits).expect("Softmax failed");
    let sum: f32 = result.iter().sum();
    assert_float_eq!(sum, 1.0, abs <= 1e-4);
}

#[test]
fn test_argmax_empty() {
    let logits: Vec<f32> = vec![];
    let result = argmax(&logits);
    assert!(result.is_err(), "Expected error for empty input");
}

#[test]
fn test_argmax_single_element() {
    let logits = vec![5.0];
    let idx = argmax(&logits).expect("Argmax failed");
    assert_eq!(idx, 0);
}

#[test]
fn test_argmax_multiple_elements() {
    let logits = vec![1.0, 3.0, 2.0];
    let idx = argmax(&logits).expect("Argmax failed");
    assert_eq!(idx, 1);
}

#[test]
fn test_argmax_ties() {
    let logits = vec![3.0, 3.0, 2.0];
    let idx = argmax(&logits).expect("Argmax failed");
    assert_eq!(idx, 0); // Should return first max
}

#[test]
fn test_argmax_large_vector() {
    let mut logits: Vec<f32> = (0..1024).map(|i| i as f32).collect();
    logits[512] = 2000.0; // Set a max in the middle
    let idx = argmax(&logits).expect("Argmax failed");
    assert_eq!(idx, 512);
}

#[test]
fn test_combined_operations() {
    let mut logits = vec![1.0, 2.0, 3.0, 4.0];
    scale_temperature(&mut logits, 0.5).expect("Temperature scaling failed");
    let soft = softmax(&logits).expect("Softmax failed");
    let idx = argmax(&soft).expect("Argmax failed");
    assert_eq!(idx, 3); // Highest logit should have highest probability
    let sum: f32 = soft.iter().sum();
    assert_float_eq!(sum, 1.0, abs <= 1e-5);
}
