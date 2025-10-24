//! SIMD correctness tests - verify all implementations produce identical results
//!
//! These tests compare SIMD implementations against scalar reference implementation
//! to ensure correctness across all CPU architectures.

#![cfg(feature = "bench")]

use float_eq::assert_float_eq;
use kodegen_simd::ops::argmax::ARGMAX_DISPATCH;
use kodegen_simd::ops::softmax::SOFTMAX_DISPATCH;
use kodegen_simd::ops::temperature::TEMPERATURE_DISPATCH;
use kodegen_simd::runtime::CpuFeatures;
use rand::Rng;

const EPSILON: f32 = 1e-5;

/// Generate random test data
fn generate_random_data(size: usize) -> Vec<f32> {
    let mut rng = rand::rng();
    (0..size).map(|_| rng.random_range(-10.0..10.0)).collect()
}

#[test]
fn test_temperature_empty_array() {
    let mut logits: Vec<f32> = vec![];
    let result = TEMPERATURE_DISPATCH.call(&mut logits, 1.0);
    assert!(
        result.is_err(),
        "Temperature scaling on empty array should error"
    );
}

#[test]
fn test_temperature_zero_temp() {
    let mut logits = vec![1.0, 2.0, 3.0];
    let result = TEMPERATURE_DISPATCH.call(&mut logits, 0.0);
    assert!(result.is_err(), "Temperature of 0.0 should error");
}
#[test]
fn test_temperature_single_element() {
    let mut logits_scalar = vec![5.0];
    let mut logits_simd = logits_scalar.clone();

    TEMPERATURE_DISPATCH
        .call_with_feature(&mut logits_scalar, 0.7, CpuFeatures::Scalar)
        .expect("Scalar temperature scaling should succeed");

    // Try AVX2 if available
    if let Ok(()) = TEMPERATURE_DISPATCH.call_with_feature(&mut logits_simd, 0.7, CpuFeatures::Avx2)
    {
        assert_float_eq!(logits_scalar[0], logits_simd[0], abs <= EPSILON);
    }
}

#[test]
fn test_temperature_power_of_two_sizes() {
    let sizes = [16, 32, 64, 128];
    let temperature = 0.7f32;

    for size in sizes {
        let mut logits_scalar = generate_random_data(size);
        let mut logits_avx2 = logits_scalar.clone();
        let mut logits_avx512 = logits_scalar.clone();

        TEMPERATURE_DISPATCH
            .call_with_feature(&mut logits_scalar, temperature, CpuFeatures::Scalar)
            .expect("Scalar should succeed");

        // Compare AVX2 if available
        if let Ok(()) =
            TEMPERATURE_DISPATCH.call_with_feature(&mut logits_avx2, temperature, CpuFeatures::Avx2)
        {
            for (i, (&scalar, &simd)) in logits_scalar.iter().zip(logits_avx2.iter()).enumerate() {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "Mismatch at index {} for size {}",
                    i,
                    size
                );
            }
        } // Compare AVX-512 if available
        if let Ok(()) = TEMPERATURE_DISPATCH.call_with_feature(
            &mut logits_avx512,
            temperature,
            CpuFeatures::Avx512,
        ) {
            for (i, (&scalar, &simd)) in logits_scalar.iter().zip(logits_avx512.iter()).enumerate()
            {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "Mismatch at index {} for size {}",
                    i,
                    size
                );
            }
        }
    }
}

#[test]
fn test_temperature_non_power_of_two_sizes() {
    let sizes = [15, 33, 65, 127];
    let temperature = 0.5f32;

    for size in sizes {
        let mut logits_scalar = generate_random_data(size);
        let mut logits_avx2 = logits_scalar.clone();
        let mut logits_avx512 = logits_scalar.clone();

        TEMPERATURE_DISPATCH
            .call_with_feature(&mut logits_scalar, temperature, CpuFeatures::Scalar)
            .expect("Scalar should succeed");

        if let Ok(()) =
            TEMPERATURE_DISPATCH.call_with_feature(&mut logits_avx2, temperature, CpuFeatures::Avx2)
        {
            for (i, (&scalar, &simd)) in logits_scalar.iter().zip(logits_avx2.iter()).enumerate() {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "Mismatch at index {} for size {}",
                    i,
                    size
                );
            }
        }

        if let Ok(()) = TEMPERATURE_DISPATCH.call_with_feature(
            &mut logits_avx512,
            temperature,
            CpuFeatures::Avx512,
        ) {
            for (i, (&scalar, &simd)) in logits_scalar.iter().zip(logits_avx512.iter()).enumerate()
            {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "Mismatch at index {} for size {}",
                    i,
                    size
                );
            }
        }
    }
}

