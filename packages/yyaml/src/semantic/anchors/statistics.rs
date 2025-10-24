//! Anchor statistics, validation, and performance analysis
//!
//! Provides comprehensive statistics collection, validation warnings,
//! and performance analysis for anchor and alias resolution.

use super::AnchorRegistry;
use crate::lexer::Position;

/// Anchor resolution statistics
#[derive(Debug, Clone)]
pub struct AnchorStatistics {
    pub total_anchors: usize,
    pub total_aliases: usize,
    pub resolved_aliases: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub circular_references_detected: usize,
    pub max_resolution_depth: usize,
    pub avg_resolution_time_ms: f64,
    pub memory_usage_bytes: usize,
    pub unused_anchors: Vec<String>,
    pub frequently_resolved_anchors: Vec<(String, usize)>,
    pub validation_warnings: Vec<AnchorValidationWarning>,
}

impl Default for AnchorStatistics {
    fn default() -> Self {
        Self {
            total_anchors: 0,
            total_aliases: 0,
            resolved_aliases: 0,
            cache_hits: 0,
            cache_misses: 0,
            circular_references_detected: 0,
            max_resolution_depth: 0,
            avg_resolution_time_ms: 0.0,
            memory_usage_bytes: 0,
            unused_anchors: Vec::new(),
            frequently_resolved_anchors: Vec::new(),
            validation_warnings: Vec::new(),
        }
    }
}

impl AnchorStatistics {
    /// Create new statistics instance
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    /// Record successful alias resolution
    pub fn record_alias_resolution(&mut self, resolution_time_ms: f64) {
        self.resolved_aliases += 1;
        self.total_aliases += 1;

        // Update average resolution time
        if self.resolved_aliases == 1 {
            self.avg_resolution_time_ms = resolution_time_ms;
        } else {
            let total_time = self.avg_resolution_time_ms * (self.resolved_aliases - 1) as f64
                + resolution_time_ms;
            self.avg_resolution_time_ms = total_time / self.resolved_aliases as f64;
        }
    }

    /// Record cache hit
    #[inline]
    pub const fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Record cache miss
    #[inline]
    pub const fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    /// Record circular reference detection
    pub fn record_circular_reference(&mut self, _anchor_name: String, max_depth: usize) {
        self.circular_references_detected += 1;
        if max_depth > self.max_resolution_depth {
            self.max_resolution_depth = max_depth;
        }
    }

    /// Update memory usage estimate
    #[inline]
    pub const fn update_memory_usage(&mut self, bytes: usize) {
        self.memory_usage_bytes = bytes;
    }

