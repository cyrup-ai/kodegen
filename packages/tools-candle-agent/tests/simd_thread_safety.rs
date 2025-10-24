//! Test SIMD thread safety with concurrent spawn_blocking calls
//!
//! This test verifies that all cyrup-simd operations are thread-safe
//! when used in concurrent spawn_blocking tasks.

#[tokio::test]
async fn test_concurrent_simd_operations() {
    use kodegen_simd::{
        argmax, prepare_nucleus_sampling_simd, scale_temperature, softmax, topk_filtering_simd,
    };

    // Spawn 100 concurrent tasks to stress test thread safety
    let handles: Vec<_> = (0..100)
        .map(|i| {
            tokio::task::spawn_blocking(move || {
                // Test data
                let mut logits = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

                // Test scale_temperature
                scale_temperature(&mut logits, 0.5).expect("scale_temperature failed");

                // Test topk_filtering_simd
                let mut logits2 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
                topk_filtering_simd(&mut logits2, 3).expect("topk_filtering_simd failed");

                // Test prepare_nucleus_sampling_simd
                let mut logits3 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
                prepare_nucleus_sampling_simd(&mut logits3, 0.9)
                    .expect("prepare_nucleus_sampling_simd failed");

                // Test softmax
                let logits4 = vec![1.0, 2.0, 3.0, 4.0];
                let probs = softmax(&logits4).expect("softmax failed");
                assert_eq!(probs.len(), 4);

                // Test argmax
                let logits5 = vec![1.0, 5.0, 3.0, 2.0];
                let max_idx = argmax(&logits5).expect("argmax failed");
                assert_eq!(max_idx, 1);

                i
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task failed");
    }

    println!("✅ All 100 concurrent SIMD operations completed successfully");
}
