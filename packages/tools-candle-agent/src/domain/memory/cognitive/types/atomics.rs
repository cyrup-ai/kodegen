//! Atomic wrapper types for lock-free concurrent operations

use std::sync::atomic::{AtomicU64, Ordering};

/// Atomic f32 wrapper for concurrent operations
#[derive(Debug)]
pub struct AtomicF32 {
    inner: AtomicU64,
}

impl AtomicF32 {
    /// Create new atomic f32
    #[inline]
    #[must_use]
    pub fn new(value: f32) -> Self {
        Self {
            inner: AtomicU64::new(u64::from(value.to_bits())),
        }
    }

    /// Load value atomically
    #[inline]
    pub fn load(&self, ordering: Ordering) -> f32 {
        let bits = self.inner.load(ordering);
        let bits_u32 = u32::try_from(bits).unwrap_or(0);
        f32::from_bits(bits_u32)
    }

    /// Store value atomically
    #[inline]
    pub fn store(&self, value: f32, ordering: Ordering) {
        self.inner.store(u64::from(value.to_bits()), ordering);
    }
}

impl Default for AtomicF32 {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// Atomic f64 wrapper for concurrent operations
#[derive(Debug)]
pub struct AtomicF64 {
    inner: AtomicU64,
}

impl AtomicF64 {
    /// Create new atomic f64
    #[inline]
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self {
            inner: AtomicU64::new(value.to_bits()),
        }
    }

    /// Load value atomically
    #[inline]
    pub fn load(&self, ordering: Ordering) -> f64 {
        f64::from_bits(self.inner.load(ordering))
    }

    /// Store value atomically
    #[inline]
    pub fn store(&self, value: f64, ordering: Ordering) {
        self.inner.store(value.to_bits(), ordering);
    }
}

impl Default for AtomicF64 {
    #[inline]
    fn default() -> Self {
        Self::new(0.0)
    }
}
