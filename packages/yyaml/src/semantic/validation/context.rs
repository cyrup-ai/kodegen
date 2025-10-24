//! Validation context for maintaining state during validation

use std::collections::HashSet;

/// Validation context for maintaining state during validation
#[derive(Debug, Default)]
pub struct ValidationContext {
    pub current_depth: usize,
    pub max_depth: usize,
    pub visited_nodes: HashSet<usize>,
    pub validation_path: Vec<String>,
    pub error_count: usize,
    pub warning_count: usize,
}

impl ValidationContext {
    /// Create a new validation context with default settings
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            current_depth: 0,
            max_depth: 1000,
            visited_nodes: HashSet::new(),
            validation_path: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Create a validation context with a custom max depth
    #[inline]
    #[must_use] 
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            max_depth,
            ..Self::new()
        }
    }

    /// Push a new path segment to the validation path
    #[inline]
    pub fn push_path(&mut self, segment: String) {
        self.validation_path.push(segment);
        self.current_depth += 1;
    }

    /// Pop a path segment from the validation path
    #[inline]
    pub fn pop_path(&mut self) {
        self.validation_path.pop();
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }

    /// Check if we've exceeded the maximum depth
    #[inline]
    #[must_use] 
    pub const fn is_depth_exceeded(&self) -> bool {
        self.current_depth > self.max_depth
    }

    /// Mark a node as visited by its ID
    #[inline]
    pub fn mark_visited(&mut self, node_id: usize) -> bool {
        self.visited_nodes.insert(node_id)
    }

    /// Check if a node has been visited
    #[inline]
    #[must_use] 
    pub fn is_visited(&self, node_id: usize) -> bool {
        self.visited_nodes.contains(&node_id)
    }

    /// Increment error count
    #[inline]
    pub const fn increment_errors(&mut self) {
        self.error_count += 1;
    }

    /// Increment warning count
    #[inline]
    pub const fn increment_warnings(&mut self) {
        self.warning_count += 1;
    }

    /// Get the current validation path as a string
    #[inline]
    #[must_use] 
    pub fn current_path(&self) -> String {
        self.validation_path.join(".")
    }

    /// Reset the context for reuse
    #[inline]
    pub fn reset(&mut self) {
        self.current_depth = 0;
        self.visited_nodes.clear();
        self.validation_path.clear();
        self.error_count = 0;
        self.warning_count = 0;
    }

    /// Set strict mode for validation
    #[inline]
    pub const fn set_strict_mode(&mut self, _strict: bool) {
        // Strict mode configuration would modify validation behavior
        // For now, this is a placeholder for future strict validation rules
    }

    /// Set YAML version for version-specific validation
    #[inline]
    pub const fn set_yaml_version(&mut self, _major: u32, _minor: u32) {
        // Version-specific validation rules would be configured here
        // For now, this is a placeholder for future version-specific validation
    }
}
