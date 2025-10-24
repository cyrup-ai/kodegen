//! Core logits processing functions

use crate::error::{SimdError, SimdResult};
// Use a simple feature detection for SIMD availability
#[inline]
const fn simd_available() -> bool {
    cfg!(target_feature = "avx2") || cfg!(target_feature = "neon")
}

/// Apply temperature scaling to logits using SIMD acceleration
///
/// # Arguments
///
/// * `logits` - Mutable slice of logits to scale
/// * `temperature` - Temperature value (must be > 0.0)
///
/// # Returns
///
/// Returns `Ok(())` on success, or error if temperature is invalid
pub fn apply_temperature_scaling_simd(logits: &mut [f32], temperature: f32) -> SimdResult<()> {
    if temperature <= 0.0 {
        return Err(SimdError::InvalidInput(
            "Temperature must be positive".to_string(),
        ));
    }

    let inv_temp = 1.0 / temperature;

    if simd_available() {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { apply_temperature_avx2(logits, inv_temp) };
            } else if is_x86_feature_detected!("sse4.1") {
                return unsafe { apply_temperature_sse(logits, inv_temp) };
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                return unsafe { apply_temperature_neon(logits, inv_temp) };
            }
        }
    }

    // Fallback to scalar implementation
    for x in logits.iter_mut() {
        *x *= inv_temp;
    }

    Ok(())
}

/// Normalize logits to probabilities using SIMD-accelerated softmax
///
/// # Arguments
///
/// * `logits` - Mutable slice of logits to normalize
///
/// # Returns
///
/// Returns `Ok(())` on success, or error if input is invalid
pub fn normalize_probabilities_simd(logits: &mut [f32]) -> SimdResult<()> {
    if logits.is_empty() {
        return Ok(());
    }

    // Find max for numerical stability
    let max = logits
        .iter()
        .copied()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .ok_or_else(|| SimdError::NumericalError("Empty logits".to_string()))?;

    // Compute sum of exp(logits - max)
    let mut sum = 0.0;
    for x in logits.iter_mut() {
        *x = (*x - max).exp();
        sum += *x;
    }

    // Normalize
    if sum > 0.0 {
        let inv_sum = 1.0 / sum;
        for x in logits.iter_mut() {
            *x *= inv_sum;
        }
    } else {
        // If sum is zero, set uniform distribution
        let uniform = 1.0 / logits.len() as f32;
        for x in logits.iter_mut() {
            *x = uniform;
        }
    }

    Ok(())
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_temperature_avx2(logits: &mut [f32], inv_temp: f32) -> SimdResult<()> {
    use std::arch::x86_64::*;

    let inv_temp_vec = _mm256_set1_ps(inv_temp);
    let mut i = 0;

    // Process 8 elements at a time with AVX2
    let chunk_size = 8;
    while i + chunk_size <= logits.len() {
        let ptr = logits.as_mut_ptr().add(i) as *mut __m256;
        let x = _mm256_loadu_ps(ptr);
        let scaled = _mm256_mul_ps(x, inv_temp_vec);
        _mm256_storeu_ps(ptr, scaled);
        i += chunk_size;
    }

    // Process remaining elements
    while i < logits.len() {
        logits[i] *= inv_temp;
        i += 1;
    }

    Ok(())
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.1")]
unsafe fn apply_temperature_sse(logits: &mut [f32], inv_temp: f32) -> SimdResult<()> {
    use std::arch::x86_64::*;

    let inv_temp_vec = _mm_set1_ps(inv_temp);
    let mut i = 0;

    // Process 4 elements at a time with SSE
    let chunk_size = 4;
    while i + chunk_size <= logits.len() {
        let ptr = logits.as_mut_ptr().add(i) as *mut __m128;
        let x = _mm_loadu_ps(ptr);
        let scaled = _mm_mul_ps(x, inv_temp_vec);
        _mm_storeu_ps(ptr, scaled);
        i += chunk_size;
    }

    // Process remaining elements
    while i < logits.len() {
        logits[i] *= inv_temp;
        i += 1;
    }

    Ok(())
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn apply_temperature_neon(logits: &mut [f32], inv_temp: f32) -> SimdResult<()> {
    use std::arch::aarch64::*;

    let inv_temp_vec = vdupq_n_f32(inv_temp);
    let mut i = 0;

    // Process 4 elements at a time with NEON
    let chunk_size = 4;
    while i + chunk_size <= logits.len() {
        unsafe {
            // SAFETY: ptr is derived from valid slice bounds (i + chunk_size <= logits.len())
            // and chunk_size is 4, matching NEON 128-bit register size
            let ptr = logits.as_mut_ptr().add(i);
            let x = vld1q_f32(ptr);
            let scaled = vmulq_f32(x, inv_temp_vec);
            vst1q_f32(ptr, scaled);
        }
        i += chunk_size;
    }

    // Process remaining elements
    while i < logits.len() {
        logits[i] *= inv_temp;
        i += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use float_eq::assert_float_eq;

    use super::*;

    #[test]
    fn test_apply_temperature() {
        let mut logits = [1.0, 2.0, 3.0];
        if let Err(e) = apply_temperature_scaling_simd(&mut logits, 0.5) {
            panic!("Temperature scaling failed: {}", e);
        }
        assert_float_eq!(logits[0], 2.0, abs <= 1e-6);
        assert_float_eq!(logits[1], 4.0, abs <= 1e-6);
        assert_float_eq!(logits[2], 6.0, abs <= 1e-6);
    }

    #[test]
    fn test_apply_temperature_invalid() {
        let mut logits = [1.0, 2.0, 3.0];
        let result = apply_temperature_scaling_simd(&mut logits, 0.0);
        assert!(matches!(result, Err(SimdError::InvalidInput(_))));
    }

    #[test]
    fn test_normalize_probabilities() {
        let mut logits = [1.0, 2.0, 3.0];
        if let Err(e) = normalize_probabilities_simd(&mut logits) {
            panic!("Normalization failed: {}", e);
        }
        let sum: f32 = logits.iter().sum();
        assert_float_eq!(sum, 1.0, abs <= 1e-6);
    }

    #[test]
    fn test_normalize_empty() {
        let mut logits: [f32; 0] = [];
        assert!(normalize_probabilities_simd(&mut logits).is_ok());
    }
}
