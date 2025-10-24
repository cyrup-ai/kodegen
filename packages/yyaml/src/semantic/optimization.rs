//! Performance optimization utilities for semantic analysis
//!
//! Provides buffer size estimation, complexity analysis, and other performance
//! optimizations for zero-allocation semantic processing.

use crate::parser::ast::{Document, Node, Stream};

/// Semantic analysis optimization utilities
pub struct SemanticOptimizations;

impl SemanticOptimizations {
    /// Estimate optimal buffer sizes for semantic analysis
    #[must_use] 
    pub fn estimate_buffer_sizes(stream: &Stream) -> BufferSizeHints {
        let total_nodes = stream
            .documents
            .iter()
            .map(|doc| Self::estimate_node_count(doc))
            .sum();

        let estimated_anchors = total_nodes / 10; // Rough estimate: 1 anchor per 10 nodes
        let estimated_aliases = estimated_anchors / 2; // Rough estimate: 1 alias per 2 anchors
        let estimated_tags = total_nodes / 5; // Rough estimate: 1 tag per 5 nodes

        BufferSizeHints {
            estimated_nodes: total_nodes,
            estimated_anchors,
            estimated_aliases,
            estimated_tags,
        }
    }

    /// Estimate node count in document
    #[must_use] 
    pub fn estimate_node_count(document: &Document) -> usize {
        document
            .content
            .as_ref()
            .map_or(0, |root| Self::count_nodes_recursive(root))
    }

    /// Recursively count nodes in AST
    fn count_nodes_recursive(node: &Node) -> usize {
        let mut count = 1; // Count this node

        match node {
            Node::Sequence(seq) => {
                for child in &seq.items {
                    count += Self::count_nodes_recursive(child);
                }
            }
            Node::Mapping(map) => {
                for pair in &map.pairs {
                    count += Self::count_nodes_recursive(&pair.key);
                    count += Self::count_nodes_recursive(&pair.value);
                }
            }
            Node::Anchor(_anchor) => {
                count += Self::count_nodes_recursive(&_anchor.node);
            }
            Node::Tagged(_tagged) => {
                count += Self::count_nodes_recursive(&_tagged.node);
            }
            Node::Scalar(_) | Node::Alias(_) | Node::Null(_) => {
                // Scalars, aliases, and null are leaf nodes
            }
        }

        count
    }

    /// Check if document requires complex analysis
    #[must_use] 
    pub fn requires_complex_analysis(document: &Document) -> bool {
        document
            .content
            .as_ref()
            .is_some_and(|root| Self::has_complex_constructs(root))
    }

    /// Check for complex YAML constructs requiring careful analysis
    fn has_complex_constructs(node: &Node) -> bool {
        match node {
            Node::Alias(_) => true, // Aliases always require complex tracking
            Node::Sequence(seq) => {
                // Check if any sequence element has complex constructs
                seq.items
                    .iter()
                    .any(|child| Self::has_complex_constructs(child))
            }
            Node::Mapping(map) => {
                // Check if any mapping pair has complex constructs
                map.pairs.iter().any(|pair| {
                    Self::has_complex_constructs(&pair.key)
                        || Self::has_complex_constructs(&pair.value)
                })
            }
            Node::Anchor(_anchor) => {
                // Anchors always require complex tracking
                true
            }
            Node::Tagged(_tagged) => {
                // Tagged nodes always require complex tracking
                true
            }
            Node::Scalar(_scalar) => {
                // Complex scalars: those with tags or special content
                _scalar.tag.is_some() || Self::is_complex_scalar_content(&_scalar.value)
            }
            Node::Null(_) => false, // Null nodes are simple
        }
    }

    /// Check if scalar content requires complex processing
    fn is_complex_scalar_content(content: &str) -> bool {
        // Consider content complex if it contains:
        // - Multiple lines (potential for folding/literal blocks)
        // - Special characters that might need escaping
        // - References to anchors/aliases within the content
        content.contains('\n')
            || content.contains('&')
            || content.contains('*')
            || content.len() > 100 // Arbitrarily consider long strings as complex
    }

    /// Estimate memory requirements for semantic analysis
    #[must_use] 
    pub fn estimate_memory_requirements(stream: &Stream) -> MemoryEstimate {
        let hints = Self::estimate_buffer_sizes(stream);

        // Rough memory estimates based on typical sizes
        let anchor_memory = hints.estimated_anchors * 64; // ~64 bytes per anchor entry
        let alias_memory = hints.estimated_aliases * 48; // ~48 bytes per alias entry
        let tag_memory = hints.estimated_tags * 32; // ~32 bytes per tag entry
        let tracking_memory = hints.estimated_nodes * 16; // ~16 bytes per node tracking

        MemoryEstimate {
            anchor_storage: anchor_memory,
            alias_storage: alias_memory,
            tag_storage: tag_memory,
            tracking_overhead: tracking_memory,
            total_estimated: anchor_memory + alias_memory + tag_memory + tracking_memory,
        }
    }

