//! Resolution context and cycle detection for anchor processing
//!
//! Provides context management and comprehensive cycle detection during
//! anchor and alias resolution with configurable depth limits.

use crate::semantic::SemanticError;
use std::collections::HashSet;

/// Alias resolution context for cycle detection
#[derive(Debug, Clone)]
pub struct ResolutionContext {
    pub current_depth: usize,
    pub max_depth: usize,
    pub resolution_path: Vec<String>,
    visited_anchors: HashSet<String>,
    pub alias_count: usize,
}

impl Default for ResolutionContext {
    fn default() -> Self {
        Self {
            current_depth: 0,
            max_depth: 100, // Default maximum depth to prevent infinite recursion
            resolution_path: Vec::with_capacity(16),
            visited_anchors: HashSet::with_capacity(16),
            alias_count: 0,
        }
    }
}

impl ResolutionContext {
    /// Create new resolution context with custom max depth
    #[must_use] 
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            current_depth: 0,
            max_depth,
            resolution_path: Vec::with_capacity(16),
            visited_anchors: HashSet::with_capacity(16),
            alias_count: 0,
        }
    }

    /// Create new resolution context with custom configuration
    #[must_use] 
    pub fn with_config(max_depth: usize, initial_capacity: usize) -> Self {
        Self {
            current_depth: 0,
            max_depth,
            resolution_path: Vec::with_capacity(initial_capacity),
            visited_anchors: HashSet::with_capacity(initial_capacity),
            alias_count: 0,
        }
    }

    /// Check if at maximum depth
    #[inline]
    #[must_use] 
    pub const fn at_max_depth(&self) -> bool {
        self.current_depth >= self.max_depth
    }

    /// Check if depth is near maximum (within warning threshold)
    #[must_use] 
    pub const fn near_max_depth(&self, warning_threshold: usize) -> bool {
        self.current_depth + warning_threshold >= self.max_depth
    }

    /// Enter new resolution level
    pub fn enter(&mut self, anchor_name: String) -> Result<(), SemanticError> {
        // Check for maximum depth
        if self.at_max_depth() {
            return Err(SemanticError::validation_depth_exceeded(
                self.max_depth,
                self.resolution_path.len(),
                crate::lexer::Position::default(),
            ));
        }

        // Check for circular reference
        if self.visited_anchors.contains(&anchor_name) {
            let mut cycle_path = self.resolution_path.clone();
            cycle_path.push(anchor_name.clone());
            return Err(SemanticError::circular_reference(
                anchor_name,
                cycle_path.join(" -> "),
                crate::lexer::Position::default(), // Will be updated by caller with actual position
            ));
        }

        // Update context state
        self.current_depth += 1;
        self.resolution_path.push(anchor_name.clone());
        self.visited_anchors.insert(anchor_name);
        self.alias_count += 1;

        Ok(())
    }

    /// Exit current resolution level
    pub fn exit(&mut self) {
        if self.current_depth > 0 {
            self.current_depth -= 1;

            if let Some(anchor_name) = self.resolution_path.pop() {
                self.visited_anchors.remove(&anchor_name);
            }
        }
    }

    /// Check if anchor is currently being resolved (in the resolution path)
    #[inline]
    #[must_use] 
    pub fn is_resolving(&self, anchor_name: &str) -> bool {
        self.visited_anchors.contains(anchor_name)
    }

    /// Get current resolution path as string
    #[must_use] 
    pub fn path_string(&self) -> String {
        self.resolution_path.join(" -> ")
    }

    /// Get remaining depth before hitting maximum
    #[inline]
    #[must_use] 
    pub const fn remaining_depth(&self) -> usize {
        self.max_depth.saturating_sub(self.current_depth)
    }

    /// Reset context for new resolution
    pub fn reset(&mut self) {
        self.current_depth = 0;
        self.resolution_path.clear();
        self.visited_anchors.clear();
        self.alias_count = 0;
    }

    /// Create snapshot of current context state
    #[must_use] 
    pub fn snapshot(&self) -> ResolutionSnapshot {
        ResolutionSnapshot {
            depth: self.current_depth,
            path: self.resolution_path.clone(),
            visited_count: self.visited_anchors.len(),
            alias_count: self.alias_count,
        }
    }

    /// Check if context is in valid state
    #[must_use] 
    pub fn is_valid(&self) -> bool {
        self.current_depth <= self.max_depth
            && self.resolution_path.len() == self.current_depth
            && self.visited_anchors.len() == self.current_depth
    }

    /// Get context statistics
    #[must_use] 
    pub fn statistics(&self) -> ContextStatistics {
        ContextStatistics {
            current_depth: self.current_depth,
            max_depth: self.max_depth,
            path_length: self.resolution_path.len(),
            visited_count: self.visited_anchors.len(),
            alias_count: self.alias_count,
            depth_utilization: if self.max_depth > 0 {
                self.current_depth as f64 / self.max_depth as f64
            } else {
                0.0
            },
        }
    }

    /// Validate context configuration
    pub const fn validate_config(&self) -> Result<(), ContextValidationError> {
        if self.max_depth == 0 {
            return Err(ContextValidationError::InvalidMaxDepth(self.max_depth));
        }

        if self.max_depth > 10000 {
            return Err(ContextValidationError::ExcessiveMaxDepth(self.max_depth));
        }

        Ok(())
    }

    /// Estimate memory usage of current context
    #[must_use] 
    pub fn estimated_memory_usage(&self) -> usize {
        // Rough estimation in bytes
        let path_memory = self
            .resolution_path
            .iter()
            .map(|s| s.len() + std::mem::size_of::<String>())
            .sum::<usize>();

        let visited_memory = self
            .visited_anchors
            .iter()
            .map(|s| s.len() + std::mem::size_of::<String>())
            .sum::<usize>();

        std::mem::size_of::<Self>() + path_memory + visited_memory
    }
}

