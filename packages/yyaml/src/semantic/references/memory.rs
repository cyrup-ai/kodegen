//! Memory management for reference tracking
//!
//! Provides zero-allocation, blazing-fast memory management for reference nodes
//! with efficient pooling, garbage collection, and fragmentation handling.

use super::types::{MemoryUsage, ReferenceNode};
use std::collections::VecDeque;

/// Memory manager for efficient reference storage
#[derive(Debug)]
pub struct MemoryManager<'input> {
    reference_pool: Vec<ReferenceNode<'input>>,
    free_indices: VecDeque<usize>,
    memory_usage: MemoryUsage,
    gc_threshold: usize,
    compact_threshold: f64,
}

impl<'input> MemoryManager<'input> {
    /// Create new memory manager with optimized settings
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            reference_pool: Vec::with_capacity(256), // Pre-allocate for performance
            free_indices: VecDeque::with_capacity(64),
            memory_usage: MemoryUsage::default(),
            gc_threshold: 1000,
            compact_threshold: 0.3, // Compact when fragmentation > 30%
        }
    }

    /// Allocate reference node - blazing-fast with pooling
    #[inline]
    pub fn allocate(&mut self, node: ReferenceNode<'input>) -> usize {
        if let Some(index) = self.free_indices.pop_front() {
            // Reuse freed slot - zero allocation
            self.reference_pool[index] = node;
            self.memory_usage.allocation_count += 1;
            self.update_memory_stats();
            index
        } else {
            // Allocate new slot
            let index = self.reference_pool.len();
            self.reference_pool.push(node);
            self.memory_usage.allocation_count += 1;
            self.update_memory_stats();
            index
        }
    }

    /// Deallocate reference node - efficient memory reclamation
    #[inline]
    pub fn deallocate(&mut self, index: usize) {
        if index < self.reference_pool.len() {
            self.free_indices.push_back(index);
            self.memory_usage.deallocation_count += 1;
            self.update_memory_stats();

            // Auto-compact only if compaction threshold is very low (aggressive mode)
            if self.compact_threshold < 0.1 && self.should_compact() {
                self.compact();
            }
        }
    }

    /// Compact memory by removing fragmentation - performance optimization
    pub fn compact(&mut self) {
        if self.free_indices.is_empty() {
            return; // Nothing to compact
        }

        // Sort free indices in descending order for efficient removal
        let mut free_indices: Vec<usize> = self.free_indices.drain(..).collect();
        free_indices.sort_by(|a, b| b.cmp(a));

        // Remove freed slots from the end first
        for &index in &free_indices {
            if index == self.reference_pool.len() - 1 {
                self.reference_pool.pop();
            } else {
                // Keep non-terminal free indices for reuse
                self.free_indices.push_back(index);
            }
        }

        self.update_memory_stats();
    }

    /// Run garbage collection - memory optimization
    #[inline]
    pub fn garbage_collect(&mut self) {
        if self.memory_usage.allocation_count >= self.gc_threshold {
            self.compact();
            // Reset counters after GC
            self.memory_usage.allocation_count = 0;
            self.memory_usage.deallocation_count = 0;
        }
    }

    /// Reset memory manager - complete cleanup
    #[inline]
    pub fn reset(&mut self) {
        self.reference_pool.clear();
        self.free_indices.clear();
        self.memory_usage = MemoryUsage::default();
    }

    /// Get memory statistics
    #[inline]
    #[must_use] 
    pub const fn get_memory_usage(&self) -> &MemoryUsage {
        &self.memory_usage
    }

    /// Get node by index with bounds checking
    #[inline]
    #[must_use] 
    pub fn get_node(&self, index: usize) -> Option<&ReferenceNode<'input>> {
        if index < self.reference_pool.len() && !self.free_indices.contains(&index) {
            Some(&self.reference_pool[index])
        } else {
            None
        }
    }

    /// Get mutable node by index with bounds checking
    #[inline]
    pub fn get_node_mut(&mut self, index: usize) -> Option<&mut ReferenceNode<'input>> {
        if index < self.reference_pool.len() && !self.free_indices.contains(&index) {
            Some(&mut self.reference_pool[index])
        } else {
            None
        }
    }

    /// Set garbage collection threshold
    #[inline]
    pub const fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_threshold = threshold;
    }

    /// Set compaction threshold (fragmentation ratio)
    #[inline]
    pub const fn set_compact_threshold(&mut self, threshold: f64) {
        self.compact_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Check if compaction should be triggered
    #[inline]
    #[must_use] 
    pub fn should_compact(&self) -> bool {
        self.memory_usage.fragmentation_ratio > self.compact_threshold
    }

    /// Update memory statistics - blazing-fast calculation
    #[inline]
    fn update_memory_stats(&mut self) {
        let total_slots = self.reference_pool.len();
        let free_slots = self.free_indices.len();
        let used_slots = total_slots - free_slots;

        // Estimate memory usage (approximate)
        let bytes_per_node = std::mem::size_of::<ReferenceNode>();
        self.memory_usage.total_bytes = total_slots * bytes_per_node;
        self.memory_usage.used_bytes = used_slots * bytes_per_node;
        self.memory_usage.free_bytes = free_slots * bytes_per_node;

        // Calculate fragmentation ratio
        self.memory_usage.fragmentation_ratio = if total_slots > 0 {
            free_slots as f64 / total_slots as f64
        } else {
            0.0
        };
    }

    /// Get current pool size
    #[inline]
    #[must_use] 
    pub const fn pool_size(&self) -> usize {
        self.reference_pool.len()
    }

    /// Get number of free slots
    #[inline]
    #[must_use] 
    pub fn free_slots(&self) -> usize {
        self.free_indices.len()
    }

    /// Get number of used slots
    #[inline]
    #[must_use] 
    pub fn used_slots(&self) -> usize {
        self.reference_pool.len() - self.free_indices.len()
    }

    /// Pre-allocate memory for expected number of nodes
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.reference_pool.reserve(additional);
    }

    /// Shrink memory to fit current usage
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.compact(); // Remove fragmentation first
        self.reference_pool.shrink_to_fit();
        self.free_indices.shrink_to_fit();
        self.update_memory_stats();
    }

    /// Check if memory manager is in a healthy state
    #[inline]
    #[must_use] 
    pub fn is_healthy(&self) -> bool {
        self.memory_usage.fragmentation_ratio < self.compact_threshold * 2.0
    }

    /// Get memory efficiency percentage
    #[inline]
    #[must_use] 
    pub fn efficiency(&self) -> f64 {
        if self.memory_usage.total_bytes > 0 {
            (self.memory_usage.used_bytes as f64 / self.memory_usage.total_bytes as f64) * 100.0
        } else {
            100.0
        }
    }
}