    /// Get cache hit rate as percentage
    #[must_use] 
    pub fn cache_hit_rate(&self) -> f64 {
        let total_lookups = self.cache_hits + self.cache_misses;
        if total_lookups > 0 {
            (self.cache_hits as f64 / total_lookups as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get alias resolution rate as percentage
    #[must_use] 
    pub fn alias_resolution_rate(&self) -> f64 {
        if self.total_aliases > 0 {
            (self.resolved_aliases as f64 / self.total_aliases as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Check if performance is acceptable based on thresholds
    #[must_use] 
    pub fn is_performance_acceptable(
        &self,
        min_cache_hit_rate: f64,
        max_avg_resolution_time: f64,
    ) -> bool {
        self.cache_hit_rate() >= min_cache_hit_rate
            && self.avg_resolution_time_ms <= max_avg_resolution_time
    }

    /// Get performance summary
    #[must_use] 
    pub fn performance_summary(&self) -> PerformanceSummary {
        PerformanceSummary {
            cache_efficiency: self.cache_hit_rate(),
            resolution_efficiency: self.alias_resolution_rate(),
            avg_resolution_time: self.avg_resolution_time_ms,
            memory_efficiency: if self.total_anchors > 0 {
                self.memory_usage_bytes as f64 / self.total_anchors as f64
            } else {
                0.0
            },
            circular_reference_rate: if self.total_aliases > 0 {
                (self.circular_references_detected as f64 / self.total_aliases as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Generate performance report
    #[must_use] 
    pub fn generate_report(&self) -> String {
        let summary = self.performance_summary();

        format!(
            "Anchor Resolution Performance Report\n\
             =====================================\n\
             Total Anchors: {}\n\
             Total Aliases: {}\n\
             Resolved Aliases: {} ({:.1}%)\n\
             Cache Hit Rate: {:.1}%\n\
             Avg Resolution Time: {:.2}ms\n\
             Memory Usage: {} bytes\n\
             Circular References: {}\n\
             Max Resolution Depth: {}\n\
             Unused Anchors: {}\n\
             Validation Warnings: {}",
            self.total_anchors,
            self.total_aliases,
            self.resolved_aliases,
            summary.resolution_efficiency,
            summary.cache_efficiency,
            self.avg_resolution_time_ms,
            self.memory_usage_bytes,
            self.circular_references_detected,
            self.max_resolution_depth,
            self.unused_anchors.len(),
            self.validation_warnings.len()
        )
    }
}

/// Performance summary metrics
#[derive(Debug, Clone, Copy)]
pub struct PerformanceSummary {
    pub cache_efficiency: f64,
    pub resolution_efficiency: f64,
    pub avg_resolution_time: f64,
    pub memory_efficiency: f64,
    pub circular_reference_rate: f64,
}

impl PerformanceSummary {
    /// Get overall performance score (0-100)
    #[must_use] 
    pub fn overall_score(&self) -> f64 {
        let cache_score = self.cache_efficiency.min(100.0);
        let resolution_score = self.resolution_efficiency.min(100.0);
        let time_score = (1.0 / (1.0 + self.avg_resolution_time / 10.0)).min(1.0) * 100.0;
        let memory_score = (1.0 / (1.0 + self.memory_efficiency / 1024.0)).min(1.0) * 100.0;
        let circular_score = (100.0 - self.circular_reference_rate).max(0.0);

        (cache_score + resolution_score + time_score + memory_score + circular_score) / 5.0
    }
}

/// Anchor validation warnings
#[derive(Debug, Clone)]
pub enum AnchorValidationWarning {
    UnusedAnchor {
        name: String,
        position: Position,
        definition_age_ms: u64,
    },
    CircularReference {
        anchor_name: String,
        reference_path: String,
        position: Position,
    },
    DeepNesting {
        anchor_name: String,
        depth: usize,
        position: Position,
    },
    PotentialMemoryIssue {
        anchor_name: String,
        estimated_size_bytes: usize,
        position: Position,
    },
    NamingConflict {
        anchor_names: Vec<String>,
        similarity_score: f64,
    },
    PerformanceIssue {
        anchor_name: String,
        resolution_count: usize,
        avg_resolution_time_ms: f64,
        position: Position,
    },
}

impl AnchorValidationWarning {
    /// Get warning message
    #[must_use] 
    pub fn message(&self) -> String {
        match self {
            Self::UnusedAnchor {
                name,
                definition_age_ms,
                ..
            } => {
                format!("Anchor '{name}' is defined but never used (age: {definition_age_ms}ms)")
            }
            Self::CircularReference {
                anchor_name,
                reference_path,
                ..
            } => {
                format!("Circular reference detected for anchor '{anchor_name}': {reference_path}")
            }
            Self::DeepNesting {
                anchor_name, depth, ..
            } => {
                format!(
                    "Anchor '{anchor_name}' has deep nesting (depth: {depth}), may impact performance"
                )
            }
            Self::PotentialMemoryIssue {
                anchor_name,
                estimated_size_bytes,
                ..
            } => {
                format!(
                    "Anchor '{anchor_name}' may use significant memory ({estimated_size_bytes} bytes)"
                )
            }
            Self::NamingConflict {
                anchor_names,
                similarity_score,
            } => {
                format!(
                    "Potential naming conflict between anchors {} (similarity: {:.1}%)",
                    anchor_names.join(", "),
                    similarity_score * 100.0
                )
            }
            Self::PerformanceIssue {
                anchor_name,
                resolution_count,
                avg_resolution_time_ms,
                ..
            } => {
                format!(
                    "Performance issue with anchor '{anchor_name}': {resolution_count} resolutions, avg {avg_resolution_time_ms:.2}ms"
                )
            }
        }
    }

    /// Get warning position
    #[must_use] 
    pub fn position(&self) -> Position {
        match self {
            Self::UnusedAnchor { position, .. }
            | Self::CircularReference { position, .. }
            | Self::DeepNesting { position, .. }
            | Self::PotentialMemoryIssue { position, .. }
            | Self::PerformanceIssue { position, .. } => *position,
            Self::NamingConflict { .. } => Position::default(),
        }
    }

    /// Get warning severity level
    #[must_use] 
    pub fn severity(&self) -> WarningSeverity {
        match self {
            Self::CircularReference { .. } => WarningSeverity::High,
            Self::PotentialMemoryIssue {
                estimated_size_bytes,
                ..
            } => {
                if *estimated_size_bytes > 1024 * 1024 {
                    // 1MB
                    WarningSeverity::High
                } else if *estimated_size_bytes > 64 * 1024 {
                    // 64KB
                    WarningSeverity::Medium
                } else {
                    WarningSeverity::Low
                }
            }
            Self::PerformanceIssue {
                avg_resolution_time_ms,
                ..
            } => {
                if *avg_resolution_time_ms > 100.0 {
                    WarningSeverity::High
                } else if *avg_resolution_time_ms > 10.0 {
                    WarningSeverity::Medium
                } else {
                    WarningSeverity::Low
                }
            }
            Self::DeepNesting { depth, .. } => {
                if *depth > 20 {
                    WarningSeverity::High
                } else if *depth > 10 {
                    WarningSeverity::Medium
                } else {
                    WarningSeverity::Low
                }
            }
            Self::UnusedAnchor { .. }
            | Self::NamingConflict { .. } => WarningSeverity::Low,
        }
    }
}

/// Warning severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningSeverity {
    Low,
    Medium,
    High,
}

impl WarningSeverity {
    /// Get severity as string
    #[must_use] 
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
        }
    }
}

/// Anchor validator for comprehensive validation
pub struct AnchorValidator;

impl AnchorValidator {
    /// Validate anchor registry and generate warnings
    #[must_use] 
    pub fn validate_anchors<'input>(
        registry: &AnchorRegistry<'input>,
    ) -> Vec<AnchorValidationWarning> {
        let mut warnings = Vec::new();

        // Check for unused anchors
        warnings.extend(Self::check_unused_anchors(registry));

        // Check for naming conflicts
        warnings.extend(Self::check_naming_conflicts(registry));

        // Check for deep nesting
        warnings.extend(Self::check_deep_nesting(registry));

        // Check for potential memory issues
        warnings.extend(Self::check_memory_issues(registry));

        // Check for performance issues
        warnings.extend(Self::check_performance_issues(registry));

        warnings
    }

    /// Check for unused anchors
    fn check_unused_anchors<'input>(
        registry: &AnchorRegistry<'input>,
    ) -> Vec<AnchorValidationWarning> {
        registry
            .unused_anchors()
            .into_iter()
            .map(|def| AnchorValidationWarning::UnusedAnchor {
                name: def.name.to_string(),
                position: def.position,
                definition_age_ms: def.age().as_millis() as u64,
            })
            .collect()
    }

    /// Check for naming conflicts
    fn check_naming_conflicts<'input>(
        registry: &AnchorRegistry<'input>,
    ) -> Vec<AnchorValidationWarning> {
        let mut warnings = Vec::new();
        let anchor_names: Vec<_> = registry.anchor_names();

        for (i, name1) in anchor_names.iter().enumerate() {
            for name2 in anchor_names.iter().skip(i + 1) {
                let similarity = Self::calculate_similarity(name1, name2);
                if similarity > 0.8 {
                    // 80% similarity threshold
                    warnings.push(AnchorValidationWarning::NamingConflict {
                        anchor_names: vec![name1.to_string(), name2.to_string()],
                        similarity_score: similarity,
                    });
                }
            }
        }

        warnings
    }

    /// Check for deep nesting in anchor definitions
    fn check_deep_nesting<'input>(
        registry: &AnchorRegistry<'input>,
    ) -> Vec<AnchorValidationWarning> {
        registry
            .anchors_in_order()
            .into_iter()
            .filter_map(|def| {
                let depth = Self::calculate_node_depth(&def.node);
                if depth > 10 {
                    // Configurable threshold
                    Some(AnchorValidationWarning::DeepNesting {
                        anchor_name: def.name.to_string(),
                        depth,
                        position: def.position,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check for potential memory issues
    fn check_memory_issues<'input>(
        registry: &AnchorRegistry<'input>,
    ) -> Vec<AnchorValidationWarning> {
        registry
            .anchors_in_order()
            .into_iter()
            .filter_map(|def| {
                let estimated_size = Self::estimate_node_size(&def.node);
                if estimated_size > 64 * 1024 {
                    // 64KB threshold
                    Some(AnchorValidationWarning::PotentialMemoryIssue {
                        anchor_name: def.name.to_string(),
                        estimated_size_bytes: estimated_size,
                        position: def.position,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check for performance issues
    fn check_performance_issues<'input>(
        registry: &AnchorRegistry<'input>,
    ) -> Vec<AnchorValidationWarning> {
        registry
            .frequently_used_anchors(10) // 10+ resolutions
            .into_iter()
            .filter_map(|def| {
                // Simulate average resolution time based on complexity
                let complexity = Self::calculate_node_depth(&def.node);
                let estimated_time = complexity as f64 * 2.0; // 2ms per depth level

                if estimated_time > 20.0 {
                    // 20ms threshold
                    Some(AnchorValidationWarning::PerformanceIssue {
                        anchor_name: def.name.to_string(),
                        resolution_count: def.resolution_count,
                        avg_resolution_time_ms: estimated_time,
                        position: def.position,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Calculate string similarity (Jaro-Winkler algorithm approximation)
    fn calculate_similarity(s1: &str, s2: &str) -> f64 {
        if s1 == s2 {
            return 1.0;
        }

        let len1 = s1.len();
        let len2 = s2.len();

        if len1 == 0 || len2 == 0 {
            return 0.0;
        }

        // Simple Levenshtein distance approximation
        let max_len = len1.max(len2);
        let min_len = len1.min(len2);

        1.0 - (max_len - min_len) as f64 / max_len as f64
    }

    /// Calculate node depth recursively
    fn calculate_node_depth(node: &crate::parser::ast::Node) -> usize {
        match node {
            crate::parser::ast::Node::Scalar(_) => 1,
            crate::parser::ast::Node::Alias(_) => 1,
            crate::parser::ast::Node::Sequence(seq) => {
                1 + seq
                    .items
                    .iter()
                    .map(|child| Self::calculate_node_depth(child))
                    .max()
                    .unwrap_or(0)
            }
            crate::parser::ast::Node::Mapping(map) => {
                1 + map
                    .pairs
                    .iter()
                    .map(|pair| {
                        Self::calculate_node_depth(&pair.key)
                            .max(Self::calculate_node_depth(&pair.value))
                    })
                    .max()
                    .unwrap_or(0)
            }
            crate::parser::ast::Node::Anchor(anchor_node) => {
                // Add 1 for the anchor layer plus depth of wrapped node
                1 + Self::calculate_node_depth(&anchor_node.node)
            }
            crate::parser::ast::Node::Tagged(tagged_node) => {
                // Add 1 for the tag layer plus depth of wrapped node
                1 + Self::calculate_node_depth(&tagged_node.node)
            }
            crate::parser::ast::Node::Null(_) => 1,
        }
    }

    /// Estimate memory usage of a node
    fn estimate_node_size(node: &crate::parser::ast::Node) -> usize {
        match node {
            crate::parser::ast::Node::Scalar(scalar) => {
                std::mem::size_of_val(scalar) + scalar.value.len()
            }
            crate::parser::ast::Node::Alias(alias) => {
                std::mem::size_of_val(alias) + alias.name.len()
            }
            crate::parser::ast::Node::Sequence(seq) => {
                std::mem::size_of_val(seq)
                    + seq
                        .items
                        .iter()
                        .map(|child| Self::estimate_node_size(child))
                        .sum::<usize>()
            }
            crate::parser::ast::Node::Mapping(map) => {
                std::mem::size_of_val(map)
                    + map
                        .pairs
                        .iter()
                        .map(|pair| {
                            Self::estimate_node_size(&pair.key)
                                + Self::estimate_node_size(&pair.value)
                        })
                        .sum::<usize>()
            }
            crate::parser::ast::Node::Anchor(anchor_node) => {
                std::mem::size_of_val(anchor_node)
                    + anchor_node.name.len()
                    + Self::estimate_node_size(&anchor_node.node)
            }
            crate::parser::ast::Node::Tagged(tagged_node) => {
                std::mem::size_of_val(tagged_node)
                    + tagged_node.handle.as_ref().map(|h| h.len()).unwrap_or(0)
                    + tagged_node.suffix.len()
                    + Self::estimate_node_size(&tagged_node.node)
            }
            crate::parser::ast::Node::Null(null_node) => std::mem::size_of_val(null_node),
        }
    }
}
