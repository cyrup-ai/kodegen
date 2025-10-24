//! Command executor statistics with zero allocation accessors
//!
//! Provides statistics calculation for command execution tracking.

use serde::{Deserialize, Serialize};

/// Command executor statistics with zero allocation accessors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExecutorStats {
    /// Number of currently active executions
    pub active_executions: u64,
    /// Total number of executions started
    pub total_executions: u64,
    /// Number of successful executions
    pub successful_executions: u64,
    /// Number of failed executions
    pub failed_executions: u64,
}

impl CommandExecutorStats {
    /// Calculate success rate as percentage - zero allocation
    #[allow(clippy::cast_precision_loss)] // Acceptable for percentage calculations
    #[inline]
    #[must_use]
    pub const fn success_rate(&self) -> f64 {
        let completed = self.successful_executions + self.failed_executions;
        if completed == 0 {
            0.0
        } else {
            (self.successful_executions as f64 / completed as f64) * 100.0
        }
    }

    /// Calculate failure rate as percentage - zero allocation
    #[inline]
    #[must_use]
    pub const fn failure_rate(&self) -> f64 {
        100.0 - self.success_rate()
    }

    /// Get completion rate (completed vs total) - zero allocation
    #[allow(clippy::cast_precision_loss)] // Acceptable for percentage calculations
    #[inline]
    #[must_use]
    pub const fn completion_rate(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            let completed = self.successful_executions + self.failed_executions;
            (completed as f64 / self.total_executions as f64) * 100.0
        }
    }

    /// Check if any executions are currently active
    #[inline]
    #[must_use]
    pub const fn has_active_executions(&self) -> bool {
        self.active_executions > 0
    }

    /// Check if system is idle (no active executions)
    #[inline]
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        self.active_executions == 0
    }
}
