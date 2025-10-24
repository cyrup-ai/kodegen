//! SIMD accelerated Argmax operations for cyrup
//!
//! This module provides vectorized implementations of argmax operations
//! optimized for AI inference tasks, focusing on zero-allocation and lock-free design.

use once_cell::sync::Lazy;

use crate::error::SimdResult;
use crate::runtime::ArgmaxDispatch;

/// Scalar implementation of argmax as a fallback
fn scalar_argmax(logits: &[f32]) -> SimdResult<usize> {
    if logits.is_empty() {
        return Err(crate::error::SimdError::InvalidInput(
            "Logits slice is empty".to_string(),
        ));
    }

    let mut max_idx = 0;
    let mut max_val = logits[0];

    for (i, &val) in logits.iter().enumerate().skip(1) {
        if val > max_val {
            max_val = val;
            max_idx = i;
        }
    }

    Ok(max_idx)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn avx2_argmax(logits: &[f32]) -> SimdResult<usize> {
    use std::arch::x86_64::*;

    if logits.is_empty() {
        return Err(crate::error::SimdError::InvalidInput(
            "Logits slice is empty".to_string(),
        ));
    }

    let len = logits.len();
    let mut maxv = _mm256_set1_ps(f32::NEG_INFINITY);
    let mut max_idx = _mm256_setzero_si256(); // Will store indices
    let indices_step = _mm256_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7);
    let mut offset = _mm256_setzero_si256();
    let mut i = 0;

    while i + 8 <= len {
        let v = _mm256_loadu_ps(logits.as_ptr().add(i));
        let mask = _mm256_cmp_ps(v, maxv, _CMP_GT_OQ);
        maxv = _mm256_max_ps(maxv, v);
        let current_indices = _mm256_add_epi32(offset, indices_step);
        max_idx = _mm256_blendv_epi8(max_idx, current_indices, _mm256_castps_si256(mask));
        offset = _mm256_add_epi32(offset, _mm256_set1_epi32(8));
        i += 8;
    }

    // Reduce maxv and max_idx
    let mut max_arr = [f32::NEG_INFINITY; 8];
    let mut idx_arr = [0i32; 8];
    _mm256_storeu_ps(max_arr.as_mut_ptr(), maxv);
    _mm256_storeu_si256(idx_arr.as_mut_ptr() as *mut __m256i, max_idx);

    let mut max_scalar = f32::NEG_INFINITY;
    let mut best_idx: usize = 0;
    for j in 0..8 {
        if max_arr[j] > max_scalar {
            max_scalar = max_arr[j];
            best_idx = idx_arr[j] as usize;
        }
    }

    // Handle remainder
    for j in i..len {
        let val = logits[j];
        if val > max_scalar {
            max_scalar = val;
            best_idx = j;
        }
    }

    Ok(best_idx)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn avx512_argmax(logits: &[f32]) -> SimdResult<usize> {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    if logits.is_empty() {
        return Err(crate::error::SimdError::InvalidInput(
            "Logits slice is empty".to_string(),
        ));
    }

    let len = logits.len();
    let mut maxv = _mm512_set1_ps(f32::NEG_INFINITY);
    let mut max_idx = _mm512_setzero_si512();
    let indices_step = _mm512_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
    let mut offset = _mm512_setzero_si512();
    let mut i = 0;

    while i + 16 <= len {
        let v = _mm512_loadu_ps(logits.as_ptr().add(i));
        let mask = _mm512_cmp_ps_mask(v, maxv, _CMP_GT_OQ);
        maxv = _mm512_max_ps(maxv, v);
        let current_indices = _mm512_add_epi32(offset, indices_step);
        max_idx = _mm512_mask_blend_epi32(mask, max_idx, current_indices);
        offset = _mm512_add_epi32(offset, _mm512_set1_epi32(16));
        i += 16;
    }

    // Reduce maxv and max_idx
    let mut max_arr = [f32::NEG_INFINITY; 16];
    let mut idx_arr = [0i32; 16];
    _mm512_storeu_ps(max_arr.as_mut_ptr(), maxv);
    _mm512_storeu_si512(idx_arr.as_mut_ptr() as *mut __m512i, max_idx);

    let mut max_scalar = f32::NEG_INFINITY;
    let mut best_idx: usize = 0;
    for j in 0..16 {
        if max_arr[j] > max_scalar {
            max_scalar = max_arr[j];
            best_idx = idx_arr[j] as usize;
        }
    }

    // Handle remainder
    for j in i..len {
        let val = logits[j];
        if val > max_scalar {
            max_scalar = val;
            best_idx = j;
        }
    }

    Ok(best_idx)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.1")]
unsafe fn sse41_argmax(logits: &[f32]) -> SimdResult<usize> {
    use std::arch::x86_64::*;

    if logits.is_empty() {
        return Err(crate::error::SimdError::InvalidInput(
            "Logits slice is empty".to_string(),
        ));
    }

    let len = logits.len();
    let mut maxv = _mm_set1_ps(f32::NEG_INFINITY);
    let mut max_idx = _mm_setzero_si128(); // Will store indices
    let indices_step = _mm_setr_epi32(0, 1, 2, 3);
    let mut offset = _mm_setzero_si128();
    let mut i = 0;

    while i + 4 <= len {
        let v = _mm_loadu_ps(logits.as_ptr().add(i));
        let mask = _mm_cmpgt_ps(v, maxv);
        maxv = _mm_max_ps(maxv, v);
        let current_indices = _mm_add_epi32(offset, indices_step);
        max_idx = _mm_blendv_epi8(max_idx, current_indices, _mm_castps_si128(mask));
        offset = _mm_add_epi32(offset, _mm_set1_epi32(4));
        i += 4;
    }

    // Reduce maxv and max_idx
    let mut max_arr = [f32::NEG_INFINITY; 4];
    let mut idx_arr = [0i32; 4];
    _mm_storeu_ps(max_arr.as_mut_ptr(), maxv);
    _mm_storeu_si128(idx_arr.as_mut_ptr() as *mut __m128i, max_idx);

    let mut max_scalar = f32::NEG_INFINITY;
    let mut best_idx: usize = 0;
    for j in 0..4 {
        if max_arr[j] > max_scalar {
            max_scalar = max_arr[j];
            best_idx = idx_arr[j] as usize;
        }
    }

    // Handle remainder
    for j in i..len {
        let val = logits[j];
        if val > max_scalar {
            max_scalar = val;
            best_idx = j;
        }
    }

    Ok(best_idx)
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn neon_argmax(logits: &[f32]) -> SimdResult<usize> {
    if logits.is_empty() {
        return Err(crate::error::SimdError::InvalidInput(
            "Logits slice is empty".to_string(),
        ));
    }

    // For NEON, fall back to scalar implementation for correct index tracking
    // The SIMD complexity of tracking indices correctly isn't worth it for most use cases
    scalar_argmax(logits)
}

fn create_argmax_dispatch() -> ArgmaxDispatch {
    ArgmaxDispatch {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        avx512: Some(avx512_argmax),
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        avx512: None,

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        avx2: Some(avx2_argmax),
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        avx2: None,

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        sse41: Some(sse41_argmax),
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        sse41: None,

        #[cfg(target_arch = "aarch64")]
        neon: Some(neon_argmax),
        #[cfg(not(target_arch = "aarch64"))]
        neon: None,

        scalar: scalar_argmax,
    }
}

/// Global dispatch table for argmax operations with runtime CPU feature detection
pub static ARGMAX_DISPATCH: Lazy<ArgmaxDispatch> = Lazy::new(create_argmax_dispatch);

/// Computes argmax over a slice of logits using the best available implementation.
/// Returns the index of the maximum value.
pub fn argmax(logits: &[f32]) -> SimdResult<usize> {
    ARGMAX_DISPATCH.call(logits)
}
