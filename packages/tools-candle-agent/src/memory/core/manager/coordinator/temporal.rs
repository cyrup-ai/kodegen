//! Temporal decay operations for memory importance

use crate::memory::utils::{Error, Result};

use super::lifecycle::MemoryCoordinator;

impl MemoryCoordinator {
    /// Apply temporal decay to memory importance based on age
    ///
    /// This method calculates exponential decay based on memory age and updates:
    /// - Memory importance score (reduced over time)
    /// - Quantum coherence level (simulating decoherence)
    pub(in crate::memory::core) async fn apply_temporal_decay(
        &self,
        memory: &mut crate::domain::memory::primitives::node::MemoryNode,
    ) -> Result<()> {
        // Convert SystemTime to DateTime<Utc> for calculation
        let created_time = chrono::DateTime::<chrono::Utc>::from(memory.base_memory.created_at);
        let now = chrono::Utc::now();

        // Calculate age of memory
        let age = now.signed_duration_since(created_time);

        // Calculate days old with fractional precision
        let days_old = age.num_seconds() as f64 / 86400.0; // seconds per day

        // Apply exponential decay: e^(-decay_rate * days)
        // Using self.decay_rate for configurability
        let decay = (-self.decay_rate * days_old).exp();

        // Apply decay to importance (ensure it doesn't go below minimum threshold)
        let current_importance = memory.importance();
        let new_importance = (current_importance * decay as f32).max(0.01); // Minimum threshold 0.01

        // Update memory importance using existing setter
        memory
            .set_importance(new_importance)
            .map_err(|e| Error::Internal(format!("Failed to set decayed importance: {}", e)))?;

        // Update last_accessed_at in metadata to track access patterns
        memory.stats.record_read();

        log::trace!(
            "Temporal decay applied to {}: importance {} -> {} (age: {:.2} days)",
            memory.id(),
            current_importance,
            new_importance,
            days_old
        );

        Ok(())
    }

    /// Apply temporal decay to core memory node (for internal use)
    ///
    /// Note: Currently unused as architecture applies decay at domain layer.
    /// Retained for future optimization where decay might be applied to core nodes
    /// before domain conversion to reduce conversion overhead.
    #[allow(dead_code)]
    pub(super) async fn apply_temporal_decay_core(
        &self,
        memory: &mut crate::memory::core::primitives::node::MemoryNode,
    ) -> Result<()> {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(memory.created_at);

        // Calculate days old with fractional precision
        let days_old = age.num_seconds() as f64 / 86400.0;

        // Apply exponential decay
        let decay = (-self.decay_rate * days_old).exp();

        // Apply decay to importance
        memory.metadata.importance = (memory.metadata.importance as f64 * decay).max(0.01) as f32;

        // Update last_accessed_at
        memory.metadata.last_accessed_at = Some(now);

        Ok(())
    }
}