#[test]
fn test_temperature_with_inf_values() {
    let mut logits_scalar = vec![1.0, f32::INFINITY, 3.0, f32::NEG_INFINITY, 5.0];
    let mut logits_simd = logits_scalar.clone();

    TEMPERATURE_DISPATCH
        .call_with_feature(&mut logits_scalar, 0.7, CpuFeatures::Scalar)
        .expect("Scalar should handle Inf values");

    if let Ok(()) = TEMPERATURE_DISPATCH.call_with_feature(&mut logits_simd, 0.7, CpuFeatures::Avx2)
    {
        for (i, (&scalar, &simd)) in logits_scalar.iter().zip(logits_simd.iter()).enumerate() {
            if scalar.is_finite() {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "Finite value mismatch at index {}",
                    i
                );
            } else {
                assert_eq!(scalar, simd, "Inf/NaN handling mismatch at index {}", i);
            }
        }
    }
}

#[test]
fn test_temperature_with_nan_values() {
    let mut logits_scalar = vec![1.0, f32::NAN, 3.0, 4.0];
    let mut logits_simd = logits_scalar.clone();
    TEMPERATURE_DISPATCH
        .call_with_feature(&mut logits_scalar, 0.7, CpuFeatures::Scalar)
        .expect("Scalar should handle NaN values");

    if let Ok(()) = TEMPERATURE_DISPATCH.call_with_feature(&mut logits_simd, 0.7, CpuFeatures::Avx2)
    {
        // Both should convert NaN to 0.0
        for (i, (&scalar, &simd)) in logits_scalar.iter().zip(logits_simd.iter()).enumerate() {
            if !scalar.is_nan() && !simd.is_nan() {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "Value mismatch at index {}",
                    i
                );
            }
        }
    }
}

#[test]
fn test_argmax_empty_array() {
    let logits: Vec<f32> = vec![];
    let result = ARGMAX_DISPATCH.call(&logits);
    assert!(result.is_err(), "Argmax on empty array should error");
}

#[test]
fn test_argmax_single_element() {
    let logits = vec![5.0];
    let idx_scalar = ARGMAX_DISPATCH
        .call_with_feature(&logits, CpuFeatures::Scalar)
        .expect("Scalar argmax should succeed");
    assert_eq!(idx_scalar, 0);

    if let Ok(idx_simd) = ARGMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx2) {
        assert_eq!(idx_scalar, idx_simd);
    }
}

#[test]
fn test_argmax_random_data() {
    let sizes = [16, 64, 256, 1024];

    for size in sizes {
        let logits = generate_random_data(size);
        let idx_scalar = ARGMAX_DISPATCH
            .call_with_feature(&logits, CpuFeatures::Scalar)
            .expect("Scalar argmax should succeed");

        if let Ok(idx_avx2) = ARGMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx2) {
            assert_eq!(
                idx_scalar, idx_avx2,
                "AVX2 argmax mismatch for size {}",
                size
            );
        }

        if let Ok(idx_avx512) = ARGMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx512) {
            assert_eq!(
                idx_scalar, idx_avx512,
                "AVX-512 argmax mismatch for size {}",
                size
            );
        }
    }
}

#[test]
fn test_argmax_sorted_data() {
    let logits_asc: Vec<f32> = (0..100).map(|i| i as f32).collect();
    let logits_desc: Vec<f32> = (0..100).rev().map(|i| i as f32).collect();

    let idx_asc_scalar = ARGMAX_DISPATCH
        .call_with_feature(&logits_asc, CpuFeatures::Scalar)
        .expect("Scalar should succeed");
    assert_eq!(
        idx_asc_scalar, 99,
        "Max should be at end of ascending array"
    );

    let idx_desc_scalar = ARGMAX_DISPATCH
        .call_with_feature(&logits_desc, CpuFeatures::Scalar)
        .expect("Scalar should succeed");
    assert_eq!(
        idx_desc_scalar, 0,
        "Max should be at start of descending array"
    );
    if let Ok(idx_avx2) = ARGMAX_DISPATCH.call_with_feature(&logits_asc, CpuFeatures::Avx2) {
        assert_eq!(idx_asc_scalar, idx_avx2);
    }

    if let Ok(idx_avx512) = ARGMAX_DISPATCH.call_with_feature(&logits_desc, CpuFeatures::Avx512) {
        assert_eq!(idx_desc_scalar, idx_avx512);
    }
}

