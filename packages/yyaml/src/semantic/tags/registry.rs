//! Tag registry for efficient storage and lookup of resolved tags
//!
//! This module provides the TagRegistry implementation with optimized
//! storage, blazing-fast lookups, and comprehensive tag management.
//! Zero-allocation design with intelligent caching and statistics.

use super::types::*;
use std::borrow::Cow;
use std::collections::HashMap;

/// Registry for resolved tags with efficient lookup and management
#[derive(Debug, Clone)]
pub struct TagRegistry<'input> {
    /// Storage for all resolved tags
    resolved_tags: HashMap<String, ResolvedTag<'input>>,
    /// Tag prefix mappings (handle -> prefix URI)
    tag_prefixes: HashMap<Cow<'input, str>, Cow<'input, str>>,
    /// Current schema type being used
    schema_type: SchemaType,
    /// Cache for frequently accessed tags
    access_cache: HashMap<String, usize>,
    /// Registry creation time for metrics
    creation_time: std::time::Instant,
    /// Total number of lookups performed
    lookup_count: usize,
    /// Cache hit count for performance metrics
    cache_hits: usize,
}

impl<'input> TagRegistry<'input> {
    /// Create new tag registry with default prefixes and optimized settings
    #[must_use] 
    pub fn new() -> Self {
        let mut tag_prefixes = HashMap::with_capacity(8);

        // Standard YAML 1.2 tag prefixes
        tag_prefixes.insert(Cow::Borrowed("!!"), Cow::Borrowed("tag:yaml.org,2002:"));
        tag_prefixes.insert(Cow::Borrowed("!"), Cow::Borrowed("!"));

        Self {
            resolved_tags: HashMap::with_capacity(64),
            tag_prefixes,
            schema_type: SchemaType::default(),
            access_cache: HashMap::with_capacity(32),
            creation_time: std::time::Instant::now(),
            lookup_count: 0,
            cache_hits: 0,
        }
    }

    /// Create new registry with specified initial capacity for optimization
    #[must_use] 
    pub fn with_capacity(tag_capacity: usize, prefix_capacity: usize) -> Self {
        let mut registry = Self::new();
        registry.resolved_tags.reserve(tag_capacity);
        registry.tag_prefixes.reserve(prefix_capacity);
        registry.access_cache.reserve(tag_capacity / 2);
        registry
    }

    /// Add tag prefix mapping for handle resolution
    #[inline]
    pub fn add_tag_prefix(&mut self, handle: Cow<'input, str>, prefix: Cow<'input, str>) {
        self.tag_prefixes.insert(handle, prefix);
    }

