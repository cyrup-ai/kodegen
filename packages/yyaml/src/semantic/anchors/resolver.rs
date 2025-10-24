//! Main anchor resolution logic with cycle detection
//!
//! Provides the core AnchorResolver implementation with comprehensive
//! cycle detection, caching, and performance optimization.

use super::cache::{CacheStatistics, CachedResolution};
use super::registry::{AnchorDefinition, AnchorRegistry};
use crate::parser::ast::Node;
use crate::semantic::SemanticError;
use std::borrow::Cow;
use std::collections::HashMap;

/// High-performance anchor resolver with cycle detection
#[derive(Debug)]
pub struct AnchorResolver<'input> {
    pub(super) anchor_registry: AnchorRegistry<'input>,
    resolution_cache: HashMap<String, CachedResolution<'input>>,
    alias_resolution_count: usize,
    cycle_detection_stack: Vec<String>,
}

impl<'input> AnchorResolver<'input> {
    /// Create new anchor resolver with optimized configuration
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            anchor_registry: AnchorRegistry::new(),
            resolution_cache: HashMap::with_capacity(32),
            alias_resolution_count: 0,
            cycle_detection_stack: Vec::with_capacity(16),
        }
    }

    /// Create anchor resolver with custom configuration
    #[must_use] 
    pub fn with_config(config: &crate::semantic::SemanticConfig<'input>) -> Self {
        let initial_capacity = if config.cycle_detection_enabled {
            64
        } else {
            32
        };

        Self {
            anchor_registry: AnchorRegistry::new(),
            resolution_cache: HashMap::with_capacity(initial_capacity),
            alias_resolution_count: 0,
            cycle_detection_stack: Vec::with_capacity(if config.cycle_detection_enabled {
                32
            } else {
                8
            }),
        }
    }

    /// Register anchor definition with conflict detection
    pub fn register_anchor(
        &mut self,
        name: Cow<'input, str>,
        node: &Node<'input>,
        _anchor_id: usize,
        definition_path: Vec<String>,
    ) -> Result<(), SemanticError> {
        let position = node.position();

        // Check for duplicate anchor names
        if let Some(existing) = self.anchor_registry.get_anchor(&name) {
            return Err(SemanticError::duplicate_anchor(
                name.to_string(),
                existing.position,
                position,
            ));
        }

        // Create anchor definition
        let definition =
            AnchorDefinition::new(name.clone(), node.clone(), position, definition_path);

        // Register in the anchor registry
        self.anchor_registry
            .register(name.to_string(), definition)?;

        Ok(())
    }

    /// Resolve alias with comprehensive cycle detection
    pub fn resolve_alias(
        &mut self,
        alias_name: &str,
    ) -> Result<Option<Node<'input>>, SemanticError> {
        // Check cache first for performance
        if let Some(cached) = self.resolution_cache.get_mut(alias_name) {
            cached.access_count += 1;
            return Ok(Some(cached.resolved_node.clone()));
        }

        // Get anchor definition and clone needed data to avoid borrowing conflicts
        let (anchor_node, anchor_position) = match self.anchor_registry.get_anchor(alias_name) {
            Some(def) => (def.node.clone(), def.position),
            None => return Ok(None), // Alias not found, will be handled as error by caller
        };

        // Set up cycle detection
        if self.cycle_detection_stack.contains(&alias_name.to_string()) {
            let path = self.cycle_detection_stack.join(" -> ");
            return Err(SemanticError::circular_reference(
                alias_name.to_string(),
                format!("{path} -> {alias_name}"),
                anchor_position,
            ));
        }

        // Enter resolution context
        self.cycle_detection_stack.push(alias_name.to_string());
        self.alias_resolution_count += 1;

        // Recursively resolve the anchor node
        let resolved_node = self.resolve_node_recursive(&anchor_node)?;

        // Exit resolution context
        self.cycle_detection_stack.pop();

        // Cache the resolution for performance
        self.cache_resolution(alias_name.to_string(), &resolved_node);

        Ok(Some(resolved_node))
    }

    /// Recursively resolve nodes with alias expansion
    fn resolve_node_recursive(
        &mut self,
        node: &Node<'input>,
    ) -> Result<Node<'input>, SemanticError> {
        match node {
            Node::Alias(alias_node) => {
                // Resolve nested alias
                match self.resolve_alias(&alias_node.name)? {
                    Some(resolved) => Ok(resolved),
                    None => Err(SemanticError::unresolved_alias(
                        alias_node.name.to_string(),
                        alias_node.position,
                    )),
                }
            }
            Node::Sequence(seq) => {
                // Recursively resolve sequence elements
                let mut resolved_values = Vec::with_capacity(seq.items.len());
                for value in &seq.items {
                    resolved_values.push(self.resolve_node_recursive(value)?);
                }

                Ok(Node::Sequence(crate::parser::ast::SequenceNode::new(
                    resolved_values,
                    seq.style,
                    seq.position,
                )))
            }
            Node::Mapping(map) => {
                // Recursively resolve mapping pairs
                let mut resolved_pairs = Vec::with_capacity(map.pairs.len());
                for pair in &map.pairs {
                    let resolved_key = self.resolve_node_recursive(&pair.key)?;
                    let resolved_value = self.resolve_node_recursive(&pair.value)?;
                    resolved_pairs.push(crate::parser::ast::MappingPair::new(
                        resolved_key,
                        resolved_value,
                    ));
                }

                Ok(Node::Mapping(crate::parser::ast::MappingNode::new(
                    resolved_pairs,
                    map.style,
                    map.position,
                )))
            }
            Node::Scalar(_) => {
                // Scalars don't contain aliases, return clone
                Ok(node.clone())
            }
            Node::Anchor(anchor_node) => {
                // Recursively resolve the anchored content
                let resolved_inner = self.resolve_node_recursive(&anchor_node.node)?;
                Ok(Node::Anchor(crate::parser::ast::AnchorNode::new(
                    anchor_node.name.clone(),
                    Box::new(resolved_inner),
                    anchor_node.position,
                )))
            }
            Node::Tagged(tagged_node) => {
                // Recursively resolve tagged content
                let resolved_inner = self.resolve_node_recursive(&tagged_node.node)?;
                Ok(Node::Tagged(crate::parser::ast::TaggedNode::new(
                    tagged_node.handle.clone(),
                    tagged_node.suffix.clone(),
                    Box::new(resolved_inner),
                    tagged_node.position,
                )))
            }
            Node::Null(_) => {
                // Null nodes don't contain aliases, return clone
                Ok(node.clone())
            }
        }
    }

    /// Cache resolution result for performance
    fn cache_resolution(&mut self, alias_name: String, resolved_node: &Node<'input>) {
        let cached = CachedResolution {
            resolved_node: resolved_node.clone(),
            cached_at: std::time::Instant::now(),
            access_count: 1,
        };

        self.resolution_cache.insert(alias_name, cached);
    }

    /// Get anchor registry for external access
    #[inline]
    #[must_use] 
    pub const fn registry(&self) -> &AnchorRegistry<'input> {
        &self.anchor_registry
    }

    /// Get number of registered anchors
    #[inline]
    #[must_use] 
    pub fn anchor_count(&self) -> usize {
        self.anchor_registry.len()
    }

    /// Get number of resolved anchors
    #[inline]
    #[must_use] 
    pub fn resolved_count(&self) -> usize {
        self.anchor_registry.len() // All registered anchors are considered resolved
    }

    /// Get alias resolution count
    #[inline]
    #[must_use] 
    pub const fn alias_count(&self) -> usize {
        self.alias_resolution_count
    }

    /// Reset resolver for new analysis
    pub fn reset(&mut self) {
        self.anchor_registry = AnchorRegistry::new();
        self.resolution_cache.clear();
        self.alias_resolution_count = 0;
        self.cycle_detection_stack.clear();
    }

    /// Optimize resolution cache by removing least recently used entries
    pub fn optimize_cache(&mut self, max_cache_size: usize) {
        if self.resolution_cache.len() <= max_cache_size {
            return;
        }

        // Collect keys to remove to avoid borrowing conflicts
        let mut cache_entries: Vec<_> = self
            .resolution_cache
            .iter()
            .map(|(k, v)| (k.clone(), v.access_count, v.cached_at))
            .collect();

        // Sort by access count (ascending) then by cache time (ascending)
        cache_entries.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));

        // Remove least recently used entries
        let to_remove = cache_entries.len() - max_cache_size;
        for (alias_name, _, _) in cache_entries.into_iter().take(to_remove) {
            self.resolution_cache.remove(&alias_name);
        }
    }

    /// Resolve all aliases in dependency order
    pub fn resolve_all_aliases(&mut self) -> Result<Vec<(String, Node<'input>)>, SemanticError> {
        let mut resolved_aliases = Vec::with_capacity(self.anchor_registry.len());

        // Get all anchor names in dependency order (clone to avoid borrowing conflicts)
        let anchor_names: Vec<String> = self
            .anchor_registry
            .anchor_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        for anchor_name in anchor_names {
            if let Some(resolved_node) = self.resolve_alias(&anchor_name)? {
                resolved_aliases.push((anchor_name, resolved_node));
            }
        }

        Ok(resolved_aliases)
    }

    /// Check if node contains potential circular references
    #[must_use] 
    pub fn contains_potential_cycles(&self, node: &Node<'input>, anchor_name: &str) -> bool {
        match node {
            Node::Alias(alias_node) => {
                // Direct self-reference
                if alias_node.name == anchor_name {
                    return true;
                }

                // Check for indirect cycles through the alias chain
                if let Some(target_def) = self.anchor_registry.get_anchor(&alias_node.name) {
                    return self.contains_potential_cycles(&target_def.node, anchor_name);
                }

                false
            }
            Node::Sequence(seq) => seq
                .items
                .iter()
                .any(|child| self.contains_potential_cycles(child, anchor_name)),
            Node::Mapping(map) => map.pairs.iter().any(|pair| {
                self.contains_potential_cycles(&pair.key, anchor_name)
                    || self.contains_potential_cycles(&pair.value, anchor_name)
            }),
            Node::Scalar(_) => false,
            Node::Anchor(anchor_node) => {
                // Check if the anchored content contains cycles
                self.contains_potential_cycles(&anchor_node.node, anchor_name)
            }
            Node::Tagged(tagged_node) => {
                // Check if the tagged content contains cycles
                self.contains_potential_cycles(&tagged_node.node, anchor_name)
            }
            Node::Null(_) => false,
        }
    }

    /// Get cache statistics
    #[must_use] 
    pub fn cache_statistics(&self) -> CacheStatistics {
        let total_accesses: usize = self
            .resolution_cache
            .values()
            .map(|cached| cached.access_count)
            .sum();

        let avg_access_count = if !self.resolution_cache.is_empty() {
            total_accesses as f64 / self.resolution_cache.len() as f64
        } else {
            0.0
        };

        CacheStatistics {
            cache_size: self.resolution_cache.len(),
            total_accesses,
            avg_access_count,
            hit_rate: if self.alias_resolution_count > 0 {
                total_accesses as f64 / self.alias_resolution_count as f64
            } else {
                0.0
            },
            memory_usage_estimate: self.resolution_cache.len()
                * std::mem::size_of::<(String, super::cache::CachedResolution)>(),
        }
    }
}

impl<'input> Default for AnchorResolver<'input> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// Cache types are now in cache.rs module