#[test]
fn test_argmax_all_equal() {
    let logits = vec![5.0; 100];
    let idx_scalar = ARGMAX_DISPATCH
        .call_with_feature(&logits, CpuFeatures::Scalar)
        .expect("Scalar should succeed");

    // When all equal, should return first index
    assert_eq!(idx_scalar, 0);

    if let Ok(idx_simd) = ARGMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx2) {
        assert_eq!(idx_simd, 0, "AVX2 should return first index when all equal");
    }
}

#[test]
fn test_softmax_empty_array() {
    let logits: Vec<f32> = vec![];
    let result = SOFTMAX_DISPATCH.call(&logits);
    assert!(
        result.is_ok(),
        "Softmax on empty array should return empty result"
    );
    let probs = result.expect("Should be Ok");
    assert!(probs.is_empty());
}
#[test]
fn test_softmax_single_element() {
    let logits = vec![5.0];
    let probs_scalar = SOFTMAX_DISPATCH
        .call_with_feature(&logits, CpuFeatures::Scalar)
        .expect("Scalar softmax should succeed");

    assert_eq!(probs_scalar.len(), 1);
    assert_float_eq!(probs_scalar[0], 1.0, abs <= EPSILON);

    if let Ok(probs_simd) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx2) {
        assert_float_eq!(probs_scalar[0], probs_simd[0], abs <= EPSILON);
    }
}

#[test]
fn test_softmax_probabilities_sum_to_one() {
    let sizes = [16, 32, 64, 128, 256];

    for size in sizes {
        let logits = generate_random_data(size);

        let probs_scalar = SOFTMAX_DISPATCH
            .call_with_feature(&logits, CpuFeatures::Scalar)
            .expect("Scalar softmax should succeed");
        let sum_scalar: f32 = probs_scalar.iter().sum();
        assert_float_eq!(
            sum_scalar,
            1.0,
            abs <= EPSILON,
            "Scalar probabilities should sum to 1.0 for size {}",
            size
        );

        if let Ok(probs_avx2) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx2) {
            let sum_avx2: f32 = probs_avx2.iter().sum();
            assert_float_eq!(
                sum_avx2,
                1.0,
                abs <= EPSILON,
                "AVX2 probabilities should sum to 1.0 for size {}",
                size
            );
        }

        if let Ok(probs_avx512) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx512) {
            let sum_avx512: f32 = probs_avx512.iter().sum();
            assert_float_eq!(
                sum_avx512,
                1.0,
                abs <= EPSILON,
                "AVX-512 probabilities should sum to 1.0 for size {}",
                size
            );
        }
    }
}

#[test]
fn test_softmax_small_value_range() {
    let logits = generate_random_data(128)
        .iter()
        .map(|&x| x / 10.0)
        .collect::<Vec<_>>();

    let probs_scalar = SOFTMAX_DISPATCH
        .call_with_feature(&logits, CpuFeatures::Scalar)
        .expect("Scalar should succeed");

    if let Ok(probs_avx2) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx2) {
        for (i, (&scalar, &simd)) in probs_scalar.iter().zip(probs_avx2.iter()).enumerate() {
            assert_float_eq!(scalar, simd, abs <= EPSILON, "Mismatch at index {}", i);
        }
    }

    if let Ok(probs_avx512) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx512) {
        for (i, (&scalar, &simd)) in probs_scalar.iter().zip(probs_avx512.iter()).enumerate() {
            assert_float_eq!(scalar, simd, abs <= EPSILON, "Mismatch at index {}", i);
        }
    }
}

#[test]
fn test_softmax_large_value_range() {
    let logits = generate_random_data(128)
        .iter()
        .map(|&x| x * 10.0)
        .collect::<Vec<_>>();

    let probs_scalar = SOFTMAX_DISPATCH
        .call_with_feature(&logits, CpuFeatures::Scalar)
        .expect("Scalar should succeed");

    if let Ok(probs_avx2) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx2) {
        for (i, (&scalar, &simd)) in probs_scalar.iter().zip(probs_avx2.iter()).enumerate() {
            // Larger epsilon for fast exp approximation with large values
            assert_float_eq!(scalar, simd, abs <= 0.001, "Mismatch at index {}", i);
        }
    }

    if let Ok(probs_avx512) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx512) {
        for (i, (&scalar, &simd)) in probs_scalar.iter().zip(probs_avx512.iter()).enumerate() {
            assert_float_eq!(scalar, simd, abs <= 0.001, "Mismatch at index {}", i);
        }
    }
}

