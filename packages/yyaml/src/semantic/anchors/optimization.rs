//! Anchor processing performance optimizations
//!
//! Provides optimization utilities, memory usage estimation, and performance
//! tuning suggestions for anchor resolution and caching systems.

use super::registry::AnchorRegistry;
use super::statistics::AnchorStatistics;
use crate::parser::ast::Node;

/// Anchor resolution optimizations
pub struct AnchorOptimizations;

impl AnchorOptimizations {
    /// Calculate optimal cache size based on document characteristics
    #[must_use] 
    pub fn calculate_optimal_cache_size(
        anchor_count: usize,
        estimated_alias_count: usize,
    ) -> usize {
        // Base cache size on expected alias resolution patterns
        let base_size = anchor_count.min(64); // Don't cache more than 64 by default
        let alias_factor = (estimated_alias_count as f64 / anchor_count.max(1) as f64).min(4.0);

        let optimal_size = (base_size as f64 * alias_factor) as usize;

        // Ensure reasonable bounds
        optimal_size.clamp(16, 512)
    }

    /// Estimate memory usage for anchor registry
    #[must_use] 
    pub fn estimate_memory_usage(registry: &AnchorRegistry) -> MemoryUsageEstimate {
        let mut total_bytes = std::mem::size_of::<AnchorRegistry>();
        let mut node_bytes = 0;
        let mut metadata_bytes = 0;

        for definition in registry.anchors_in_order() {
            // Estimate definition metadata size
            metadata_bytes += std::mem::size_of_val(definition);
            metadata_bytes += definition.name.len();
            metadata_bytes += definition
                .definition_path
                .iter()
                .map(|s| s.len())
                .sum::<usize>();

            // Estimate node size
            node_bytes += Self::estimate_node_memory(&definition.node);
        }

        total_bytes += node_bytes + metadata_bytes;

        MemoryUsageEstimate {
            total_bytes,
            node_bytes,
            metadata_bytes,
            registry_overhead: std::mem::size_of::<AnchorRegistry>(),
            estimated_cache_size: Self::estimate_cache_memory_usage(registry.len()),
        }
    }

    /// Suggest optimizations based on anchor usage patterns
    #[must_use] 
    pub fn suggest_optimizations(statistics: &AnchorStatistics) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Cache efficiency suggestions
        if statistics.cache_hit_rate() < 60.0 && statistics.total_aliases > 10 {
            suggestions.push(OptimizationSuggestion::IncreaseCacheSize);
        }

        // Memory usage suggestions
        if statistics.memory_usage_bytes > 1024 * 1024 {
            // 1MB
            suggestions.push(OptimizationSuggestion::OptimizeMemoryUsage);
        }

        // Resolution time suggestions
        if statistics.avg_resolution_time_ms > 10.0 {
            suggestions.push(OptimizationSuggestion::OptimizeResolutionAlgorithm);
        }

        // Circular reference suggestions
        if statistics.circular_references_detected > 0 {
            suggestions.push(OptimizationSuggestion::ImproveCircularReferenceDetection);
        }

        // Unused anchor suggestions
        if statistics.unused_anchors.len() > statistics.total_anchors / 4 {
            suggestions.push(OptimizationSuggestion::RemoveUnusedAnchors);
        }

        // Depth optimization suggestions
        if statistics.max_resolution_depth > 50 {
            suggestions.push(OptimizationSuggestion::ReduceNestingDepth);
        }

