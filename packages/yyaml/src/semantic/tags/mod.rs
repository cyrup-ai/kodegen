//! Modular tag resolution system for YAML semantic analysis
//!
//! This module provides comprehensive tag resolution with complete YAML 1.2
//! schema support, custom tag handling, and blazing-fast type inference.
//! The implementation is split into focused submodules for better maintainability
//! and separation of concerns.
//!
//! ## Architecture
//!
//! - **types**: Core types, enums, and data structures
//! - **registry**: Tag storage and efficient lookup with caching
//! - **schema**: YAML schema processing (Core, JSON, Failsafe)
//! - **resolver**: Main tag resolution engine with validation
//!
//! ## Performance
//!
//! - Zero-allocation design with intelligent caching
//! - Blazing-fast lookups with optimized data structures
//! - Comprehensive metrics and profiling support
//!
//! ## Usage
//!
//! ```rust
//! use yyaml::semantic::tags::TagResolver;
//!
//! let mut resolver = TagResolver::new();
//! // Resolution logic here
//! ```

// Core modules
pub mod registry;
pub mod resolver;
pub mod schema;
pub mod types;

// Re-export all public items for clean API
pub use registry::{RegistryExportData, TagRegistry};
pub use resolver::{CustomTagResolver, ResolverStatistics, SimpleCustomResolver, TagResolver};
pub use schema::{CoreSchema, FailsafeSchema, JsonSchema, SchemaProcessor};
pub use types::*;

// Convenience type aliases
pub type DefaultTagResolver<'input> = TagResolver<'input>;
pub type FastTagRegistry<'input> = TagRegistry<'input>;

// Convenience functions for common operations

/// Create a new tag resolver with optimized settings for performance
#[inline]
#[must_use] 
pub fn create_fast_resolver<'input>() -> TagResolver<'input> {
    TagResolver::with_capacity(256, 16) // Optimized for typical YAML files
}

/// Create a new tag resolver with minimal memory footprint
#[inline]
#[must_use] 
pub fn create_minimal_resolver<'input>() -> TagResolver<'input> {
    TagResolver::with_capacity(32, 4) // Minimal for small files
}

/// Create a new tag resolver for large documents
#[inline]
#[must_use] 
pub fn create_large_resolver<'input>() -> TagResolver<'input> {
    TagResolver::with_capacity(1024, 32) // Optimized for large files
}

/// Create a tag registry with default YAML 1.2 prefixes
#[inline]
#[must_use] 
pub fn create_registry<'input>() -> TagRegistry<'input> {
    TagRegistry::new()
}

/// Create a schema processor with all standard schemas
#[inline]
#[must_use] 
pub fn create_schema_processor<'input>() -> SchemaProcessor<'input> {
    SchemaProcessor::new()
}

/// Quick tag type inference from scalar value
#[inline]
#[must_use] 
pub fn infer_scalar_type(value: &str) -> YamlType {
    let processor = SchemaProcessor::new();
    processor.infer_scalar_type(value)
}

/// Check if a tag is a standard YAML 1.2 tag
#[inline]
#[must_use] 
pub fn is_standard_yaml_tag(tag: &str) -> bool {
    tag.starts_with("tag:yaml.org,2002:")
}

/// Get the standard tag URI for a YAML type
#[inline]
#[must_use] 
pub const fn get_standard_tag_uri(yaml_type: &YamlType) -> Option<&'static str> {
    yaml_type.standard_tag_uri()
}

/// Create a simple custom resolver for application-specific tags
pub fn create_simple_custom_resolver(
    name: String,
    tag_prefix: String,
    resolver_fn: fn(&str) -> YamlType,
) -> SimpleCustomResolver {
    SimpleCustomResolver::new(name, tag_prefix, resolver_fn)
}

// Validation and utility functions

/// Validate tag format according to YAML specification
pub fn validate_tag_format(tag: &str) -> Result<(), String> {
    if tag.is_empty() {
        return Err("Tag cannot be empty".to_string());
    }

    if tag.starts_with("tag:") {
        // Full URI format
        if !tag.contains(',') {
            return Err("Tag URI must contain date component".to_string());
        }
        if !tag.contains(':') {
            return Err("Tag URI must contain namespace separator".to_string());
        }
    } else if tag.starts_with('!') {
        // Local tag format
        if tag.len() < 2 {
            return Err("Local tag must have content after '!'".to_string());
        }
    } else {
        return Err("Tag must start with 'tag:' (URI) or '!' (local)".to_string());
    }

    Ok(())
}

