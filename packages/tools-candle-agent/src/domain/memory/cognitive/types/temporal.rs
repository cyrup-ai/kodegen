//! Temporal context and causal dependency tracking for time-aware memory operations

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Temporal context for time-aware memory operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContext {
    /// Start time of temporal window
    pub window_start: SystemTime,
    /// End time of temporal window
    pub window_end: SystemTime,
    /// Duration of temporal window in seconds
    pub window_duration_secs: u64,
    /// Causal links between memories
    pub causal_links: Vec<CausalLink>,
    /// Last consolidation timestamp
    pub last_consolidation: SystemTime,
}

impl TemporalContext {
    /// Create new temporal context with specified window duration
    #[must_use]
    pub fn new(window_duration_secs: u64) -> Self {
        let window_start = SystemTime::now();
        let window_end = window_start + std::time::Duration::from_secs(window_duration_secs);
        Self {
            window_start,
            window_end,
            window_duration_secs,
            causal_links: Vec::new(),
            last_consolidation: SystemTime::now(),
        }
    }

    /// Add causal link between memories
    pub fn add_causal_link(&mut self, link: CausalLink) {
        self.causal_links.push(link);
    }

    /// Check if a timestamp is within the temporal window
    #[must_use]
    pub fn is_within_window(&self, timestamp: SystemTime) -> bool {
        timestamp >= self.window_start && timestamp <= self.window_end
    }

    /// Shift temporal window forward
    pub fn shift_window_forward(&mut self) {
        let duration = std::time::Duration::from_secs(self.window_duration_secs);
        self.window_start = self.window_end;
        self.window_end = self.window_start + duration;
    }

    /// Get causal predecessors of a memory
    #[must_use]
    pub fn get_causal_predecessors(&self, memory_id: Uuid) -> Vec<Uuid> {
        self.causal_links
            .iter()
            .filter(|link| link.target_id == memory_id)
            .map(|link| link.source_id)
            .collect()
    }

    /// Get causal successors of a memory
    #[must_use]
    pub fn get_causal_successors(&self, memory_id: Uuid) -> Vec<Uuid> {
        self.causal_links
            .iter()
            .filter(|link| link.source_id == memory_id)
            .map(|link| link.target_id)
            .collect()
    }
}

impl Default for TemporalContext {
    fn default() -> Self {
        Self::new(3600) // Default 1-hour window
    }
}

/// Causal link between memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalLink {
    /// Source event ID
    pub source_id: Uuid,
    /// Target event ID
    pub target_id: Uuid,
    /// Causal strength (0.0 to 1.0)
    pub strength: f32,
    /// Temporal distance in milliseconds
    pub temporal_distance: i64,
}

impl CausalLink {
    /// Create new causal link
    #[inline]
    #[must_use]
    pub fn new(source_id: Uuid, target_id: Uuid, strength: f32, temporal_distance: i64) -> Self {
        Self {
            source_id,
            target_id,
            strength: strength.clamp(0.0, 1.0),
            temporal_distance,
        }
    }
}