        suggestions
    }

    /// Estimate memory usage for a single node
    fn estimate_node_memory(node: &Node) -> usize {
        match node {
            Node::Scalar(scalar) => std::mem::size_of_val(scalar) + scalar.value.len(),
            Node::Alias(alias) => std::mem::size_of_val(alias) + alias.name.len(),
            Node::Sequence(seq) => {
                let base_size = std::mem::size_of_val(seq);
                let children_size: usize = seq
                    .items
                    .iter()
                    .map(|child| Self::estimate_node_memory(child))
                    .sum();
                base_size + children_size
            }
            Node::Mapping(map) => {
                let base_size = std::mem::size_of_val(map);
                let pairs_size: usize = map
                    .pairs
                    .iter()
                    .map(|pair| {
                        Self::estimate_node_memory(&pair.key)
                            + Self::estimate_node_memory(&pair.value)
                    })
                    .sum();
                base_size + pairs_size
            }
            Node::Anchor(anchor_node) => {
                std::mem::size_of_val(anchor_node)
                    + anchor_node.name.len()
                    + Self::estimate_node_memory(&anchor_node.node)
            }
            Node::Tagged(tagged_node) => {
                std::mem::size_of_val(tagged_node)
                    + tagged_node.handle.as_ref().map_or(0, |h| h.len())
                    + tagged_node.suffix.len()
                    + Self::estimate_node_memory(&tagged_node.node)
            }
            Node::Null(null_node) => std::mem::size_of_val(null_node),
        }
    }

    /// Estimate cache memory usage based on anchor count
    const fn estimate_cache_memory_usage(anchor_count: usize) -> usize {
        // Estimate based on typical cache entry size
        let avg_entry_size = 256; // Estimated average cached node size
        let cache_overhead = 64; // HashMap overhead per entry

        anchor_count * (avg_entry_size + cache_overhead)
    }

    /// Generate optimization report
    #[must_use] 
    pub fn generate_optimization_report(
        registry: &AnchorRegistry,
        statistics: &AnchorStatistics,
    ) -> OptimizationReport {
        let memory_estimate = Self::estimate_memory_usage(registry);
        let suggestions = Self::suggest_optimizations(statistics);
        let optimal_cache_size =
            Self::calculate_optimal_cache_size(statistics.total_anchors, statistics.total_aliases);

        OptimizationReport {
            memory_estimate,
            suggestions,
            optimal_cache_size,
            performance_score: statistics.performance_summary().overall_score(),
            efficiency_metrics: EfficiencyMetrics::calculate(statistics),
        }
    }

    /// Analyze anchor complexity and suggest simplifications
    #[must_use] 
    pub fn analyze_complexity(registry: &AnchorRegistry) -> ComplexityAnalysis {
        let mut max_depth = 0;
        let mut total_depth = 0;
        let mut complex_anchors = Vec::new();

        for definition in registry.anchors_in_order() {
            let depth = Self::calculate_node_complexity(&definition.node);
            total_depth += depth;

            if depth > max_depth {
                max_depth = depth;
            }

            if depth > 10 {
                // Complexity threshold
                complex_anchors.push(ComplexAnchor {
                    name: definition.name.to_string(),
                    complexity_score: depth,
                    position: definition.position,
                });
            }
        }

        let avg_depth = if !registry.is_empty() {
            total_depth as f64 / registry.len() as f64
        } else {
            0.0
        };

        ComplexityAnalysis {
            max_complexity: max_depth,
            avg_complexity: avg_depth,
            complex_anchors,
            total_anchors: registry.len(),
        }
    }

    /// Calculate node complexity score
    fn calculate_node_complexity(node: &Node) -> usize {
        match node {
            Node::Scalar(_) => 1,
            Node::Alias(_) => 2, // Aliases add resolution overhead
            Node::Sequence(seq) => {
                1 + seq
                    .items
                    .iter()
                    .map(|child| Self::calculate_node_complexity(child))
                    .sum::<usize>()
            }
            Node::Mapping(map) => {
                1 + map
                    .pairs
                    .iter()
                    .map(|pair| {
                        Self::calculate_node_complexity(&pair.key)
                            + Self::calculate_node_complexity(&pair.value)
                    })
                    .sum::<usize>()
            }
            Node::Anchor(anchor_node) => {
                2 + Self::calculate_node_complexity(&anchor_node.node) // Anchor adds complexity
            }
            Node::Tagged(tagged_node) => {
                1 + Self::calculate_node_complexity(&tagged_node.node) // Tagged adds minimal complexity
            }
            Node::Null(_) => 1,
        }
    }

    /// Generate cache tuning recommendations
    #[must_use] 
    pub fn cache_tuning_recommendations(
        statistics: &AnchorStatistics,
    ) -> CacheTuningRecommendations {
        let hit_rate = statistics.cache_hit_rate();
        let total_lookups = statistics.cache_hits + statistics.cache_misses;

        let recommended_size = if hit_rate < 50.0 {
            // Low hit rate - increase cache size
            (statistics.total_anchors * 2).min(1024)
        } else if hit_rate > 90.0 {
            // Very high hit rate - could potentially reduce cache size
            (statistics.total_anchors / 2).max(16)
        } else {
            // Reasonable hit rate - keep current size or slight adjustment
            statistics.total_anchors.max(32)
        };

        let eviction_strategy = if total_lookups > 1000 && hit_rate < 70.0 {
            EvictionStrategy::LeastRecentlyUsed
        } else {
            EvictionStrategy::LeastFrequentlyUsed
        };

        CacheTuningRecommendations {
            recommended_cache_size: recommended_size,
            eviction_strategy,
            prefetch_suggestions: Self::generate_prefetch_suggestions(statistics),
            monitoring_recommendations: Self::generate_monitoring_recommendations(statistics),
        }
    }

    /// Generate prefetch suggestions
    fn generate_prefetch_suggestions(statistics: &AnchorStatistics) -> Vec<PrefetchSuggestion> {
        let mut suggestions = Vec::new();

        // Suggest prefetching frequently accessed anchors
        for (anchor_name, resolution_count) in &statistics.frequently_resolved_anchors {
            if *resolution_count > 5 {
                suggestions.push(PrefetchSuggestion::FrequentlyAccessed {
                    anchor_name: anchor_name.clone(),
                    access_count: *resolution_count,
                });
            }
        }

        suggestions
    }

    /// Generate monitoring recommendations
    fn generate_monitoring_recommendations(
        statistics: &AnchorStatistics,
    ) -> Vec<MonitoringRecommendation> {
        let mut recommendations = Vec::new();

        if statistics.avg_resolution_time_ms > 5.0 {
            recommendations.push(MonitoringRecommendation::TrackResolutionTime);
        }

        if statistics.memory_usage_bytes > 512 * 1024 {
            recommendations.push(MonitoringRecommendation::TrackMemoryUsage);
        }

        if statistics.circular_references_detected > 0 {
            recommendations.push(MonitoringRecommendation::TrackCircularReferences);
        }

        recommendations
    }
}

