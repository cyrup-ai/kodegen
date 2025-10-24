//! Atomic attention weight management for concurrent cognitive updates

use super::atomics::AtomicF32;
use std::sync::atomic::Ordering;

/// Atomic attention weights for concurrent cognitive updates
#[derive(Debug)]
pub struct AtomicAttentionWeights {
    /// Primary attention weight
    primary: AtomicF32,
    /// Secondary attention weight
    secondary: AtomicF32,
    /// Background attention weight
    background: AtomicF32,
    /// Meta-attention weight
    meta: AtomicF32,
}

impl AtomicAttentionWeights {
    /// Create new atomic attention weights
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            primary: AtomicF32::new(0.6),
            secondary: AtomicF32::new(0.3),
            background: AtomicF32::new(0.1),
            meta: AtomicF32::new(0.0),
        }
    }

    /// Update primary attention atomically
    #[inline]
    pub fn set_primary(&self, value: f32) {
        self.primary.store(value.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    /// Get primary attention value
    #[inline]
    pub fn primary(&self) -> f32 {
        self.primary.load(Ordering::Relaxed)
    }

    /// Update secondary attention atomically
    #[inline]
    pub fn set_secondary(&self, value: f32) {
        self.secondary
            .store(value.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    /// Get secondary attention value
    #[inline]
    pub fn secondary(&self) -> f32 {
        self.secondary.load(Ordering::Relaxed)
    }

    /// Update background attention atomically
    #[inline]
    pub fn set_background(&self, value: f32) {
        self.background
            .store(value.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    /// Get background attention value
    #[inline]
    pub fn background(&self) -> f32 {
        self.background.load(Ordering::Relaxed)
    }

    /// Update meta attention atomically
    #[inline]
    pub fn set_meta(&self, value: f32) {
        self.meta.store(value.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    /// Get meta attention value
    #[inline]
    pub fn meta(&self) -> f32 {
        self.meta.load(Ordering::Relaxed)
    }

    /// Normalize all weights to sum to 1.0
    pub fn normalize(&self) {
        let total = self.primary() + self.secondary() + self.background() + self.meta();
        if total > 0.0 {
            self.set_primary(self.primary() / total);
            self.set_secondary(self.secondary() / total);
            self.set_background(self.background() / total);
            self.set_meta(self.meta() / total);
        }
    }

    /// Update primary attention weight from normalized activation energy
    ///
    /// Maps activation energy [0, 1] to primary attention weight.
    /// Other weights are adjusted proportionally to maintain normalization.
    pub fn update_from_energy(&self, energy: f32) {
        let clamped = energy.clamp(0.0, 1.0);
        self.set_primary(clamped);

        // Reduce other weights proportionally
        let remaining = 1.0 - clamped;
        self.set_secondary(remaining * 0.5);
        self.set_background(remaining * 0.3);
        self.set_meta(remaining * 0.2);
    }
}

impl Default for AtomicAttentionWeights {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
