//! Anchor registry and definition management
//!
//! Provides efficient storage and lookup for anchor definitions with
//! metadata tracking and validation support.

use crate::lexer::Position;
use crate::parser::ast::Node;
use crate::semantic::SemanticError;
use std::borrow::Cow;
use std::collections::HashMap;

/// Registry for anchor definitions with efficient lookup
#[derive(Debug, Clone)]
pub struct AnchorRegistry<'input> {
    anchors: HashMap<String, AnchorDefinition<'input>>,
    resolution_order: Vec<String>,
}

/// Anchor definition with complete metadata
#[derive(Debug, Clone)]
pub struct AnchorDefinition<'input> {
    pub name: Cow<'input, str>,
    pub node: Node<'input>,
    pub position: Position,
    pub definition_path: Vec<String>,
    pub first_seen: std::time::Instant,
    pub resolution_count: usize,
}

impl<'input> AnchorRegistry<'input> {
    /// Create new anchor registry
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            anchors: HashMap::with_capacity(16),
            resolution_order: Vec::with_capacity(16),
        }
    }

    /// Create anchor registry with specified capacity
    #[must_use] 
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            anchors: HashMap::with_capacity(capacity),
            resolution_order: Vec::with_capacity(capacity),
        }
    }

    /// Register a new anchor definition
    pub fn register(
        &mut self,
        name: String,
        definition: AnchorDefinition<'input>,
    ) -> Result<(), SemanticError> {
        // Check for duplicate names
        if self.anchors.contains_key(&name) {
            return Err(SemanticError::duplicate_anchor(
                name.clone(),
                self.anchors[&name].position,
                definition.position,
            ));
        }

        // Add to resolution order
        self.resolution_order.push(name.clone());

        // Register the definition
        self.anchors.insert(name, definition);

        Ok(())
    }

    /// Get anchor definition by name
    #[inline]
    #[must_use] 
    pub fn get_anchor(&self, name: &str) -> Option<&AnchorDefinition<'input>> {
        self.anchors.get(name)
    }

    /// Get mutable anchor definition by name
    #[inline]
    pub fn get_anchor_mut(&mut self, name: &str) -> Option<&mut AnchorDefinition<'input>> {
        self.anchors.get_mut(name)
    }

    /// Get all anchor names
    #[must_use] 
    pub fn anchor_names(&self) -> Vec<&str> {
        self.resolution_order.iter().map(|s| s.as_str()).collect()
    }

    /// Get anchors in resolution order
    #[must_use] 
    pub fn anchors_in_order(&self) -> Vec<&AnchorDefinition<'input>> {
        self.resolution_order
            .iter()
            .filter_map(|name| self.anchors.get(name))
            .collect()
    }

    /// Check if anchor exists
    #[inline]
    #[must_use] 
    pub fn contains_anchor(&self, name: &str) -> bool {
        self.anchors.contains_key(name)
    }

    /// Get anchor count
    #[inline]
    #[must_use] 
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Check if registry is empty
    #[inline]
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Remove anchor by name
    pub fn remove_anchor(&mut self, name: &str) -> Option<AnchorDefinition<'input>> {
        if let Some(definition) = self.anchors.remove(name) {
            // Remove from resolution order
            self.resolution_order.retain(|n| n != name);
            Some(definition)
        } else {
            None
        }
    }

    /// Clear all anchors
    pub fn clear(&mut self) {
        self.anchors.clear();
        self.resolution_order.clear();
    }

    /// Get anchors that match a predicate
    pub fn find_anchors<F>(&self, predicate: F) -> Vec<&AnchorDefinition<'input>>
    where
        F: Fn(&AnchorDefinition<'input>) -> bool,
    {
        self.anchors.values().filter(|def| predicate(def)).collect()
    }

    /// Get anchors by path prefix
    #[must_use] 
    pub fn anchors_by_path_prefix(&self, prefix: &str) -> Vec<&AnchorDefinition<'input>> {
        self.find_anchors(|def| def.path_string().starts_with(prefix))
    }

    /// Get recently defined anchors (within specified duration)
    #[must_use] 
    pub fn recent_anchors(&self, duration: std::time::Duration) -> Vec<&AnchorDefinition<'input>> {
        let threshold = std::time::Instant::now() - duration;
        self.find_anchors(|def| def.first_seen >= threshold)
    }

    /// Get frequently used anchors (resolution count above threshold)
    #[must_use] 
    pub fn frequently_used_anchors(&self, min_count: usize) -> Vec<&AnchorDefinition<'input>> {
        self.find_anchors(|def| def.resolution_count >= min_count)
    }

    /// Get unused anchors (never resolved)
    #[must_use] 
    pub fn unused_anchors(&self) -> Vec<&AnchorDefinition<'input>> {
        self.find_anchors(|def| def.resolution_count == 0)
    }

    /// Get registry statistics
    #[must_use] 
    pub fn statistics(&self) -> RegistryStatistics {
        let total_resolutions: usize = self.anchors.values().map(|def| def.resolution_count).sum();

        let unused_count = self.unused_anchors().len();

        let avg_resolutions = if !self.anchors.is_empty() {
            total_resolutions as f64 / self.anchors.len() as f64
        } else {
            0.0
        };

        RegistryStatistics {
            total_anchors: self.anchors.len(),
            total_resolutions,
            unused_anchors: unused_count,
            avg_resolutions_per_anchor: avg_resolutions,
        }
    }

    /// Validate all anchor definitions
    #[must_use] 
    pub fn validate(&self) -> Vec<RegistryValidationError> {
        let mut errors = Vec::new();

        // Check for potential naming conflicts (case variations)
        let mut name_variations: HashMap<String, Vec<String>> = HashMap::new();
        for name in self.anchors.keys() {
            let lower_name = name.to_lowercase();
            name_variations
                .entry(lower_name)
                .or_default()
                .push(name.clone());
        }

        for (lower_name, variations) in name_variations {
            if variations.len() > 1 {
                errors.push(RegistryValidationError::PotentialNamingConflict {
                    similar_names: variations,
                    case_insensitive_name: lower_name,
                });
            }
        }

        // Check for very long definition paths (potential inefficiency)
        for definition in self.anchors.values() {
            if definition.definition_path.len() > 10 {
                errors.push(RegistryValidationError::DeepNesting {
                    anchor_name: definition.name.to_string(),
                    depth: definition.definition_path.len(),
                    position: definition.position,
                });
            }
        }

        errors
    }
}

