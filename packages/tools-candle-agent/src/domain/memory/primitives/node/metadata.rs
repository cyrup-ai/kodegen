use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Cache-padded metadata to prevent false sharing between CPU cores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNodeMetadata {
    /// Importance score (0.0 to 1.0)
    pub importance: f32,
    /// Keywords for search optimization
    pub keywords: Vec<Arc<str>>,
    /// Classification tags
    pub tags: Vec<Arc<str>>,
    /// Custom metadata with zero-copy keys
    pub custom: HashMap<Arc<str>, Arc<serde_json::Value>>,
    /// Version for optimistic concurrency control
    pub version: u64,
}

impl MemoryNodeMetadata {
    /// Create new metadata with default values
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            importance: 0.5,
            keywords: Vec::new(),
            tags: Vec::new(),
            custom: HashMap::new(),
            version: 1,
        }
    }

    /// Add keyword with zero-copy sharing
    #[inline]
    pub fn add_keyword(&mut self, keyword: impl Into<Arc<str>>) {
        self.keywords.push(keyword.into());
        self.version += 1;
    }

    /// Add tag with zero-copy sharing
    #[inline]
    pub fn add_tag(&mut self, tag: impl Into<Arc<str>>) {
        self.tags.push(tag.into());
        self.version += 1;
    }

    /// Set custom metadata with zero-copy key
    #[inline]
    pub fn set_custom(&mut self, key: impl Into<Arc<str>>, value: serde_json::Value) {
        self.custom.insert(key.into(), Arc::new(value));
        self.version += 1;
    }
}

impl Default for MemoryNodeMetadata {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