/// Memory usage estimation
#[derive(Debug, Clone)]
pub struct MemoryUsageEstimate {
    pub total_bytes: usize,
    pub node_bytes: usize,
    pub metadata_bytes: usize,
    pub registry_overhead: usize,
    pub estimated_cache_size: usize,
}

impl MemoryUsageEstimate {
    /// Get memory usage breakdown as percentages
    #[must_use] 
    pub fn breakdown_percentages(&self) -> MemoryBreakdown {
        let total = self.total_bytes as f64;

        MemoryBreakdown {
            nodes: (self.node_bytes as f64 / total) * 100.0,
            metadata: (self.metadata_bytes as f64 / total) * 100.0,
            registry_overhead: (self.registry_overhead as f64 / total) * 100.0,
            cache: (self.estimated_cache_size as f64 / total) * 100.0,
        }
    }

    /// Check if memory usage is within acceptable limits
    #[must_use] 
    pub const fn is_within_limits(&self, max_bytes: usize) -> bool {
        self.total_bytes <= max_bytes
    }
}

/// Memory usage breakdown by category
#[derive(Debug, Clone, Copy)]
pub struct MemoryBreakdown {
    pub nodes: f64,
    pub metadata: f64,
    pub registry_overhead: f64,
    pub cache: f64,
}

/// Optimization suggestions for anchor processing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizationSuggestion {
    IncreaseCacheSize,
    OptimizeMemoryUsage,
    OptimizeResolutionAlgorithm,
    ImproveCircularReferenceDetection,
    RemoveUnusedAnchors,
    ReduceNestingDepth,
}

