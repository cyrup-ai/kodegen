//! Additional domain types for agent role configuration
//!
//! This module provides additional types used in agent role configuration
//! that maintain zero-allocation patterns and performance characteristics.

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

/// Additional parameters for completion providers
///
/// Used to pass provider-specific configuration options like beta features,
/// custom parameters, and experimental settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleAdditionalParams {
    /// Key-value pairs for additional parameters
    pub params: HashMap<String, String>,
}

impl CandleAdditionalParams {
    /// Create a new empty additional parameters set
    #[must_use]
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    /// Create from key-value pairs
    #[must_use]
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
        Self {
            params: pairs.into_iter().collect(),
        }
    }

    /// Create from a JSON value map (converts values to strings)
    #[must_use]
    pub fn from_json_map(map: std::collections::HashMap<String, serde_json::Value>) -> Self {
        let params = map.into_iter().map(|(k, v)| (k, v.to_string())).collect();
        Self { params }
    }

    /// Add a parameter
    #[must_use]
    pub fn add_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Get a parameter value
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.params.get(key)
    }
}

impl Default for CandleAdditionalParams {
    fn default() -> Self {
        Self::new()
    }
}

impl From<HashMap<String, String>> for CandleAdditionalParams {
    fn from(params: HashMap<String, String>) -> Self {
        Self { params }
    }
}

impl<const N: usize> From<[(String, String); N]> for CandleAdditionalParams {
    fn from(array: [(String, String); N]) -> Self {
        Self {
            params: array.into_iter().collect(),
        }
    }
}

impl<const N: usize> From<[(&str, &str); N]> for CandleAdditionalParams {
    fn from(array: [(&str, &str); N]) -> Self {
        Self {
            params: array
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

/// Metadata for agent configuration
///
/// Used to store custom metadata, tags, and descriptive information
/// about agents and their configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleMetadata {
    /// Key-value pairs for metadata
    pub metadata: HashMap<String, String>,
}

impl CandleMetadata {
    /// Create a new empty metadata set
    #[must_use]
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
        }
    }

    /// Create from key-value pairs
    #[must_use]
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
        Self {
            metadata: pairs.into_iter().collect(),
        }
    }

    /// Create from a JSON value map (converts values to strings)
    #[must_use]
    pub fn from_json_map(map: std::collections::HashMap<String, serde_json::Value>) -> Self {
        let metadata = map.into_iter().map(|(k, v)| (k, v.to_string())).collect();
        Self { metadata }
    }

    /// Add metadata
    #[must_use]
    pub fn add_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get metadata value
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

impl Default for CandleMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl From<HashMap<String, String>> for CandleMetadata {
    fn from(metadata: HashMap<String, String>) -> Self {
        Self { metadata }
    }
}

impl<const N: usize> From<[(String, String); N]> for CandleMetadata {
    fn from(array: [(String, String); N]) -> Self {
        Self {
            metadata: array.into_iter().collect(),
        }
    }
}

impl<const N: usize> From<[(&str, &str); N]> for CandleMetadata {
    fn from(array: [(&str, &str); N]) -> Self {
        Self {
            metadata: array
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}