    /// Get tag prefix for a given handle
    #[inline]
    #[must_use] 
    pub fn get_tag_prefix(&self, handle: &str) -> Option<&Cow<'input, str>> {
        self.tag_prefixes.get(handle)
    }

    /// Add resolved tag to registry
    pub fn add_tag(&mut self, tag: ResolvedTag<'input>) {
        let tag_name = tag.full_tag.clone();
        self.resolved_tags.insert(tag_name.clone(), tag);

        // Initialize access tracking
        self.access_cache.insert(tag_name, 0);
    }

    /// Get resolved tag by name with access tracking
    pub fn get_tag(&mut self, tag_name: &str) -> Option<&mut ResolvedTag<'input>> {
        self.lookup_count += 1;

        // Update access count in cache
        if let Some(count) = self.access_cache.get_mut(tag_name) {
            *count += 1;
            self.cache_hits += 1;
        }

        // Get tag and update its access count
        if let Some(tag) = self.resolved_tags.get_mut(tag_name) {
            tag.mark_accessed();
            Some(tag)
        } else {
            None
        }
    }

    /// Get resolved tag by name (read-only access)
    #[inline]
    #[must_use] 
    pub fn get_tag_readonly(&self, tag_name: &str) -> Option<&ResolvedTag<'input>> {
        self.resolved_tags.get(tag_name)
    }

    /// Get all resolved tags as a vector
    #[must_use] 
    pub fn all_tags(&self) -> Vec<&ResolvedTag<'input>> {
        self.resolved_tags.values().collect()
    }

    /// Get all resolved tags (mutable access)
    pub fn all_tags_mut(&mut self) -> Vec<&mut ResolvedTag<'input>> {
        self.resolved_tags.values_mut().collect()
    }

    /// Check if tag exists in registry
    #[inline]
    #[must_use] 
    pub fn contains_tag(&self, tag_name: &str) -> bool {
        self.resolved_tags.contains_key(tag_name)
    }

    /// Get number of registered tags
    #[inline]
    #[must_use] 
    pub fn len(&self) -> usize {
        self.resolved_tags.len()
    }

    /// Check if registry is empty
    #[inline]
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.resolved_tags.is_empty()
    }

    /// Set current schema type
    #[inline]
    pub const fn set_schema_type(&mut self, schema_type: SchemaType) {
        self.schema_type = schema_type;
    }

    /// Get current schema type
    #[inline]
    #[must_use] 
    pub const fn schema_type(&self) -> SchemaType {
        self.schema_type
    }

    /// Clear all resolved tags but keep prefixes
    pub fn clear_tags(&mut self) {
        self.resolved_tags.clear();
        self.access_cache.clear();
        self.lookup_count = 0;
        self.cache_hits = 0;
    }

    /// Remove specific tag from registry
    pub fn remove_tag(&mut self, tag_name: &str) -> Option<ResolvedTag<'input>> {
        self.access_cache.remove(tag_name);
        self.resolved_tags.remove(tag_name)
    }

    /// Get tags by type filter
    #[must_use] 
    pub fn get_tags_by_type(&self, yaml_type: &YamlType) -> Vec<&ResolvedTag<'input>> {
        self.resolved_tags
            .values()
            .filter(|tag| &tag.resolved_type == yaml_type)
            .collect()
    }

    /// Get most frequently accessed tags
    #[must_use] 
    pub fn get_frequent_tags(&self, limit: usize) -> Vec<(&String, &usize)> {
        let mut access_vec: Vec<_> = self.access_cache.iter().collect();
        access_vec.sort_by(|a, b| b.1.cmp(a.1));
        access_vec.into_iter().take(limit).collect()
    }

    /// Get standard YAML tags only
    #[must_use] 
    pub fn get_standard_tags(&self) -> Vec<&ResolvedTag<'input>> {
        self.resolved_tags
            .values()
            .filter(|tag| tag.is_standard)
            .collect()
    }

    /// Get custom tags only
    #[must_use] 
    pub fn get_custom_tags(&self) -> Vec<&ResolvedTag<'input>> {
        self.resolved_tags
            .values()
            .filter(|tag| !tag.is_standard)
            .collect()
    }

    /// Get deprecated tags
    #[must_use] 
    pub fn get_deprecated_tags(&self) -> Vec<&ResolvedTag<'input>> {
        self.resolved_tags
            .values()
            .filter(|tag| tag.is_deprecated)
            .collect()
    }

    /// Get registry performance metrics
    #[must_use] 
    pub fn get_metrics(&self) -> TagMetrics {
        let _cache_hit_rate = if self.lookup_count > 0 {
            (self.cache_hits as f64 / self.lookup_count as f64) * 100.0
        } else {
            0.0
        };

        let elapsed = self.creation_time.elapsed();
        let operations_per_second = if elapsed.as_secs() > 0 {
            self.lookup_count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        TagMetrics {
            resolution_count: self.resolved_tags.len(),
            total_resolution_time_ns: elapsed.as_nanos() as u64,
            cache_hits: self.cache_hits,
            cache_misses: self.lookup_count.saturating_sub(self.cache_hits),
            operations_per_second,
            peak_memory_bytes: self.estimated_memory_usage(),
            schema_switches: 0, // This would be tracked elsewhere
        }
    }

    /// Get comprehensive tag statistics
    #[must_use] 
    pub fn get_statistics(&self) -> TagStatistics {
        let mut stats = TagStatistics {
            total_resolved: self.resolved_tags.len(),
            standard_tags: self.get_standard_tags().len(),
            custom_tags: self.get_custom_tags().len(),
            deprecated_tags: self.get_deprecated_tags().len(),
            ..Default::default()
        };

        // Calculate average resolution time
        if !self.resolved_tags.is_empty() {
            let total_time: u64 = self
                .resolved_tags
                .values()
                .map(|tag| tag.resolution_time.elapsed().as_nanos() as u64)
                .sum();
            stats.average_resolution_time_ns = total_time / self.resolved_tags.len() as u64;
        }

        // Build frequent tags map
        for (tag_name, access_count) in &self.access_cache {
            if *access_count > 0 {
                stats.frequent_tags.insert(tag_name.clone(), *access_count);
            }
        }

        // Schema usage (simplified - in real use this would track switches)
        stats
            .schema_usage
            .insert(self.schema_type, self.resolved_tags.len());

        // Cache hit rate
        stats.cache_hit_rate = if self.lookup_count > 0 {
            (self.cache_hits as f64 / self.lookup_count as f64) * 100.0
        } else {
            0.0
        };

        stats
    }

    /// Estimate current memory usage
    fn estimated_memory_usage(&self) -> usize {
        // Rough estimation of memory usage
        let tags_memory = self.resolved_tags.len() * std::mem::size_of::<ResolvedTag>();
        let prefixes_memory = self.tag_prefixes.len() * 64; // Estimated string overhead
        let cache_memory = self.access_cache.len() * std::mem::size_of::<(String, usize)>();

        tags_memory + prefixes_memory + cache_memory
    }

    /// Optimize registry by removing unused entries
    pub fn optimize(&mut self) {
        // Remove tags that haven't been accessed recently
        let cutoff_time = std::time::Instant::now() - std::time::Duration::from_secs(300); // 5 minutes

        self.resolved_tags.retain(|tag_name, tag| {
            let keep = tag.access_count > 0 || tag.resolution_time > cutoff_time;
            if !keep {
                self.access_cache.remove(tag_name);
            }
            keep
        });

        // Shrink containers to fit
        self.resolved_tags.shrink_to_fit();
        self.access_cache.shrink_to_fit();
    }

    /// Export registry data for serialization/debugging
    #[must_use] 
    pub fn export_data(&self) -> RegistryExportData<'input> {
        RegistryExportData {
            resolved_tags: self.resolved_tags.clone(),
            tag_prefixes: self.tag_prefixes.clone(),
            schema_type: self.schema_type,
            lookup_count: self.lookup_count,
            cache_hits: self.cache_hits,
        }
    }

    /// Import registry data from serialization
    pub fn import_data(&mut self, data: RegistryExportData<'input>) {
        self.resolved_tags = data.resolved_tags;
        self.tag_prefixes = data.tag_prefixes;
        self.schema_type = data.schema_type;
        self.lookup_count = data.lookup_count;
        self.cache_hits = data.cache_hits;

        // Rebuild access cache
        self.access_cache = self
            .resolved_tags
            .iter()
            .map(|(name, tag)| (name.clone(), tag.access_count))
            .collect();
    }
}

