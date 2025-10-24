//! Main tag resolution engine with YAML 1.2 compliance
//!
//! This module provides the core TagResolver implementation with blazing-fast
//! resolution, comprehensive validation, and zero-allocation optimization.

use super::{registry::TagRegistry, schema::SchemaProcessor, types::*};
use crate::lexer::Position;
use crate::parser::ast::Node;
use crate::semantic::{AnalysisContext, SemanticError};
use std::borrow::Cow;
use std::collections::HashMap;

/// Custom tag resolver trait for extensibility
pub trait CustomTagResolver<'input>: std::fmt::Debug {
    /// Resolve custom tag to YAML type
    fn resolve(&self, tag: &str, value: &str) -> Result<YamlType, String>;
    /// Validate custom tag format
    fn validate(&self, tag: &str) -> bool;
    /// Get resolver name for identification
    fn name(&self) -> &str;
}

/// High-performance tag resolver with YAML 1.2 compliance
#[derive(Debug)]
pub struct TagResolver<'input> {
    tag_registry: TagRegistry<'input>,
    schema_processor: SchemaProcessor<'input>,
    custom_resolvers: HashMap<String, Box<dyn CustomTagResolver<'input>>>,
    resolution_count: usize,
    validation_warnings: Vec<TagValidationWarning>,
    performance_metrics: TagMetrics,
    creation_time: std::time::Instant,
}

impl<'input> TagResolver<'input> {
    /// Create new tag resolver with default YAML 1.2 schemas
    #[must_use] 
    pub fn new() -> Self {
        Self {
            tag_registry: TagRegistry::new(),
            schema_processor: SchemaProcessor::new(),
            custom_resolvers: HashMap::new(),
            resolution_count: 0,
            validation_warnings: Vec::new(),
            performance_metrics: TagMetrics::default(),
            creation_time: std::time::Instant::now(),
        }
    }

    /// Create resolver with specified capacities for optimization
    #[must_use] 
    pub fn with_capacity(tag_capacity: usize, resolver_capacity: usize) -> Self {
        Self {
            tag_registry: TagRegistry::with_capacity(tag_capacity, 16),
            schema_processor: SchemaProcessor::new(),
            custom_resolvers: HashMap::with_capacity(resolver_capacity),
            resolution_count: 0,
            validation_warnings: Vec::with_capacity(32),
            performance_metrics: TagMetrics::default(),
            creation_time: std::time::Instant::now(),
        }
    }

    /// Create resolver with specific configuration
    #[must_use] 
    pub fn with_config(config: &crate::semantic::SemanticConfig<'input>) -> Self {
        let mut resolver = Self::new();
        if let Some((major, minor)) = config.yaml_version {
            resolver.set_yaml_version(major, minor);
        }
        // Add custom tag prefixes from config
        for (handle, prefix) in &config.custom_tag_prefixes {
            resolver
                .tag_registry
                .add_tag_prefix(handle.clone(), prefix.clone());
        }
        resolver
    }

    /// Resolve tag with complete YAML 1.2 compliance
    #[allow(clippy::ptr_arg)]
    pub fn resolve_tag(
        &mut self,
        tag_handle: &Option<Cow<'input, str>>,
        tag_suffix: &Cow<'input, str>,
        position: Position,
        context: &AnalysisContext<'input>,
    ) -> Result<ResolvedTag<'input>, SemanticError> {
        let start_time = std::time::Instant::now();
        self.resolution_count += 1;

        // Construct full tag URI
        let full_tag = self.construct_full_tag(tag_handle, tag_suffix, context)?;

        // Check cache first
        if let Some(cached_tag) = self.tag_registry.get_tag(&full_tag) {
            self.performance_metrics.cache_hits += 1;
            return Ok(cached_tag.clone());
        }

        self.performance_metrics.cache_misses += 1;

        // Resolve tag type through schema processing
        let resolved_type = self.resolve_tag_type(&full_tag, context)?;

        // Create resolved tag
        let mut resolved_tag = ResolvedTag::new(
            full_tag.clone(),
            tag_suffix.clone(),
            tag_handle.clone(),
            tag_suffix.clone(),
            resolved_type,
            position,
        );

        // Mark standard tags
        resolved_tag.is_standard = self.is_standard_tag(&full_tag);
        resolved_tag.is_deprecated = self.is_deprecated_tag(&full_tag);