    /// Determine if parallel processing would be beneficial
    #[must_use] 
    pub fn should_use_parallel_processing(stream: &Stream) -> bool {
        let total_nodes = stream
            .documents
            .iter()
            .map(|doc| Self::estimate_node_count(doc))
            .sum::<usize>();

        // Use parallel processing for large documents
        // But avoid it if there are many inter-document references
        total_nodes > 10000 && stream.documents.len() > 1
    }

    /// Get optimization level based on document complexity
    #[must_use] 
    pub fn get_optimization_level(document: &Document) -> OptimizationLevel {
        let node_count = Self::estimate_node_count(document);
        let has_complex = Self::requires_complex_analysis(document);

        match (node_count, has_complex) {
            (n, _) if n > 50000 => OptimizationLevel::Maximum,
            (n, true) if n > 5000 => OptimizationLevel::High,
            (n, false) if n > 5000 => OptimizationLevel::Medium,
            (n, true) if n > 1000 => OptimizationLevel::Medium,
            _ => OptimizationLevel::Basic,
        }
    }

    /// Pre-allocate collections with optimal capacity
    #[must_use] 
    pub fn pre_allocate_collections(hints: &BufferSizeHints) -> CollectionCapacities {
        // Add 25% buffer to avoid frequent reallocations
        let buffer_factor = 1.25;

        CollectionCapacities {
            anchor_map: ((hints.estimated_anchors as f64 * buffer_factor) as usize).max(8),
            alias_map: ((hints.estimated_aliases as f64 * buffer_factor) as usize).max(4),
            tag_map: ((hints.estimated_tags as f64 * buffer_factor) as usize).max(4),
            reference_tracking: ((hints.estimated_nodes as f64 * buffer_factor) as usize).max(16),
        }
    }
}

/// Buffer size optimization hints
#[derive(Debug, Clone, Copy)]
pub struct BufferSizeHints {
    pub estimated_nodes: usize,
    pub estimated_anchors: usize,
    pub estimated_aliases: usize,
    pub estimated_tags: usize,
}

impl Default for BufferSizeHints {
    fn default() -> Self {
        Self {
            estimated_nodes: 100,
            estimated_anchors: 10,
            estimated_aliases: 5,
            estimated_tags: 20,
        }
    }
}

/// Memory usage estimates for semantic analysis
#[derive(Debug, Clone, Copy)]
pub struct MemoryEstimate {
    pub anchor_storage: usize,
    pub alias_storage: usize,
    pub tag_storage: usize,
    pub tracking_overhead: usize,
    pub total_estimated: usize,
}

/// Optimization levels for different document complexities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    Basic,   // Simple documents, minimal optimization
    Medium,  // Moderate complexity, standard optimizations
    High,    // Complex documents, aggressive optimization
    Maximum, // Very large/complex documents, all optimizations
}

/// Pre-calculated collection capacities for optimal allocation
#[derive(Debug, Clone, Copy)]
pub struct CollectionCapacities {
    pub anchor_map: usize,
    pub alias_map: usize,
    pub tag_map: usize,
    pub reference_tracking: usize,
}

impl Default for CollectionCapacities {
    fn default() -> Self {
        Self {
            anchor_map: 16,
            alias_map: 8,
            tag_map: 8,
            reference_tracking: 32,
        }
    }
}

/// Configuration for performance-critical operations
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub optimization_level: OptimizationLevel,
    pub enable_parallel_processing: bool,
    pub memory_limit: Option<usize>,
    pub collection_capacities: CollectionCapacities,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            optimization_level: OptimizationLevel::Medium,
            enable_parallel_processing: false,
            memory_limit: None,
            collection_capacities: CollectionCapacities::default(),
        }
    }
}

impl PerformanceConfig {
    /// Create configuration optimized for speed
    #[must_use] 
    pub const fn for_speed() -> Self {
        Self {
            optimization_level: OptimizationLevel::Maximum,
            enable_parallel_processing: true,
            memory_limit: None,
            collection_capacities: CollectionCapacities {
                anchor_map: 64,
                alias_map: 32,
                tag_map: 32,
                reference_tracking: 128,
            },
        }
    }

    /// Create configuration optimized for memory usage
    #[must_use] 
    pub const fn for_memory() -> Self {
        Self {
            optimization_level: OptimizationLevel::Basic,
            enable_parallel_processing: false,
            memory_limit: Some(1024 * 1024), // 1MB limit
            collection_capacities: CollectionCapacities {
                anchor_map: 8,
                alias_map: 4,
                tag_map: 4,
                reference_tracking: 16,
            },
        }
    }

    /// Create balanced configuration for general use
    #[must_use] 
    pub fn balanced() -> Self {
        Self::default()
    }
}
