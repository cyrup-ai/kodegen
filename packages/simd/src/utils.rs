//! Utility functions for SIMD-accelerated operations

/// Check if SIMD operations are available on the current platform
#[inline]
#[must_use]
pub fn simd_available() -> bool {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        #[cfg(target_arch = "x86_64")]
        {
            is_x86_feature_detected!("sse4.1")
        }
        #[cfg(target_arch = "aarch64")]
        {
            std::arch::is_aarch64_feature_detected!("neon")
        }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        false
    }
}

/// Align a pointer to the specified alignment
#[inline]
#[must_use]
pub fn align_ptr<T>(ptr: *const T, align: usize) -> *const T {
    let addr = ptr as usize;
    let aligned = (addr + align - 1) & !(align - 1);
    aligned as *const T
}

/// Align a mutable pointer to the specified alignment
#[inline]
#[must_use]
pub fn align_ptr_mut<T>(ptr: *mut T, align: usize) -> *mut T {
    let addr = ptr as usize;
    let aligned = (addr + align - 1) & !(align - 1);
    aligned as *mut T
}