/// Get recommended tag for deprecated tags
#[must_use] 
pub fn get_recommended_tag(deprecated_tag: &str) -> Option<&'static str> {
    match deprecated_tag {
        "tag:yaml.org,2002:python/none" => Some("tag:yaml.org,2002:null"),
        "tag:yaml.org,2002:python/bool" => Some("tag:yaml.org,2002:bool"),
        "tag:yaml.org,2002:yaml/merge" => Some("tag:yaml.org,2002:merge"),
        "tag:yaml.org,2002:yaml/value" => Some("tag:yaml.org,2002:value"),
        _ => None,
    }
}

/// Check if a tag is deprecated
#[must_use] 
pub fn is_deprecated_tag(tag: &str) -> bool {
    get_recommended_tag(tag).is_some()
}

// Performance and debugging utilities

/// Create a performance-optimized resolver configuration
pub struct ResolverConfig {
    pub tag_cache_size: usize,
    pub registry_capacity: usize,
    pub custom_resolver_capacity: usize,
    pub enable_metrics: bool,
    pub enable_validation: bool,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            tag_cache_size: 256,
            registry_capacity: 256,
            custom_resolver_capacity: 16,
            enable_metrics: true,
            enable_validation: true,
        }
    }
}

impl ResolverConfig {
    /// Create configuration optimized for speed
    #[must_use] 
    pub const fn fast() -> Self {
        Self {
            tag_cache_size: 512,
            registry_capacity: 512,
            custom_resolver_capacity: 32,
            enable_metrics: false,
            enable_validation: false,
        }
    }

    /// Create configuration optimized for memory
    #[must_use] 
    pub const fn minimal() -> Self {
        Self {
            tag_cache_size: 32,
            registry_capacity: 32,
            custom_resolver_capacity: 4,
            enable_metrics: false,
            enable_validation: false,
        }
    }

    /// Create configuration with full debugging
    #[must_use] 
    pub const fn debug() -> Self {
        Self {
            tag_cache_size: 128,
            registry_capacity: 128,
            custom_resolver_capacity: 8,
            enable_metrics: true,
            enable_validation: true,
        }
    }

    /// Apply configuration to create resolver
    #[must_use] 
    pub fn create_resolver<'input>(&self) -> TagResolver<'input> {
        TagResolver::with_capacity(self.registry_capacity, self.custom_resolver_capacity)
    }
}

/// Tag resolution statistics aggregator
pub struct TagStatsAggregator {
    resolvers: Vec<ResolverStatistics>,
}

impl TagStatsAggregator {
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    pub fn add_resolver_stats(&mut self, stats: ResolverStatistics) {
        self.resolvers.push(stats);
    }

    #[must_use] 
    pub fn get_aggregate_stats(&self) -> AggregateTagStatistics {
        let mut aggregate = AggregateTagStatistics::default();

        for stats in &self.resolvers {
            aggregate.total_resolvers += 1;
            aggregate.total_resolutions += stats.resolution_count;
            aggregate.total_warnings += stats.validation_warnings;
            aggregate.total_custom_resolvers += stats.custom_resolvers;
            aggregate.total_uptime_seconds += stats.uptime_seconds;
        }

        if !self.resolvers.is_empty() {
            aggregate.average_resolutions = aggregate.total_resolutions / self.resolvers.len();
            aggregate.average_uptime = aggregate.total_uptime_seconds / self.resolvers.len() as f64;
        }

        aggregate
    }
}

/// Aggregate statistics across multiple resolvers
#[derive(Debug, Clone, Default)]
pub struct AggregateTagStatistics {
    pub total_resolvers: usize,
    pub total_resolutions: usize,
    pub total_warnings: usize,
    pub total_custom_resolvers: usize,
    pub total_uptime_seconds: f64,
    pub average_resolutions: usize,
    pub average_uptime: f64,
}

impl Default for TagStatsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

// Common tag patterns and utilities

/// Standard YAML 1.2 tag prefixes
pub const YAML_TAG_PREFIX: &str = "tag:yaml.org,2002:";
pub const LOCAL_TAG_PREFIX: &str = "!";
pub const SECONDARY_TAG_PREFIX: &str = "!!";

