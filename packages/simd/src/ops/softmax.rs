//! SIMD accelerated Softmax operations for cyrup
//!
//! This module provides vectorized implementations of softmax operations
//! optimized for AI inference tasks, focusing on zero-allocation and lock-free design.

use once_cell::sync::Lazy;

use crate::error::SimdResult;
use crate::runtime::SoftmaxDispatch;

/// Scalar implementation of softmax as a fallback
fn scalar_softmax(logits: &[f32]) -> SimdResult<Vec<f32>> {
    if logits.is_empty() {
        return Ok(Vec::new());
    }

    let mut max_val = logits[0];
    for &val in logits.iter().skip(1) {
        if val > max_val {
            max_val = val;
        }
    }

    let mut result = Vec::with_capacity(logits.len());
    let mut sum = 0.0f32;

    for &val in logits.iter() {
        let shifted = val - max_val;
        let exp_val = shifted.exp();
        sum += exp_val;
        result.push(exp_val);
    }

    for val in result.iter_mut() {
        *val /= sum;
    }

    Ok(result)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn avx2_softmax(logits: &[f32]) -> SimdResult<Vec<f32>> {
    use std::arch::x86_64::*;

    if logits.is_empty() {
        return Ok(Vec::new());
    }

    let len = logits.len();
    let mut maxv = _mm256_set1_ps(f32::NEG_INFINITY);
    let mut i = 0;
    while i + 8 <= len {
        let v = _mm256_loadu_ps(logits.as_ptr().add(i));
        maxv = _mm256_max_ps(maxv, v);
        i += 8;
    }
    let mut max_arr = [f32::NEG_INFINITY; 8];
    _mm256_storeu_ps(max_arr.as_mut_ptr(), maxv);
    let mut max_scalar = max_arr.iter().fold(f32::NEG_INFINITY, |acc, &x| acc.max(x));
    for &v in &logits[i..] {
        max_scalar = max_scalar.max(v);
    }

    let max_b = _mm256_set1_ps(max_scalar);
    let mut sum = 0.0f32;
    let mut result = vec![0.0f32; len]; // Allocate once
    i = 0;
    while i + 8 <= len {
        let v = _mm256_loadu_ps(logits.as_ptr().add(i));
        let shifted = _mm256_sub_ps(v, max_b);

        // Fast exp approximation
        let log2e = _mm256_set1_ps(1.442695041f32);
        let c127 = _mm256_set1_ps(127.0f32);
        let tmp = _mm256_mul_ps(shifted, log2e);
        let y = _mm256_add_ps(tmp, c127);
        let exp_i = _mm256_cvttps_epi32(y);
        let exp_f = _mm256_castsi256_ps(_mm256_slli_epi32(exp_i, 23));

        _mm256_storeu_ps(result.as_mut_ptr().add(i), exp_f);
        let chunk_sum = {
            let mut arr = [0.0; 8];
            _mm256_storeu_ps(arr.as_mut_ptr(), exp_f);
            arr.iter().sum::<f32>()
        };
        sum += chunk_sum;
        i += 8;
    }
    for j in i..len {
        let shifted = logits[j] - max_scalar;
        let exp_val = shifted.exp(); // More accurate for remainder
        result[j] = exp_val;
        sum += exp_val;
    }

    let inv_sum = 1.0 / sum;
    let inv_sum_b = _mm256_set1_ps(inv_sum);
    i = 0;
    while i + 8 <= len {
        let v = _mm256_loadu_ps(result.as_ptr().add(i));
        let norm = _mm256_mul_ps(v, inv_sum_b);
        _mm256_storeu_ps(result.as_mut_ptr().add(i), norm);
        i += 8;
    }
    for result_item in result.iter_mut().take(len).skip(i) {
        *result_item *= inv_sum;
    }

    Ok(result)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn avx512_softmax(logits: &[f32]) -> SimdResult<Vec<f32>> {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    if logits.is_empty() {
        return Ok(Vec::new());
    }

    let len = logits.len();
    let mut maxv = _mm512_set1_ps(f32::NEG_INFINITY);
    let mut i = 0;
    while i + 16 <= len {
        let v = _mm512_loadu_ps(logits.as_ptr().add(i));
        maxv = _mm512_max_ps(maxv, v);
        i += 16;
    }
    let mut max_arr = [f32::NEG_INFINITY; 16];
    _mm512_storeu_ps(max_arr.as_mut_ptr(), maxv);
    let mut max_scalar = max_arr.iter().fold(f32::NEG_INFINITY, |acc, &x| acc.max(x));
    for &v in &logits[i..] {
        max_scalar = max_scalar.max(v);
    }

    let max_b = _mm512_set1_ps(max_scalar);
    let mut sum = 0.0f32;
    let mut result = vec![0.0f32; len];
    i = 0;
    while i + 16 <= len {
        let v = _mm512_loadu_ps(logits.as_ptr().add(i));
        let shifted = _mm512_sub_ps(v, max_b);

        // Fast exp approximation
        let log2e = _mm512_set1_ps(1.442695041f32);
        let c127 = _mm512_set1_ps(127.0f32);
        let tmp = _mm512_mul_ps(shifted, log2e);
        let y = _mm512_add_ps(tmp, c127);
        let exp_i = _mm512_cvttps_epi32(y);
        let exp_f = _mm512_castsi512_ps(_mm512_slli_epi32(exp_i, 23));

        _mm512_storeu_ps(result.as_mut_ptr().add(i), exp_f);
        let chunk_sum = {
            let mut arr = [0.0; 16];
            _mm512_storeu_ps(arr.as_mut_ptr(), exp_f);
            arr.iter().sum::<f32>()
        };
        sum += chunk_sum;
        i += 16;
    }
    for j in i..len {
        let shifted = logits[j] - max_scalar;
        let exp_val = shifted.exp();
        result[j] = exp_val;
        sum += exp_val;
    }

    let inv_sum = 1.0 / sum;
    let inv_sum_b = _mm512_set1_ps(inv_sum);
    i = 0;
    while i + 16 <= len {
        let v = _mm512_loadu_ps(result.as_ptr().add(i));
        let norm = _mm512_mul_ps(v, inv_sum_b);
        _mm512_storeu_ps(result.as_mut_ptr().add(i), norm);
        i += 16;
    }
    for result_item in result.iter_mut().take(len).skip(i) {
        *result_item *= inv_sum;
    }

    Ok(result)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.1")]
unsafe fn sse41_softmax(logits: &[f32]) -> SimdResult<Vec<f32>> {
    use std::arch::x86_64::*;

    if logits.is_empty() {
        return Ok(Vec::new());
    }

    let len = logits.len();
    let mut maxv = _mm_set1_ps(f32::NEG_INFINITY);
    let mut i = 0;
    while i + 4 <= len {
        let v = _mm_loadu_ps(logits.as_ptr().add(i));
        maxv = _mm_max_ps(maxv, v);
        i += 4;
    }
    let mut max_arr = [f32::NEG_INFINITY; 4];
    _mm_storeu_ps(max_arr.as_mut_ptr(), maxv);
    let mut max_scalar = max_arr.iter().fold(f32::NEG_INFINITY, |acc, &x| acc.max(x));
    for &v in &logits[i..] {
        max_scalar = max_scalar.max(v);
    }

    let max_b = _mm_set1_ps(max_scalar);
    let mut sum = 0.0f32;
    let mut result = vec![0.0f32; len];
    i = 0;
    while i + 4 <= len {
        let v = _mm_loadu_ps(logits.as_ptr().add(i));
        let shifted = _mm_sub_ps(v, max_b);

        // Fast exp approximation
        let log2e = _mm_set1_ps(1.442695041f32);
        let c127 = _mm_set1_ps(127.0f32);
        let tmp = _mm_mul_ps(shifted, log2e);
        let y = _mm_add_ps(tmp, c127);
        let exp_i = _mm_cvttps_epi32(y);
        let exp_f = _mm_castsi128_ps(_mm_slli_epi32(exp_i, 23));

        _mm_storeu_ps(result.as_mut_ptr().add(i), exp_f);
        let chunk_sum = {
            let mut arr = [0.0; 4];
            _mm_storeu_ps(arr.as_mut_ptr(), exp_f);
            arr.iter().sum::<f32>()
        };
        sum += chunk_sum;
        i += 4;
    }
    for j in i..len {
        let shifted = logits[j] - max_scalar;
        let exp_val = shifted.exp();
        result[j] = exp_val;
        sum += exp_val;
    }

    let inv_sum = 1.0 / sum;
    let inv_sum_b = _mm_set1_ps(inv_sum);
    i = 0;
    while i + 4 <= len {
        let v = _mm_loadu_ps(result.as_ptr().add(i));
        let norm = _mm_mul_ps(v, inv_sum_b);
        _mm_storeu_ps(result.as_mut_ptr().add(i), norm);
        i += 4;
    }
    for result_item in result.iter_mut().take(len).skip(i) {
        *result_item *= inv_sum;
    }

    Ok(result)
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn neon_softmax(logits: &[f32]) -> SimdResult<Vec<f32>> {
    use std::arch::aarch64::*;

    if logits.is_empty() {
        return Ok(Vec::new());
    }

    let len = logits.len();
    let mut maxv = vdupq_n_f32(f32::NEG_INFINITY);
    let mut i = 0;
    while i + 4 <= len {
        let v = unsafe { vld1q_f32(logits.as_ptr().add(i)) };
        maxv = vmaxq_f32(maxv, v);
        i += 4;
    }
    let mut max_arr = [f32::NEG_INFINITY; 4];
    unsafe { vst1q_f32(max_arr.as_mut_ptr(), maxv) };
    let mut max_scalar = max_arr.iter().fold(f32::NEG_INFINITY, |acc, &x| acc.max(x));
    for &v in &logits[i..] {
        max_scalar = max_scalar.max(v);
    }

    let max_b = vdupq_n_f32(max_scalar);
    let mut sum = 0.0f32;
    let mut result = vec![0.0f32; len];
    i = 0;
    while i + 4 <= len {
        let v = unsafe { vld1q_f32(logits.as_ptr().add(i)) };
        let shifted = vsubq_f32(v, max_b);

        // Fast exp approximation for NEON
        let log2e = vdupq_n_f32(std::f32::consts::LOG2_E);
        let c127 = vdupq_n_f32(127.0f32);
        let tmp = vmulq_f32(shifted, log2e);
        let y = vaddq_f32(tmp, c127);
        let exp_i = vcvtq_s32_f32(y);
        let exp_shift = vshlq_n_s32(exp_i, 23);
        let exp_f = vcvtq_f32_s32(exp_shift);

        unsafe { vst1q_f32(result.as_mut_ptr().add(i), exp_f) };
        let chunk_sum = {
            let mut arr = [0.0; 4];
            unsafe { vst1q_f32(arr.as_mut_ptr(), exp_f) };
            arr.iter().sum::<f32>()
        };
        sum += chunk_sum;
        i += 4;
    }
    for j in i..len {
        let shifted = logits[j] - max_scalar;
        let exp_val = shifted.exp();
        result[j] = exp_val;
        sum += exp_val;
    }

    let inv_sum = 1.0 / sum;
    let inv_sum_b = vdupq_n_f32(inv_sum);
    i = 0;
    while i + 4 <= len {
        let v = unsafe { vld1q_f32(result.as_ptr().add(i)) };
        let norm = vmulq_f32(v, inv_sum_b);
        unsafe { vst1q_f32(result.as_mut_ptr().add(i), norm) };
        i += 4;
    }
    for result_item in result.iter_mut().take(len).skip(i) {
        *result_item *= inv_sum;
    }

    Ok(result)
}

/// Computes softmax over a slice of logits using the best available implementation.
/// Returns a Vec with softmax probabilities.
/// Dispatch table for softmax operations across different CPU capabilities
pub static SOFTMAX_DISPATCH: Lazy<SoftmaxDispatch> = Lazy::new(create_softmax_dispatch);

fn create_softmax_dispatch() -> SoftmaxDispatch {
    SoftmaxDispatch {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        avx512: Some(avx512_softmax),
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        avx512: None,

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        avx2: Some(avx2_softmax),
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        avx2: None,

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        sse41: Some(sse41_softmax),
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        sse41: None,

        #[cfg(target_arch = "aarch64")]
        neon: Some(neon_softmax),
        #[cfg(not(target_arch = "aarch64"))]
        neon: None,

        scalar: scalar_softmax,
    }
}

/// Compute softmax over logits using the best available SIMD implementation
///
/// This function automatically selects the optimal SIMD implementation based on runtime
/// CPU feature detection, providing maximum performance across different hardware.
///
/// # Arguments
/// * `logits` - Input logits slice
///
/// # Returns
/// * `SimdResult<Vec<f32>>` - Normalized probabilities or error
pub fn softmax(logits: &[f32]) -> SimdResult<Vec<f32>> {
    SOFTMAX_DISPATCH.call(logits)
}
