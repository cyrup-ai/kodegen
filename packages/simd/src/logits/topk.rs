//! Top-k filtering implementations

use crate::error::{SimdError, SimdResult};
// Use a simple feature detection for SIMD availability
#[inline]
const fn simd_available() -> bool {
    cfg!(target_feature = "avx2") || cfg!(target_feature = "neon")
}

/// Apply top-k filtering to logits using SIMD acceleration
///
/// This function preserves the top-k logits and sets all others to negative infinity.
///
/// # Arguments
///
/// * `logits` - Mutable slice of logits to filter
/// * `k` - Number of top logits to preserve
///
/// # Returns
///
/// Returns `Ok(())` on success, or error if k is invalid
pub fn topk_filtering_simd(logits: &mut [f32], k: usize) -> SimdResult<()> {
    if k == 0 || k > logits.len() {
        return Err(SimdError::InvalidInput(format!(
            "k ({}) must be between 1 and {}",
            k,
            logits.len()
        )));
    }

    if k == logits.len() {
        return Ok(()); // No filtering needed
    }

    if simd_available() {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { simd_top_k_large_avx2(logits, k) };
            } else if is_x86_feature_detected!("sse4.1") {
                return unsafe { simd_top_k_large_sse(logits, k) };
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                return unsafe { simd_top_k_large_neon(logits, k) };
            }
        }
    }

    // Fallback to scalar implementation
    let mut values: Vec<(usize, f32)> = logits.iter().enumerate().map(|(i, &x)| (i, x)).collect();

    // Use partial sort to find the k-th largest element
    let kth = values.len() - k;
    values.select_nth_unstable_by(kth, |a, b| {
        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Get the k-th largest value
    let threshold = values[kth].1;

    // Apply mask
    for x in logits.iter_mut() {
        if *x < threshold {
            *x = f32::NEG_INFINITY;
        }
    }

    Ok(())
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn simd_top_k_large_avx2(logits: &mut [f32], k: usize) -> SimdResult<()> {
    use std::arch::x86_64::*;

    // Simple implementation - for production, use a more sophisticated approach
    // like a priority queue or quickselect
    let mut values: Vec<f32> = logits.to_vec();
    values.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let threshold = if k > 0 {
        values[k - 1]
    } else {
        f32::NEG_INFINITY
    };

    // Process in chunks of 8 with AVX2
    let chunk_size = 8;
    let threshold_vec = _mm256_set1_ps(threshold);
    let mut i = 0;

    while i + chunk_size <= logits.len() {
        let ptr = logits.as_mut_ptr().add(i) as *mut __m256;
        let x = _mm256_loadu_ps(ptr);
        let mask = _mm256_cmp_ps(x, threshold_vec, _CMP_LT_OQ);
        let masked = _mm256_blendv_ps(x, _mm256_set1_ps(f32::NEG_INFINITY), mask);
        _mm256_storeu_ps(ptr, masked);
        i += chunk_size;
    }

    // Process remaining elements
    while i < logits.len() {
        if logits[i] < threshold {
            logits[i] = f32::NEG_INFINITY;
        }
        i += 1;
    }

    Ok(())
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.1")]
unsafe fn simd_top_k_large_sse(logits: &mut [f32], k: usize) -> SimdResult<()> {
    use std::arch::x86_64::*;

    // Simple implementation - for production, use a more sophisticated approach
    let mut values: Vec<f32> = logits.to_vec();
    values.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let threshold = if k > 0 {
        values[k - 1]
    } else {
        f32::NEG_INFINITY
    };

    // Process in chunks of 4 with SSE
    let chunk_size = 4;
    let threshold_vec = _mm_set1_ps(threshold);
    let mut i = 0;

    while i + chunk_size <= logits.len() {
        let ptr = logits.as_mut_ptr().add(i) as *mut __m128;
        let x = _mm_loadu_ps(ptr);
        let mask = _mm_cmplt_ps(x, threshold_vec);
        let masked = _mm_blendv_ps(x, _mm_set1_ps(f32::NEG_INFINITY), mask);
        _mm_storeu_ps(ptr, masked);
        i += chunk_size;
    }

    // Process remaining elements
    while i < logits.len() {
        if logits[i] < threshold {
            logits[i] = f32::NEG_INFINITY;
        }
        i += 1;
    }

    Ok(())
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn simd_top_k_large_neon(logits: &mut [f32], k: usize) -> SimdResult<()> {
    use std::arch::aarch64::*;

    // Simple implementation - for production, use a more sophisticated approach
    let mut values: Vec<f32> = logits.to_vec();
    values.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let threshold = if k > 0 {
        values[k - 1]
    } else {
        f32::NEG_INFINITY
    };

    // Process in chunks of 4 with NEON
    let chunk_size = 4;
    let threshold_vec = vdupq_n_f32(threshold);
    let inf_vec = vdupq_n_f32(f32::NEG_INFINITY);
    let mut i = 0;

    while i + chunk_size <= logits.len() {
        let ptr = unsafe { logits.as_mut_ptr().add(i) };
        let x = unsafe { vld1q_f32(ptr) };
        let mask = vcltq_f32(x, threshold_vec);
        let masked = vbslq_f32(mask, inf_vec, x);
        unsafe { vst1q_f32(ptr, masked) };
        i += chunk_size;
    }

    // Process remaining elements
    while i < logits.len() {
        if logits[i] < threshold {
            logits[i] = f32::NEG_INFINITY;
        }
        i += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topk_filtering() {
        let mut logits = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        if let Err(e) = topk_filtering_simd(&mut logits, 3) {
            panic!("Top-k filtering failed: {}", e);
        }

        // Only top 3 values should remain
        let mut count = 0;
        for &x in &logits {
            if x > f32::NEG_INFINITY {
                count += 1;
            }
        }
        assert_eq!(count, 3);

        // The top 3 values should be 9.0, 6.0, 5.0
        assert!(logits.contains(&9.0));
        assert!(logits.contains(&6.0));
        assert!(logits.contains(&5.0));
    }

    #[test]
    fn test_topk_filtering_invalid_k() {
        let mut logits = [1.0, 2.0, 3.0];
        let result = topk_filtering_simd(&mut logits, 0);
        assert!(matches!(result, Err(SimdError::InvalidInput(_))));

        let result = topk_filtering_simd(&mut logits, 4);
        assert!(matches!(result, Err(SimdError::InvalidInput(_))));
    }

    #[test]
    fn test_topk_filtering_all() {
        let mut logits = [1.0, 2.0, 3.0];
        if let Err(e) = topk_filtering_simd(&mut logits, 3) {
            panic!("Top-k filtering failed: {}", e);
        }
        assert_eq!(logits, [1.0, 2.0, 3.0]);
    }
}