impl<'input> Default for AnchorRegistry<'input> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'input> AnchorDefinition<'input> {
    /// Create new anchor definition
    #[must_use] 
    pub fn new(
        name: Cow<'input, str>,
        node: Node<'input>,
        position: Position,
        definition_path: Vec<String>,
    ) -> Self {
        Self {
            name,
            node,
            position,
            definition_path,
            first_seen: std::time::Instant::now(),
            resolution_count: 0,
        }
    }

    /// Get anchor name as string
    #[inline]
    #[must_use] 
    pub fn name_str(&self) -> &str {
        &self.name
    }

    /// Get definition path as string
    #[must_use] 
    pub fn path_string(&self) -> String {
        self.definition_path.join(".")
    }

    /// Check if anchor has been resolved
    #[inline]
    #[must_use] 
    pub const fn is_resolved(&self) -> bool {
        self.resolution_count > 0
    }

    /// Get time since first seen
    #[inline]
    #[must_use] 
    pub fn age(&self) -> std::time::Duration {
        self.first_seen.elapsed()
    }

    /// Increment resolution count
    pub const fn increment_resolution_count(&mut self) {
        self.resolution_count += 1;
    }

    /// Get node type as string for debugging
    #[must_use] 
    pub const fn node_type(&self) -> &'static str {
        match &self.node {
            Node::Scalar(_) => "scalar",
            Node::Sequence(_) => "sequence",
            Node::Mapping(_) => "mapping",
            Node::Alias(_) => "alias",
            Node::Anchor(_) => "anchor",
            Node::Tagged(_) => "tagged",
            Node::Null(_) => "null",
        }
    }

    /// Check if definition contains cycles
    #[must_use] 
    pub fn contains_self_reference(&self) -> bool {
        self.contains_alias_to(&self.name)
    }

    /// Check if node contains reference to specific alias
    fn contains_alias_to(&self, alias_name: &str) -> bool {
        self.check_node_for_alias(&self.node, alias_name)
    }

    /// Recursively check node for alias reference
    #[allow(clippy::only_used_in_recursion)]
    fn check_node_for_alias(&self, node: &Node<'input>, alias_name: &str) -> bool {
        match node {
            Node::Alias(alias_node) => alias_node.name == alias_name,
            Node::Sequence(seq) => seq
                .items
                .iter()
                .any(|child| self.check_node_for_alias(child, alias_name)),
            Node::Mapping(map) => map.pairs.iter().any(|pair| {
                self.check_node_for_alias(&pair.key, alias_name)
                    || self.check_node_for_alias(&pair.value, alias_name)
            }),
            Node::Scalar(_) => false,
            Node::Anchor(anchor_node) => {
                // Check if the anchored content contains the alias
                self.check_node_for_alias(&anchor_node.node, alias_name)
            }
            Node::Tagged(tagged_node) => {
                // Check if the tagged content contains the alias
                self.check_node_for_alias(&tagged_node.node, alias_name)
            }
            Node::Null(_) => false,
        }
    }
}

/// Registry performance and usage statistics
#[derive(Debug, Clone, Copy)]
pub struct RegistryStatistics {
    pub total_anchors: usize,
    pub total_resolutions: usize,
    pub unused_anchors: usize,
    pub avg_resolutions_per_anchor: f64,
}

/// Registry validation errors
#[derive(Debug, Clone)]
pub enum RegistryValidationError {
    PotentialNamingConflict {
        similar_names: Vec<String>,
        case_insensitive_name: String,
    },
    DeepNesting {
        anchor_name: String,
        depth: usize,
        position: Position,
    },
}

impl RegistryValidationError {
    /// Get error message
    #[must_use] 
    pub fn message(&self) -> String {
        match self {
            Self::PotentialNamingConflict { similar_names, .. } => {
                format!(
                    "Potential naming conflict between anchors: {}",
                    similar_names.join(", ")
                )
            }
            Self::DeepNesting {
                anchor_name, depth, ..
            } => {
                format!("Anchor '{anchor_name}' has very deep nesting (depth: {depth})")
            }
        }
    }

    /// Get associated position if available
    #[must_use] 
    pub const fn position(&self) -> Option<Position> {
        match self {
            Self::PotentialNamingConflict { .. } => None,
            Self::DeepNesting { position, .. } => Some(*position),
        }
    }
}