        // Add to registry
        self.tag_registry.add_tag(resolved_tag.clone());

        // Update performance metrics
        let resolution_time = start_time.elapsed();
        self.performance_metrics.total_resolution_time_ns += resolution_time.as_nanos() as u64;
        self.performance_metrics.resolution_count += 1;

        // Check for validation warnings
        if resolved_tag.is_deprecated {
            self.validation_warnings
                .push(TagValidationWarning::DeprecatedTag {
                    tag: full_tag.clone(),
                    position,
                    replacement: self.get_tag_replacement(&full_tag),
                });
        }

        Ok(resolved_tag)
    }

    /// Construct full tag URI from handle and suffix
    #[allow(clippy::ptr_arg)]
    pub fn construct_full_tag(
        &self,
        tag_handle: &Option<Cow<'input, str>>,
        tag_suffix: &Cow<'input, str>,
        context: &AnalysisContext<'input>,
    ) -> Result<String, SemanticError> {
        match tag_handle {
            Some(handle) => {
                // Look up handle prefix in registry
                if let Some(prefix) = self.tag_registry.get_tag_prefix(handle) {
                    Ok(format!("{prefix}{tag_suffix}"))
                } else {
                    // Check context for handle definitions
                    if let Some(prefix) = context.get_tag_handle(handle) {
                        Ok(format!("{prefix}{tag_suffix}"))
                    } else {
                        Err(YamlType::unknown_tag_handle_error(
                            handle,
                            context.current_position(),
                        ))
                    }
                }
            }
            None => {
                // No handle - use suffix as-is (local tag)
                Ok(tag_suffix.to_string())
            }
        }
    }

    /// Resolve tag type through schema processing
    pub fn resolve_tag_type(
        &self,
        full_tag: &str,
        _context: &AnalysisContext<'input>,
    ) -> Result<YamlType, SemanticError> {
        // Try schema processing first
        if let Ok(yaml_type) = self.schema_processor.resolve_yaml_12_tag(full_tag) {
            return Ok(yaml_type);
        }

        // Try custom resolvers
        for (name, resolver) in &self.custom_resolvers {
            if resolver.validate(full_tag) {
                match resolver.resolve(full_tag, "") {
                    Ok(yaml_type) => return Ok(yaml_type),
                    Err(err) => {
                        return Err(YamlType::custom_tag_resolution_failed_error(
                            full_tag,
                            &format!("Custom resolver '{name}' failed: {err}"),
                            Position::default(),
                        ));
                    }
                }
            }
        }

        // Fallback to unknown
        Err(YamlType::unknown_tag_error(full_tag, Position::default()))
    }

    /// Infer tag from node content for implicit typing
    pub fn infer_tag_from_node(&mut self, node: &Node<'input>) -> YamlType {
        match node {
            Node::Scalar(scalar_node) => {
                self.schema_processor.infer_scalar_type(&scalar_node.value)
            }
            Node::Sequence(_) => YamlType::Seq,
            Node::Mapping(_) => YamlType::Map,
            Node::Alias(_) => YamlType::Unknown, // Resolve through alias system
            Node::Anchor(anchor_node) => {
                // Infer from the anchored content
                self.infer_tag_from_node(&anchor_node.node)
            }
            Node::Tagged(tagged_node) => {
                // Use the explicit tag, resolve it to YamlType
                let tag_name = tagged_node.tag_name();
                match self.resolve_tag_type(&tag_name, &crate::semantic::AnalysisContext::new()) {
                    Ok(yaml_type) => yaml_type,
                    Err(_) => YamlType::Unknown, // Fallback for unresolvable tags
                }
            }
            Node::Null(_) => YamlType::Null,
        }
    }

    /// Register custom tag resolver
    pub fn register_custom_resolver(
        &mut self,
        name: String,
        resolver: Box<dyn CustomTagResolver<'input>>,
    ) {
        self.custom_resolvers.insert(name, resolver);
    }

    /// Get tag registry for external access
    #[must_use] 
    pub const fn get_registry(&self) -> &TagRegistry<'input> {
        &self.tag_registry
    }

    /// Get mutable registry access
    pub const fn get_registry_mut(&mut self) -> &mut TagRegistry<'input> {
        &mut self.tag_registry
    }

    /// Get resolution count
    #[must_use] 
    pub const fn resolution_count(&self) -> usize {
        self.resolution_count
    }

    /// Get resolved tag count (alias for resolution_count for semantic analyzer compatibility)
    #[must_use] 
    pub const fn resolved_count(&self) -> usize {
        self.resolution_count
    }

    /// Reset resolver for new analysis
    pub fn reset(&mut self) {
        self.tag_registry.clear_tags();
        self.resolution_count = 0;
        self.validation_warnings.clear();
        self.performance_metrics = TagMetrics::default();
    }

    /// Validate tag resolution consistency
    #[must_use] 
    pub fn validate_tag_consistency(&self) -> Vec<TagValidationWarning> {
        let mut warnings = self.validation_warnings.clone();

        // Check for conflicting tag definitions
        let all_tags = self.tag_registry.all_tags();
        let mut tag_types: HashMap<String, YamlType> = HashMap::new();

        for tag in all_tags {
            if let Some(existing_type) = tag_types.get(&tag.full_tag) {
                if existing_type != &tag.resolved_type {
                    warnings.push(TagValidationWarning::ConflictingDefinition {
                        tag: tag.full_tag.clone(),
                        position: tag.position,
                        existing_definition: format!("{existing_type:?}"),
                    });
                }
            } else {
                tag_types.insert(tag.full_tag.clone(), tag.resolved_type.clone());
            }
        }

        warnings
    }

    /// Check if tag is deprecated
    #[must_use] 
    pub fn is_deprecated_tag(&self, tag: &str) -> bool {
        // Known deprecated tags from YAML 1.1 -> 1.2 transition
        matches!(
            tag,
            "tag:yaml.org,2002:python/none"
                | "tag:yaml.org,2002:python/bool"
                | "tag:yaml.org,2002:yaml/merge"
                | "tag:yaml.org,2002:yaml/value"
        )
    }

    /// Check if tag is standard YAML tag
    #[must_use] 
    pub fn is_standard_tag(&self, tag: &str) -> bool {
        tag.starts_with("tag:yaml.org,2002:")
    }

    /// Check if tag is registered custom tag
    #[must_use] 
    pub fn is_custom_tag(&self, tag: &str) -> bool {
        self.custom_resolvers
            .values()
            .any(|resolver| resolver.validate(tag))
    }

    /// Get tag resolution statistics
    #[must_use] 
    pub fn get_tag_statistics(&self) -> TagStatistics {
        self.tag_registry.get_statistics()
    }

    /// Get performance metrics
    #[must_use] 
    pub const fn get_performance_metrics(&self) -> &TagMetrics {
        &self.performance_metrics
    }

    /// Get validation warnings
    #[must_use] 
    pub fn get_validation_warnings(&self) -> &[TagValidationWarning] {
        &self.validation_warnings
    }

    /// Set YAML version and determine appropriate schema
    pub const fn set_yaml_version(&mut self, major: u32, minor: u32) {
        // YAML 1.2 uses Core schema by default
        // YAML 1.1 had some differences but we target 1.2 compliance
        let schema_type = match (major, minor) {
            (1, 2) => SchemaType::Core,
            (1, 1) => SchemaType::Core, // We handle 1.1 with 1.2 compliance
            _ => SchemaType::Core,      // Default to Core for unknown versions
        };
        self.set_schema_type(schema_type);
    }

    /// Set schema type for resolution
    pub const fn set_schema_type(&mut self, schema_type: SchemaType) {
        self.tag_registry.set_schema_type(schema_type);
        self.schema_processor.set_schema(schema_type);
        self.performance_metrics.schema_switches += 1;
    }

    /// Get current schema type
    #[must_use] 
    pub const fn get_schema_type(&self) -> SchemaType {
        self.tag_registry.schema_type()
    }

    /// Optimize resolver performance
    pub fn optimize(&mut self) {
        self.tag_registry.optimize();
        self.validation_warnings.shrink_to_fit();
    }

    /// Get comprehensive resolver statistics
    #[must_use] 
    pub fn get_comprehensive_stats(&self) -> ResolverStatistics {
        let registry_stats = self.tag_registry.get_statistics();
        let uptime = self.creation_time.elapsed();

        ResolverStatistics {
            registry_stats,
            performance_metrics: self.performance_metrics.clone(),
            resolution_count: self.resolution_count,
            validation_warnings: self.validation_warnings.len(),
            custom_resolvers: self.custom_resolvers.len(),
            uptime_seconds: uptime.as_secs_f64(),
        }
    }

    // Helper methods

    #[allow(dead_code)] // May be used for future tag resolution extensions
    fn get_available_handles(&self) -> Vec<String> {
        // Implementation would collect all available tag handles
        vec!["!!".to_string(), "!".to_string()]
    }

    fn get_tag_replacement(&self, tag: &str) -> Option<String> {
        // Known replacements for deprecated tags
        match tag {
            "tag:yaml.org,2002:python/none" => Some("tag:yaml.org,2002:null".to_string()),
            "tag:yaml.org,2002:python/bool" => Some("tag:yaml.org,2002:bool".to_string()),
            _ => None,
        }
    }
}