#[test]
fn test_softmax_correctness_across_implementations() {
    let sizes = [16, 33, 64, 127, 256];

    for size in sizes {
        let logits = generate_random_data(size);

        let probs_scalar = SOFTMAX_DISPATCH
            .call_with_feature(&logits, CpuFeatures::Scalar)
            .expect("Scalar should succeed");
        if let Ok(probs_sse41) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Sse41) {
            for (i, (&scalar, &simd)) in probs_scalar.iter().zip(probs_sse41.iter()).enumerate() {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "SSE4.1 mismatch at index {} for size {}",
                    i,
                    size
                );
            }
        }

        if let Ok(probs_avx2) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx2) {
            for (i, (&scalar, &simd)) in probs_scalar.iter().zip(probs_avx2.iter()).enumerate() {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "AVX2 mismatch at index {} for size {}",
                    i,
                    size
                );
            }
        }

        if let Ok(probs_avx512) = SOFTMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx512) {
            for (i, (&scalar, &simd)) in probs_scalar.iter().zip(probs_avx512.iter()).enumerate() {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "AVX-512 mismatch at index {} for size {}",
                    i,
                    size
                );
            }
        }
    }
}

#[test]
fn test_temperature_various_values() {
    let temperatures = [0.1f32, 0.5f32, 1.0f32, 2.0f32, 10.0f32];
    let logits_base = generate_random_data(64);

    for temp in temperatures {
        let mut logits_scalar = logits_base.clone();
        let mut logits_avx2 = logits_base.clone();
        let mut logits_avx512 = logits_base.clone();

        TEMPERATURE_DISPATCH
            .call_with_feature(&mut logits_scalar, temp, CpuFeatures::Scalar)
            .expect("Scalar should succeed");
        if let Ok(()) =
            TEMPERATURE_DISPATCH.call_with_feature(&mut logits_avx2, temp, CpuFeatures::Avx2)
        {
            for (i, (&scalar, &simd)) in logits_scalar.iter().zip(logits_avx2.iter()).enumerate() {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "AVX2 mismatch at index {} for temp {}",
                    i,
                    temp
                );
            }
        }

        if let Ok(()) =
            TEMPERATURE_DISPATCH.call_with_feature(&mut logits_avx512, temp, CpuFeatures::Avx512)
        {
            for (i, (&scalar, &simd)) in logits_scalar.iter().zip(logits_avx512.iter()).enumerate()
            {
                assert_float_eq!(
                    scalar,
                    simd,
                    abs <= EPSILON,
                    "AVX-512 mismatch at index {} for temp {}",
                    i,
                    temp
                );
            }
        }
    }
}

#[test]
fn test_large_array_correctness() {
    let size = 10000;
    let logits = generate_random_data(size);

    // Temperature test
    let mut temp_scalar = logits.clone();
    let mut temp_simd = logits.clone();
    TEMPERATURE_DISPATCH
        .call_with_feature(&mut temp_scalar, 0.7, CpuFeatures::Scalar)
        .expect("Scalar should succeed");

    if let Ok(()) = TEMPERATURE_DISPATCH.call_with_feature(&mut temp_simd, 0.7, CpuFeatures::Avx512)
    {
        for (i, (&scalar, &simd)) in temp_scalar.iter().zip(temp_simd.iter()).enumerate() {
            assert_float_eq!(
                scalar,
                simd,
                abs <= EPSILON,
                "Temperature mismatch at index {}",
                i
            );
        }
    }

    // Argmax test
    let idx_scalar = ARGMAX_DISPATCH
        .call_with_feature(&logits, CpuFeatures::Scalar)
        .expect("Scalar argmax should succeed");
    if let Ok(idx_simd) = ARGMAX_DISPATCH.call_with_feature(&logits, CpuFeatures::Avx512) {
        assert_eq!(idx_scalar, idx_simd, "Argmax mismatch for large array");
    }

    // Softmax test (only on subset for performance)
    let softmax_subset = &logits[0..1000];
    let probs_scalar = SOFTMAX_DISPATCH
        .call_with_feature(softmax_subset, CpuFeatures::Scalar)
        .expect("Scalar softmax should succeed");

    if let Ok(probs_simd) = SOFTMAX_DISPATCH.call_with_feature(softmax_subset, CpuFeatures::Avx512)
    {
        for (i, (&scalar, &simd)) in probs_scalar.iter().zip(probs_simd.iter()).enumerate() {
            assert_float_eq!(
                scalar,
                simd,
                abs <= EPSILON,
                "Softmax mismatch at index {}",
                i
            );
        }
    }
}