/// Standard YAML 1.2 tags
pub mod standard_tags {
    pub const NULL: &str = "tag:yaml.org,2002:null";
    pub const BOOL: &str = "tag:yaml.org,2002:bool";
    pub const INT: &str = "tag:yaml.org,2002:int";
    pub const FLOAT: &str = "tag:yaml.org,2002:float";
    pub const STR: &str = "tag:yaml.org,2002:str";
    pub const BINARY: &str = "tag:yaml.org,2002:binary";
    pub const TIMESTAMP: &str = "tag:yaml.org,2002:timestamp";
    pub const SEQ: &str = "tag:yaml.org,2002:seq";
    pub const MAP: &str = "tag:yaml.org,2002:map";
    pub const SET: &str = "tag:yaml.org,2002:set";
    pub const OMAP: &str = "tag:yaml.org,2002:omap";
    pub const PAIRS: &str = "tag:yaml.org,2002:pairs";
    pub const MERGE: &str = "tag:yaml.org,2002:merge";
    pub const VALUE: &str = "tag:yaml.org,2002:value";
}

/// Deprecated YAML 1.1 tags for migration
pub mod deprecated_tags {
    pub const PYTHON_NONE: &str = "tag:yaml.org,2002:python/none";
    pub const PYTHON_BOOL: &str = "tag:yaml.org,2002:python/bool";
    pub const YAML_MERGE: &str = "tag:yaml.org,2002:yaml/merge";
    pub const YAML_VALUE: &str = "tag:yaml.org,2002:yaml/value";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convenience_functions() {
        let resolver = create_fast_resolver();
        assert!(resolver.resolution_count() == 0);

        let registry = create_registry();
        assert!(registry.is_empty());

        let processor = create_schema_processor();
        assert!(
            processor
                .resolve_yaml_12_tag("tag:yaml.org,2002:str")
                .is_ok()
        );
    }

    #[test]
    fn test_scalar_inference() {
        assert_eq!(infer_scalar_type("null"), YamlType::Null);
        assert_eq!(infer_scalar_type("true"), YamlType::Bool);
        assert_eq!(infer_scalar_type("123"), YamlType::Int);
        assert_eq!(infer_scalar_type("12.34"), YamlType::Float);
        assert_eq!(infer_scalar_type("hello"), YamlType::Str);
    }

    #[test]
    fn test_tag_validation() {
        assert!(validate_tag_format("tag:yaml.org,2002:str").is_ok());
        assert!(validate_tag_format("!local").is_ok());
        assert!(validate_tag_format("").is_err());
        assert!(validate_tag_format("invalid").is_err());
    }

    #[test]
    fn test_standard_tag_detection() {
        assert!(is_standard_yaml_tag("tag:yaml.org,2002:str"));
        assert!(!is_standard_yaml_tag("!local"));
        assert!(!is_standard_yaml_tag("tag:example.com:custom"));
    }

    #[test]
    fn test_deprecated_tag_detection() {
        assert!(is_deprecated_tag("tag:yaml.org,2002:python/none"));
        assert!(!is_deprecated_tag("tag:yaml.org,2002:str"));

        assert_eq!(
            get_recommended_tag("tag:yaml.org,2002:python/none"),
            Some("tag:yaml.org,2002:null")
        );
    }

    #[test]
    fn test_resolver_configs() {
        let config = ResolverConfig::fast();
        let resolver = config.create_resolver();
        assert_eq!(resolver.resolution_count(), 0);

        let minimal_config = ResolverConfig::minimal();
        assert!(minimal_config.tag_cache_size < config.tag_cache_size);
    }

    #[test]
    fn test_stats_aggregator() {
        let mut aggregator = TagStatsAggregator::new();

        let stats1 = ResolverStatistics {
            registry_stats: TagStatistics::default(),
            performance_metrics: TagMetrics::default(),
            resolution_count: 100,
            validation_warnings: 5,
            custom_resolvers: 2,
            uptime_seconds: 10.0,
        };

        aggregator.add_resolver_stats(stats1);

        let aggregate = aggregator.get_aggregate_stats();
        assert_eq!(aggregate.total_resolvers, 1);
        assert_eq!(aggregate.total_resolutions, 100);
        assert_eq!(aggregate.average_resolutions, 100);
    }

    #[test]
    fn test_standard_tag_constants() {
        assert_eq!(standard_tags::STR, "tag:yaml.org,2002:str");
        assert_eq!(standard_tags::INT, "tag:yaml.org,2002:int");
        assert_eq!(
            deprecated_tags::PYTHON_NONE,
            "tag:yaml.org,2002:python/none"
        );
    }
}