impl<'input> Default for MemoryManager<'input> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Position;
    use crate::semantic::references::types::{ReferenceId, ReferenceNodeType};
    use std::borrow::Cow;

    fn create_test_node(id: usize) -> ReferenceNode<'static> {
        ReferenceNode {
            id: ReferenceId(id),
            name: Cow::Borrowed("test"),
            node_type: ReferenceNodeType::Scalar {
                value: Cow::Borrowed("test_value"),
                scalar_type: crate::semantic::references::types::ScalarType::String,
            },
            position: Position::default(),
            reference_path: vec!["test".to_string()],
        }
    }

    #[test]
    fn test_memory_allocation() {
        let mut manager = MemoryManager::new();
        let node = create_test_node(1);

        let index = manager.allocate(node);
        assert_eq!(index, 0);
        assert_eq!(manager.used_slots(), 1);
        assert_eq!(manager.free_slots(), 0);
    }

    #[test]
    fn test_memory_deallocation() {
        let mut manager = MemoryManager::new();
        let node = create_test_node(1);

        let index = manager.allocate(node);
        manager.deallocate(index);

        assert_eq!(manager.free_slots(), 1);
        assert!(manager.get_node(index).is_none());
    }

    #[test]
    fn test_memory_reuse() {
        let mut manager = MemoryManager::new();

        // Allocate and deallocate
        let node1 = create_test_node(1);
        let index1 = manager.allocate(node1);
        manager.deallocate(index1);

        // Allocate again - should reuse the slot
        let node2 = create_test_node(2);
        let index2 = manager.allocate(node2);

        assert_eq!(index1, index2); // Same slot reused
    }

    #[test]
    fn test_compaction() {
        let mut manager = MemoryManager::new();
        manager.set_compact_threshold(0.0); // Always compact

        // Allocate several nodes
        for i in 0..5 {
            let node = create_test_node(i);
            manager.allocate(node);
        }

        // Deallocate the last node
        manager.deallocate(4);

        // Should auto-compact
        assert_eq!(manager.pool_size(), 4);
        assert_eq!(manager.free_slots(), 0);
    }
}