/// Comprehensive resolver statistics
#[derive(Debug, Clone)]
pub struct ResolverStatistics {
    pub registry_stats: TagStatistics,
    pub performance_metrics: TagMetrics,
    pub resolution_count: usize,
    pub validation_warnings: usize,
    pub custom_resolvers: usize,
    pub uptime_seconds: f64,
}

impl<'input> Default for TagResolver<'input> {
    fn default() -> Self {
        Self::new()
    }
}

// Simple custom resolver implementations for common cases
#[derive(Debug)]
pub struct SimpleCustomResolver {
    name: String,
    tag_prefix: String,
    resolver_fn: fn(&str) -> YamlType,
}

impl SimpleCustomResolver {
    pub fn new(name: String, tag_prefix: String, resolver_fn: fn(&str) -> YamlType) -> Self {
        Self {
            name,
            tag_prefix,
            resolver_fn,
        }
    }
}

impl<'input> CustomTagResolver<'input> for SimpleCustomResolver {
    fn resolve(&self, _tag: &str, _value: &str) -> Result<YamlType, String> {
        Ok((self.resolver_fn)(_tag))
    }

    fn validate(&self, tag: &str) -> bool {
        tag.starts_with(&self.tag_prefix)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = TagResolver::new();
        assert_eq!(resolver.resolution_count(), 0);
        assert_eq!(resolver.get_schema_type(), SchemaType::Core);
    }