/// Snapshot of resolution context state
#[derive(Debug, Clone)]
pub struct ResolutionSnapshot {
    pub depth: usize,
    pub path: Vec<String>,
    pub visited_count: usize,
    pub alias_count: usize,
}

impl std::fmt::Display for ResolutionSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ResolutionSnapshot {{ depth: {}, path: [{}], visited: {}, aliases: {} }}",
            self.depth,
            self.path.join(" -> "),
            self.visited_count,
            self.alias_count
        )
    }
}

/// Resolution context statistics
#[derive(Debug, Clone, Copy)]
pub struct ContextStatistics {
    pub current_depth: usize,
    pub max_depth: usize,
    pub path_length: usize,
    pub visited_count: usize,
    pub alias_count: usize,
    pub depth_utilization: f64,
}

impl ContextStatistics {
    /// Check if context is approaching limits
    #[must_use] 
    pub fn is_approaching_limits(&self, threshold: f64) -> bool {
        self.depth_utilization >= threshold
    }

    /// Get efficiency score (lower is better)
    #[must_use] 
    pub fn efficiency_score(&self) -> f64 {
        if self.alias_count > 0 {
            self.current_depth as f64 / self.alias_count as f64
        } else {
            0.0
        }
    }
}

/// Context validation errors
#[derive(Debug, Clone)]
pub enum ContextValidationError {
    InvalidMaxDepth(usize),
    ExcessiveMaxDepth(usize),
    InconsistentState {
        depth: usize,
        path_length: usize,
        visited_count: usize,
    },
}

impl ContextValidationError {
    /// Get error message
    #[must_use] 
    pub fn message(&self) -> String {
        match self {
            Self::InvalidMaxDepth(depth) => {
                format!("Invalid maximum depth: {depth}")
            }
            Self::ExcessiveMaxDepth(depth) => {
                format!("Excessive maximum depth: {depth} (should be <= 10000)")
            }
            Self::InconsistentState {
                depth,
                path_length,
                visited_count,
            } => {
                format!(
                    "Inconsistent context state: depth={depth}, path_length={path_length}, visited_count={visited_count}"
                )
            }
        }
    }
}

/// Resolution context builder for advanced configuration
#[derive(Debug)]
pub struct ResolutionContextBuilder {
    max_depth: usize,
    initial_capacity: usize,
    warning_threshold: Option<usize>,
}

impl ResolutionContextBuilder {
    /// Create new context builder
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            max_depth: 100,
            initial_capacity: 16,
            warning_threshold: None,
        }
    }

    /// Set maximum resolution depth
    #[must_use] 
    pub const fn max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Set initial capacity for collections
    #[must_use] 
    pub const fn initial_capacity(mut self, capacity: usize) -> Self {
        self.initial_capacity = capacity;
        self
    }

    /// Set warning threshold for depth monitoring
    #[must_use] 
    pub const fn warning_threshold(mut self, threshold: usize) -> Self {
        self.warning_threshold = Some(threshold);
        self
    }

    /// Build resolution context
    #[must_use] 
    pub fn build(self) -> ResolutionContext {
        ResolutionContext::with_config(self.max_depth, self.initial_capacity)
    }
}

impl Default for ResolutionContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_context_creation() {
        let context = ResolutionContext::default();
        assert_eq!(context.current_depth, 0);
        assert_eq!(context.max_depth, 100);
        assert!(context.resolution_path.is_empty());
        assert!(context.visited_anchors.is_empty());
    }

    #[test]
    fn test_resolution_context_enter_exit() {
        let mut context = ResolutionContext::default();

        // Enter first level
        assert!(context.enter("anchor1".to_string()).is_ok());
        assert_eq!(context.current_depth, 1);
        assert_eq!(context.resolution_path.len(), 1);
        assert!(context.visited_anchors.contains("anchor1"));

        // Enter second level
        assert!(context.enter("anchor2".to_string()).is_ok());
        assert_eq!(context.current_depth, 2);

        // Exit levels
        context.exit();
        assert_eq!(context.current_depth, 1);
        assert!(!context.visited_anchors.contains("anchor2"));

        context.exit();
        assert_eq!(context.current_depth, 0);
        assert!(!context.visited_anchors.contains("anchor1"));
    }

    #[test]
    fn test_circular_reference_detection() {
        let mut context = ResolutionContext::default();

        // Enter anchor1
        assert!(context.enter("anchor1".to_string()).is_ok());

        // Try to enter anchor1 again (circular reference)
        let result = context.enter("anchor1".to_string());
        assert!(result.is_err());

        if let Err(SemanticError::CircularReference { .. }) = result {
            // Expected error type
        } else {
            panic!("Expected CircularReference error");
        }
    }

    #[test]
    fn test_max_depth_enforcement() {
        let mut context = ResolutionContext::with_max_depth(2);

        assert!(context.enter("anchor1".to_string()).is_ok());
        assert!(context.enter("anchor2".to_string()).is_ok());

        // Should fail at max depth
        let result = context.enter("anchor3".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_context_builder() {
        let context = ResolutionContextBuilder::new()
            .max_depth(50)
            .initial_capacity(32)
            .build();

        assert_eq!(context.max_depth, 50);
    }
}