impl OptimizationSuggestion {
    /// Get suggestion description
    #[must_use] 
    pub const fn description(&self) -> &'static str {
        match self {
            Self::IncreaseCacheSize => {
                "Consider increasing cache size to improve hit rate"
            }
            Self::OptimizeMemoryUsage => {
                "Memory usage is high, consider optimizing node storage"
            }
            Self::OptimizeResolutionAlgorithm => {
                "Resolution times are slow, consider algorithm improvements"
            }
            Self::ImproveCircularReferenceDetection => {
                "Circular references detected, improve detection efficiency"
            }
            Self::RemoveUnusedAnchors => {
                "Many anchors are unused, consider removing them"
            }
            Self::ReduceNestingDepth => {
                "Deep nesting detected, consider flattening structure"
            }
        }
    }

    /// Get priority level
    #[must_use] 
    pub const fn priority(&self) -> Priority {
        match self {
            Self::ImproveCircularReferenceDetection => Priority::High,
            Self::OptimizeResolutionAlgorithm => Priority::High,
            Self::OptimizeMemoryUsage => Priority::Medium,
            Self::IncreaseCacheSize => Priority::Medium,
            Self::ReduceNestingDepth => Priority::Medium,
            Self::RemoveUnusedAnchors => Priority::Low,
        }
    }
}

/// Priority levels for optimization suggestions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
}

/// Comprehensive optimization report
#[derive(Debug, Clone)]
pub struct OptimizationReport {
    pub memory_estimate: MemoryUsageEstimate,
    pub suggestions: Vec<OptimizationSuggestion>,
    pub optimal_cache_size: usize,
    pub performance_score: f64,
    pub efficiency_metrics: EfficiencyMetrics,
}

/// Efficiency metrics for anchor processing
#[derive(Debug, Clone, Copy)]
pub struct EfficiencyMetrics {
    pub memory_efficiency: f64,
    pub time_efficiency: f64,
    pub cache_efficiency: f64,
    pub overall_efficiency: f64,
}

impl EfficiencyMetrics {
    /// Calculate efficiency metrics from statistics
    #[must_use] 
    pub fn calculate(statistics: &AnchorStatistics) -> Self {
        let memory_efficiency = if statistics.memory_usage_bytes > 0 {
            (statistics.total_anchors as f64 * 1024.0) / statistics.memory_usage_bytes as f64
        } else {
            1.0
        };

        let time_efficiency = if statistics.avg_resolution_time_ms > 0.0 {
            1.0 / (1.0 + statistics.avg_resolution_time_ms / 10.0)
        } else {
            1.0
        };

        let cache_efficiency = statistics.cache_hit_rate() / 100.0;

        let overall_efficiency = (memory_efficiency + time_efficiency + cache_efficiency) / 3.0;

        Self {
            memory_efficiency,
            time_efficiency,
            cache_efficiency,
            overall_efficiency,
        }
    }
}

/// Complexity analysis results
#[derive(Debug, Clone)]
pub struct ComplexityAnalysis {
    pub max_complexity: usize,
    pub avg_complexity: f64,
    pub complex_anchors: Vec<ComplexAnchor>,
    pub total_anchors: usize,
}

/// Complex anchor information
#[derive(Debug, Clone)]
pub struct ComplexAnchor {
    pub name: String,
    pub complexity_score: usize,
    pub position: crate::lexer::Position,
}

/// Cache tuning recommendations
#[derive(Debug, Clone)]
pub struct CacheTuningRecommendations {
    pub recommended_cache_size: usize,
    pub eviction_strategy: EvictionStrategy,
    pub prefetch_suggestions: Vec<PrefetchSuggestion>,
    pub monitoring_recommendations: Vec<MonitoringRecommendation>,
}

/// Cache eviction strategies
#[derive(Debug, Clone, Copy)]
pub enum EvictionStrategy {
    LeastRecentlyUsed,
    LeastFrequentlyUsed,
    TimeBasedExpiration,
}

/// Prefetch suggestions
#[derive(Debug, Clone)]
pub enum PrefetchSuggestion {
    FrequentlyAccessed {
        anchor_name: String,
        access_count: usize,
    },
}

/// Monitoring recommendations
#[derive(Debug, Clone)]
pub enum MonitoringRecommendation {
    TrackResolutionTime,
    TrackMemoryUsage,
    TrackCircularReferences,
}