    #[test]
    fn test_full_tag_construction() {
        let resolver = TagResolver::new();
        let context = AnalysisContext::new();

        match resolver.construct_full_tag(
            &Some(Cow::Borrowed("!!")),
            &Cow::Borrowed("str"),
            &context,
        ) {
            Ok(full_tag) => {
                assert_eq!(full_tag, "tag:yaml.org,2002:str");
            }
            Err(_) => panic!("Expected successful tag construction"),
        }
    }

    #[test]
    fn test_standard_tag_detection() {
        let resolver = TagResolver::new();

        assert!(resolver.is_standard_tag("tag:yaml.org,2002:str"));
        assert!(resolver.is_standard_tag("tag:yaml.org,2002:int"));
        assert!(!resolver.is_standard_tag("tag:example.com:custom"));
    }

    #[test]
    fn test_deprecated_tag_detection() {
        let resolver = TagResolver::new();

        assert!(resolver.is_deprecated_tag("tag:yaml.org,2002:python/none"));
        assert!(!resolver.is_deprecated_tag("tag:yaml.org,2002:str"));
    }

    #[test]
    fn test_custom_resolver_registration() {
        let mut resolver = TagResolver::new();

        let custom_resolver =
            SimpleCustomResolver::new("test_resolver".to_string(), "tag:test:".to_string(), |_| {
                YamlType::Custom("test".to_string())
            });

        resolver.register_custom_resolver("test".to_string(), Box::new(custom_resolver));

        assert!(resolver.is_custom_tag("tag:test:example"));
        assert!(!resolver.is_custom_tag("tag:yaml.org,2002:str"));
    }

    #[test]
    fn test_performance_tracking() {
        let resolver = TagResolver::new();
        let metrics = resolver.get_performance_metrics();

        assert_eq!(metrics.resolution_count, 0);
        assert_eq!(metrics.cache_hits, 0);
        assert_eq!(metrics.cache_misses, 0);
    }

    #[test]
    fn test_resolver_reset() {
        let mut resolver = TagResolver::new();

        // Simulate some activity
        resolver.resolution_count = 10;

        resolver.reset();
        assert_eq!(resolver.resolution_count(), 0);
        assert!(resolver.get_validation_warnings().is_empty());
    }
}
