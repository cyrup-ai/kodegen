//! SIMD performance monitoring and metrics collection
//!
//! This module provides specialized metrics and monitoring for SIMD acceleration
//! in text generation, tracking utilization rates, performance gains, and
//! optimization opportunities.

use std::time::Duration;

use super::stats::GenerationStatistics;

/// SIMD-specific performance metrics
#[derive(Debug, Clone, Default)]
pub struct SimdMetrics {
    /// Number of SIMD temperature operations
    pub simd_temperature_ops: u64,

    /// Number of SIMD softmax operations
    pub simd_softmax_ops: u64,

    /// Number of SIMD argmax operations
    pub simd_argmax_ops: u64,

    /// Number of SIMD top-k operations
    pub simd_topk_ops: u64,

    /// Number of SIMD nucleus sampling operations
    pub simd_nucleus_ops: u64,

    /// Number of SIMD penalty operations
    pub simd_penalty_ops: u64,

    /// Total time spent in SIMD operations
    pub simd_duration: Duration,

    /// Total time spent in scalar fallbacks
    pub scalar_duration: Duration,
}

impl SimdMetrics {
    /// Create new SIMD metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a SIMD temperature operation
    pub fn record_temperature_op(&mut self) {
        self.simd_temperature_ops += 1;
    }

    /// Record a SIMD softmax operation
    pub fn record_softmax_op(&mut self) {
        self.simd_softmax_ops += 1;
    }

    /// Record a SIMD argmax operation
    pub fn record_argmax_op(&mut self) {
        self.simd_argmax_ops += 1;
    }
    /// Record a SIMD top-k operation
    pub fn record_topk_op(&mut self) {
        self.simd_topk_ops += 1;
    }

    /// Record a SIMD nucleus sampling operation
    pub fn record_nucleus_op(&mut self) {
        self.simd_nucleus_ops += 1;
    }

    /// Record a SIMD penalty operation
    pub fn record_penalty_op(&mut self) {
        self.simd_penalty_ops += 1;
    }

    /// Add SIMD execution time
    pub fn add_simd_duration(&mut self, duration: Duration) {
        self.simd_duration += duration;
    }

    /// Add scalar execution time
    pub fn add_scalar_duration(&mut self, duration: Duration) {
        self.scalar_duration += duration;
    }

    /// Get total SIMD operations
    pub fn total_simd_ops(&self) -> u64 {
        self.simd_temperature_ops
            + self.simd_softmax_ops
            + self.simd_argmax_ops
            + self.simd_topk_ops
            + self.simd_nucleus_ops
            + self.simd_penalty_ops
    }

    /// Calculate SIMD speedup ratio vs scalar
    pub fn speedup_ratio(&self) -> f64 {
        if self.scalar_duration.as_secs_f64() > 0.0 {
            self.scalar_duration.as_secs_f64() / self.simd_duration.as_secs_f64()
        } else {
            1.0
        }
    }
    /// Get performance report as formatted string
    pub fn performance_report(&self) -> String {
        format!(
            "SIMD Ops: {} | Speedup: {:.2}x | Temp: {} | Softmax: {} | ArgMax: {} | TopK: {} | Nucleus: {} | Penalties: {}",
            self.total_simd_ops(),
            self.speedup_ratio(),
            self.simd_temperature_ops,
            self.simd_softmax_ops,
            self.simd_argmax_ops,
            self.simd_topk_ops,
            self.simd_nucleus_ops,
            self.simd_penalty_ops
        )
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Merge metrics from another SimdMetrics instance
    pub fn merge(&mut self, other: &SimdMetrics) {
        self.simd_temperature_ops += other.simd_temperature_ops;
        self.simd_softmax_ops += other.simd_softmax_ops;
        self.simd_argmax_ops += other.simd_argmax_ops;
        self.simd_topk_ops += other.simd_topk_ops;
        self.simd_nucleus_ops += other.simd_nucleus_ops;
        self.simd_penalty_ops += other.simd_penalty_ops;
        self.simd_duration += other.simd_duration;
        self.scalar_duration += other.scalar_duration;
    }
}

/// Integration utilities for SIMD metrics with GenerationStatistics
impl SimdMetrics {
    /// Update GenerationStatistics with SIMD metrics
    pub fn update_generation_stats(&self, stats: &mut GenerationStatistics) {
        // Add total SIMD operations to stats
        let total_simd = self.total_simd_ops();
        for _ in 0..total_simd {
            stats.record_simd_operation();
        }
    }
}