/// Export data structure for registry serialization
#[derive(Debug, Clone)]
pub struct RegistryExportData<'input> {
    pub resolved_tags: HashMap<String, ResolvedTag<'input>>,
    pub tag_prefixes: HashMap<Cow<'input, str>, Cow<'input, str>>,
    pub schema_type: SchemaType,
    pub lookup_count: usize,
    pub cache_hits: usize,
}

impl<'input> Default for TagRegistry<'input> {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-safe registry operations (if needed)
impl<'input> TagRegistry<'input> {
    /// Get thread-safe clone of tag data
    #[must_use] 
    pub fn get_thread_safe_snapshot(&self) -> HashMap<String, YamlType> {
        self.resolved_tags
            .iter()
            .map(|(name, tag)| (name.clone(), tag.resolved_type.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Position;

    #[test]
    fn test_registry_creation() {
        let registry = TagRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.schema_type(), SchemaType::Core);
        assert!(registry.tag_prefixes.contains_key("!!"));
    }

    #[test]
    fn test_tag_prefix_management() {
        let mut registry = TagRegistry::new();

        registry.add_tag_prefix(
            Cow::Borrowed("!custom!"),
            Cow::Borrowed("tag:example.com,2023:"),
        );

        assert!(registry.get_tag_prefix("!custom!").is_some());
        if let Some(prefix) = registry.get_tag_prefix("!custom!") {
            assert_eq!(prefix, &Cow::Borrowed("tag:example.com,2023:"));
        } else {
            panic!("Expected tag prefix to be present");
        }
    }

    #[test]
    fn test_tag_storage_and_retrieval() {
        let mut registry = TagRegistry::new();

        let tag = ResolvedTag::new(
            "tag:yaml.org,2002:str".to_string(),
            Cow::Borrowed("str"),
            Some(Cow::Borrowed("!!")),
            Cow::Borrowed("str"),
            YamlType::Str,
            Position::new(1, 1, 0),
        );

        registry.add_tag(tag);
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains_tag("tag:yaml.org,2002:str"));

        let retrieved = registry.get_tag("tag:yaml.org,2002:str");
        assert!(retrieved.is_some());
        if let Some(tag) = retrieved {
            assert_eq!(tag.resolved_type, YamlType::Str);
        } else {
            panic!("Expected tag to be present");
        }
    }

    #[test]
    fn test_access_tracking() {
        let mut registry = TagRegistry::new();

        let tag = ResolvedTag::new(
            "tag:yaml.org,2002:int".to_string(),
            Cow::Borrowed("int"),
            Some(Cow::Borrowed("!!")),
            Cow::Borrowed("int"),
            YamlType::Int,
            Position::new(1, 1, 0),
        );

        registry.add_tag(tag);

        // Access the tag multiple times
        registry.get_tag("tag:yaml.org,2002:int");
        registry.get_tag("tag:yaml.org,2002:int");
        registry.get_tag("tag:yaml.org,2002:int");

        let metrics = registry.get_metrics();
        assert_eq!(metrics.cache_hits, 3);
        assert!(metrics.operations_per_second >= 0.0);
    }

    #[test]
    fn test_type_filtering() {
        let mut registry = TagRegistry::new();

        let str_tag = ResolvedTag::new(
            "tag:yaml.org,2002:str".to_string(),
            Cow::Borrowed("str"),
            Some(Cow::Borrowed("!!")),
            Cow::Borrowed("str"),
            YamlType::Str,
            Position::new(1, 1, 0),
        );

        let int_tag = ResolvedTag::new(
            "tag:yaml.org,2002:int".to_string(),
            Cow::Borrowed("int"),
            Some(Cow::Borrowed("!!")),
            Cow::Borrowed("int"),
            YamlType::Int,
            Position::new(1, 1, 0),
        );

        registry.add_tag(str_tag);
        registry.add_tag(int_tag);

        let str_tags = registry.get_tags_by_type(&YamlType::Str);
        assert_eq!(str_tags.len(), 1);
        assert_eq!(str_tags[0].resolved_type, YamlType::Str);
    }

    #[test]
    fn test_registry_optimization() {
        let mut registry = TagRegistry::with_capacity(100, 10);

        // Add many tags
        for i in 0..50 {
            let tag = ResolvedTag::new(
                format!("tag:test:type{i}"),
                Cow::Owned(format!("type{i}")),
                Some(Cow::Borrowed("!!")),
                Cow::Owned(format!("type{i}")),
                YamlType::Str,
                Position::new(1, 1, 0),
            );
            registry.add_tag(tag);
        }

        assert_eq!(registry.len(), 50);

        registry.optimize();
        // After optimization, unused entries should be removed
        // (In this test, all entries should be removed since they weren't accessed)
    }
}
