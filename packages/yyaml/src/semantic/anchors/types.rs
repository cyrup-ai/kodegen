//! Core types and traits for anchor/alias resolution
//!
//! This module provides the foundational types used throughout the anchor
//! resolution system with zero-allocation optimizations.

use crate::lexer::Position;
use crate::parser::ast::Node;
use crate::semantic::SemanticError;

/// Anchor resolution result with complete error context
pub type AnchorResult<T> = Result<T, SemanticError>;

/// Anchor resolution status for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStatus {
    /// Anchor not yet resolved
    Pending,
    /// Currently being resolved (cycle detection)
    InProgress,
    /// Successfully resolved
    Resolved,
    /// Resolution failed with error
    Failed,
}

/// Anchor resolution priority for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResolutionPriority {
    /// Low priority - resolve when convenient
    Low,
    /// Normal priority - standard resolution
    Normal,
    /// High priority - resolve immediately
    High,
    /// Critical priority - blocking resolution
    Critical,
}

/// Anchor scope for namespace management
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorScope {
    /// Document-level scope
    Document,
    /// Stream-level scope (across documents)
    Stream,
    /// Local scope (within a specific context)
    Local(String),
}

/// Resolution strategy for performance optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStrategy {
    /// Eager resolution - resolve immediately
    Eager,
    /// Lazy resolution - resolve on first access
    Lazy,
    /// Cached resolution - resolve once and cache
    Cached,
    /// Smart resolution - adaptive strategy
    Smart,
}

/// Anchor resolution trait for extensibility
pub trait AnchorResolution<'input> {
    /// Resolve anchor by name with context
    fn resolve_anchor(
        &mut self,
        name: &str,
        context: &ResolutionContext,
    ) -> AnchorResult<Node<'input>>;

    /// Check if anchor exists without resolving
    fn has_anchor(&self, name: &str) -> bool;

    /// Get resolution status for anchor
    fn resolution_status(&self, name: &str) -> ResolutionStatus;

    /// Clear resolution cache
    fn clear_cache(&mut self);
}

/// Anchor validation trait for integrity checks
pub trait AnchorValidation<'input> {
    /// Validate anchor definition
    fn validate_anchor(
        &self,
        name: &str,
        node: &Node<'input>,
        position: Position,
    ) -> AnchorResult<()>;

    /// Validate alias reference
    fn validate_alias(&self, name: &str, position: Position) -> AnchorResult<()>;

    /// Check for circular references
    fn check_cycles(&self) -> Vec<Vec<String>>;
}

// AnchorResolver is now in resolver.rs module

// AnchorRegistry and AnchorDefinition are now in registry.rs module

// CachedResolution is now in resolver.rs module

/// Alias resolution context for cycle detection
#[derive(Debug, Clone)]
pub struct ResolutionContext {
    pub current_depth: usize,
    pub max_depth: usize,
    pub resolution_path: Vec<String>,
    pub visited_anchors: Vec<String>,
}

/// Anchor resolution statistics
#[derive(Debug, Clone)]
pub struct AnchorStatistics {
    pub total_anchors: usize,
    pub total_resolutions: usize,
    pub cached_resolutions: usize,
    pub alias_resolution_count: usize,
    pub avg_resolution_count: f64,
    pub cache_hit_ratio: f64,
}

/// Anchor validation warnings
#[derive(Debug, Clone)]
pub enum AnchorValidationWarning {
    UnusedAnchor {
        anchor_name: String,
        position: Position,
    },
    DeeplyNestedAnchor {
        anchor_name: String,
        depth: usize,
        position: Position,
    },
    PotentialCircularStructure {
        anchor_name: String,
        position: Position,
    },
}

impl AnchorValidationWarning {
    /// Get warning message
    #[must_use] 
    pub fn message(&self) -> String {
        match self {
            Self::UnusedAnchor { anchor_name, .. } => {
                format!("unused anchor: '{anchor_name}'")
            }
            Self::DeeplyNestedAnchor {
                anchor_name, depth, ..
            } => {
                format!("deeply nested anchor '{anchor_name}' at depth {depth}")
            }
            Self::PotentialCircularStructure { anchor_name, .. } => {
                format!("potential circular structure in anchor '{anchor_name}'")
            }
        }
    }

    /// Get warning position
    #[must_use] 
    pub const fn position(&self) -> Position {
        match self {
            Self::UnusedAnchor { position, .. } => *position,
            Self::DeeplyNestedAnchor { position, .. } => *position,
            Self::PotentialCircularStructure { position, .. } => *position,
        }
    }
}

// Optimization-related types are now in optimization.rs module
