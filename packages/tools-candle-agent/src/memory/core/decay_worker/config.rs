//! Decay worker configuration

use serde::{Deserialize, Serialize};

/// Configuration for background decay worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayWorkerConfig {
    /// Sleep interval between decay cycles (seconds)
    pub cycle_interval_secs: u64,

    /// Number of memories to process per batch
    pub batch_size: usize,

    /// Minimum memory age before applying decay (hours)
    /// Prevents thrashing on fresh memories
    pub min_age_hours: u64,
}

impl Default for DecayWorkerConfig {
    fn default() -> Self {
        Self {
            cycle_interval_secs: 3600, // 1 hour between cycles
            batch_size: 500,           // Process 500 memories per batch
            min_age_hours: 1,          // Apply decay to memories older than 1 hour
        }
    }
}
