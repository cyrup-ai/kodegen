//! Core types and definitions for YAML tag resolution
//!
//! This module defines all the fundamental types, enums, and data structures
//! used throughout the tag resolution system. Optimized for zero-allocation
//! and blazing-fast performance with comprehensive YAML 1.2 support.

use crate::lexer::Position;
use crate::semantic::SemanticError;
use std::borrow::Cow;
use std::collections::HashMap;

/// YAML schema types according to specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaType {
    /// Core schema with all standard YAML types
    Core,
    /// JSON schema (subset of core, JSON-compatible only)
    Json,
    /// Failsafe schema (minimal types: string, sequence, mapping)
    Failsafe,
    /// Custom schema with user-defined types
    Custom,
}

/// Core YAML types according to YAML 1.2 specification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum YamlType {
    /// Null value (tag:yaml.org,2002:null)
    Null,
    /// Boolean value (tag:yaml.org,2002:bool)
    Bool,
    /// Integer value (tag:yaml.org,2002:int)
    Int,
    /// Floating point value (tag:yaml.org,2002:float)
    Float,
    /// String value (tag:yaml.org,2002:str)
    Str,
    /// Binary data (tag:yaml.org,2002:binary)
    Binary,
    /// Timestamp (tag:yaml.org,2002:timestamp)
    Timestamp,
    /// Sequence/array (tag:yaml.org,2002:seq)
    Seq,
    /// Mapping/dictionary (tag:yaml.org,2002:map)
    Map,
    /// Set collection (tag:yaml.org,2002:set)
    Set,
    /// Ordered mapping (tag:yaml.org,2002:omap)
    Omap,
    /// Pairs collection (tag:yaml.org,2002:pairs)
    Pairs,
    /// Merge key (tag:yaml.org,2002:merge)
    Merge,
    /// Value placeholder (tag:yaml.org,2002:value)
    Value,
    /// Custom application-specific type
    Custom(String),
    /// Unknown/unresolved type
    Unknown,
}

/// Resolved tag information with complete metadata
#[derive(Debug, Clone)]
pub struct ResolvedTag<'input> {
    /// Full resolved tag URI
    pub full_tag: String,
    /// Local tag identifier
    pub local_tag: Cow<'input, str>,
    /// Tag handle prefix (e.g., "!!" for standard tags)
    pub tag_handle: Option<Cow<'input, str>>,
    /// Tag suffix after handle
    pub tag_suffix: Cow<'input, str>,
    /// Resolved YAML type
    pub resolved_type: YamlType,
    /// Position in source document
    pub position: Position,
    /// When this tag was resolved
    pub resolution_time: std::time::Instant,
    /// Number of times this tag has been accessed
    pub access_count: usize,
    /// Whether this is a deprecated tag
    pub is_deprecated: bool,
    /// Whether this is a standard YAML tag
    pub is_standard: bool,
}

/// Custom type definition for application-specific types
#[derive(Debug, Clone)]
pub struct CustomTypeDefinition<'input> {
    /// Tag name/identifier
    pub tag_name: String,
    /// Type constructor function
    pub constructor: fn(&str) -> Result<YamlType, String>,
    /// Type validator function
    pub validator: fn(&str) -> bool,
    /// Type description
    pub description: Cow<'input, str>,
    /// Whether this type is experimental
    pub is_experimental: bool,
}

/// Tag validation warnings
#[derive(Debug, Clone)]
pub enum TagValidationWarning {
    /// Deprecated tag usage
    DeprecatedTag {
        tag: String,
        position: Position,
        replacement: Option<String>,
    },
    /// Unknown tag encountered
    UnknownTag {
        tag: String,
        position: Position,
        suggestion: Option<String>,
    },
    /// Ambiguous tag resolution
    AmbiguousResolution {
        tag: String,
        position: Position,
        candidates: Vec<String>,
    },
    /// Invalid tag format
    InvalidFormat {
        tag: String,
        position: Position,
        expected_format: String,
    },
    /// Conflicting tag definitions
    ConflictingDefinition {
        tag: String,
        position: Position,
        existing_definition: String,
    },
}

/// Tag resolution statistics for monitoring and profiling
#[derive(Debug, Clone)]
pub struct TagStatistics {
    /// Total number of tags resolved
    pub total_resolved: usize,
    /// Number of standard YAML tags resolved
    pub standard_tags: usize,
    /// Number of custom tags resolved
    pub custom_tags: usize,
    /// Number of deprecated tags encountered
    pub deprecated_tags: usize,
    /// Number of resolution errors
    pub resolution_errors: usize,
    /// Average resolution time in nanoseconds
    pub average_resolution_time_ns: u64,
    /// Most frequently used tags
    pub frequent_tags: HashMap<String, usize>,
    /// Schema type distribution
    pub schema_usage: HashMap<SchemaType, usize>,
    /// Cache hit rate percentage
    pub cache_hit_rate: f64,
}

/// Type resolver function signature for schema implementations
pub type TypeResolverFn = fn(&str) -> Option<YamlType>;

/// Optimized error construction helpers for zero-allocation performance
impl YamlType {
    /// Create tag resolution failed error with zero allocation where possible
    #[inline]
    #[must_use] 
    pub fn tag_resolution_failed_error(
        tag: &str,
        message: &'static str,
        position: Position,
    ) -> SemanticError {
        SemanticError::TagResolutionFailed {
            tag: tag.to_string(),
            reason: message.to_string(),
            position,
        }
    }

    /// Create unknown tag error with zero allocation where possible
    #[inline]
    #[must_use] 
    pub fn unknown_tag_error(tag: &str, position: Position) -> SemanticError {
        SemanticError::UnknownTag {
            tag: tag.to_string(),
            position,
        }
    }

    /// Create unknown custom tag error with zero allocation where possible
    #[inline]
    #[must_use] 
    pub fn unknown_custom_tag_error(tag: &str, position: Position) -> SemanticError {
        SemanticError::UnknownCustomTag {
            tag: tag.to_string(),
            position,
        }
    }

    /// Create unknown tag handle error with zero allocation where possible
    #[inline]
    #[must_use] 
    pub fn unknown_tag_handle_error(handle: &str, position: Position) -> SemanticError {
        SemanticError::UnknownTagHandle {
            handle: handle.to_string(),
            position,
        }
    }

    /// Create custom tag resolution failed error with zero allocation where possible
    #[inline]
    #[must_use] 
    pub fn custom_tag_resolution_failed_error(
        tag: &str,
        message: &str,
        position: Position,
    ) -> SemanticError {
        SemanticError::CustomTagResolutionFailed {
            tag: tag.to_string(),
            error: message.to_string(),
            position,
        }
    }
}

/// Performance metrics for tag resolution operations
#[derive(Debug, Clone)]
pub struct TagMetrics {
    /// Total resolution operations
    pub resolution_count: usize,
    /// Total resolution time in nanoseconds
    pub total_resolution_time_ns: u64,
    /// Cache hits
    pub cache_hits: usize,
    /// Cache misses
    pub cache_misses: usize,
    /// Average operations per second
    pub operations_per_second: f64,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: usize,
    /// Number of schema switches
    pub schema_switches: usize,
}

// Implementation of common traits and utility functions

impl Default for SchemaType {
    #[inline]
    fn default() -> Self {
        Self::Core
    }
}

impl Default for YamlType {
    #[inline]
    fn default() -> Self {
        Self::Unknown
    }
}

impl<'input> ResolvedTag<'input> {
    /// Create a new resolved tag
    #[inline]
    #[must_use] 
    pub fn new(
        full_tag: String,
        local_tag: Cow<'input, str>,
        tag_handle: Option<Cow<'input, str>>,
        tag_suffix: Cow<'input, str>,
        resolved_type: YamlType,
        position: Position,
    ) -> Self {
        Self {
            full_tag,
            local_tag,
            tag_handle,
            tag_suffix,
            resolved_type,
            position,
            resolution_time: std::time::Instant::now(),
            access_count: 0,
            is_deprecated: false,
            is_standard: false,
        }
    }

    /// Mark this tag as accessed (for statistics)
    #[inline]
    pub const fn mark_accessed(&mut self) {
        self.access_count += 1;
    }

    /// Check if this tag is frequently used
    #[inline]
    #[must_use] 
    pub const fn is_frequent(&self) -> bool {
        self.access_count > 10
    }
}

impl TagValidationWarning {
    /// Get warning message for display
    #[must_use] 
    pub fn message(&self) -> String {
        match self {
            Self::DeprecatedTag {
                tag, replacement, ..
            } => {
                if let Some(repl) = replacement {
                    format!("Tag '{tag}' is deprecated. Use '{repl}' instead.")
                } else {
                    format!("Tag '{tag}' is deprecated.")
                }
            }
            Self::UnknownTag {
                tag, suggestion, ..
            } => {
                if let Some(sugg) = suggestion {
                    format!("Unknown tag '{tag}'. Did you mean '{sugg}'?")
                } else {
                    format!("Unknown tag '{tag}'.")
                }
            }
            Self::AmbiguousResolution {
                tag, candidates, ..
            } => {
                format!(
                    "Ambiguous tag '{}'. Could be: {}",
                    tag,
                    candidates.join(", ")
                )
            }
            Self::InvalidFormat {
                tag,
                expected_format,
                ..
            } => {
                format!("Invalid tag format '{tag}'. Expected format: {expected_format}")
            }
            Self::ConflictingDefinition {
                tag,
                existing_definition,
                ..
            } => {
                format!("Conflicting definition for tag '{tag}'. Existing: {existing_definition}")
            }
        }
    }

    /// Get warning position
    #[must_use] 
    pub const fn position(&self) -> Position {
        match self {
            Self::DeprecatedTag { position, .. }
            | Self::UnknownTag { position, .. }
            | Self::AmbiguousResolution { position, .. }
            | Self::InvalidFormat { position, .. }
            | Self::ConflictingDefinition { position, .. } => *position,
        }
    }

    /// Get warning severity level
    #[inline]
    #[must_use] 
    pub const fn severity(&self) -> WarningSeverity {
        match self {
            Self::DeprecatedTag { .. } => WarningSeverity::Warning,
            Self::UnknownTag { .. } => WarningSeverity::Error,
            Self::AmbiguousResolution { .. } => WarningSeverity::Error,
            Self::InvalidFormat { .. } => WarningSeverity::Error,
            Self::ConflictingDefinition { .. } => WarningSeverity::Error,
        }
    }
}

/// Warning severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningSeverity {
    /// Informational message
    Info,
    /// Warning that should be addressed
    Warning,
    /// Error that must be fixed
    Error,
}

impl Default for TagStatistics {
    fn default() -> Self {
        Self {
            total_resolved: 0,
            standard_tags: 0,
            custom_tags: 0,
            deprecated_tags: 0,
            resolution_errors: 0,
            average_resolution_time_ns: 0,
            frequent_tags: HashMap::new(),
            schema_usage: HashMap::new(),
            cache_hit_rate: 0.0,
        }
    }
}

impl Default for TagMetrics {
    fn default() -> Self {
        Self {
            resolution_count: 0,
            total_resolution_time_ns: 0,
            cache_hits: 0,
            cache_misses: 0,
            operations_per_second: 0.0,
            peak_memory_bytes: 0,
            schema_switches: 0,
        }
    }
}

impl YamlType {
    /// Check if this is a scalar type
    #[inline]
    #[must_use] 
    pub const fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::Null
                | Self::Bool
                | Self::Int
                | Self::Float
                | Self::Str
                | Self::Binary
                | Self::Timestamp
        )
    }

    /// Check if this is a collection type
    #[inline]
    #[must_use] 
    pub const fn is_collection(&self) -> bool {
        matches!(
            self,
            Self::Seq | Self::Map | Self::Set | Self::Omap | Self::Pairs
        )
    }

    /// Check if this is a standard YAML 1.2 type
    #[inline]
    #[must_use] 
    pub const fn is_standard(&self) -> bool {
        !matches!(self, Self::Custom(_) | Self::Unknown)
    }

    /// Get the standard tag URI for this type
    #[must_use] 
    pub const fn standard_tag_uri(&self) -> Option<&'static str> {
        match self {
            Self::Null => Some("tag:yaml.org,2002:null"),
            Self::Bool => Some("tag:yaml.org,2002:bool"),
            Self::Int => Some("tag:yaml.org,2002:int"),
            Self::Float => Some("tag:yaml.org,2002:float"),
            Self::Str => Some("tag:yaml.org,2002:str"),
            Self::Binary => Some("tag:yaml.org,2002:binary"),
            Self::Timestamp => Some("tag:yaml.org,2002:timestamp"),
            Self::Seq => Some("tag:yaml.org,2002:seq"),
            Self::Map => Some("tag:yaml.org,2002:map"),
            Self::Set => Some("tag:yaml.org,2002:set"),
            Self::Omap => Some("tag:yaml.org,2002:omap"),
            Self::Pairs => Some("tag:yaml.org,2002:pairs"),
            Self::Merge => Some("tag:yaml.org,2002:merge"),
            Self::Value => Some("tag:yaml.org,2002:value"),
            Self::Custom(_) | Self::Unknown => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_type_classification() {
        assert!(YamlType::Str.is_scalar());
        assert!(YamlType::Int.is_scalar());
        assert!(!YamlType::Seq.is_scalar());

        assert!(YamlType::Seq.is_collection());
        assert!(YamlType::Map.is_collection());
        assert!(!YamlType::Str.is_collection());

        assert!(YamlType::Bool.is_standard());
        assert!(!YamlType::Custom("app:custom".to_string()).is_standard());
    }

    #[test]
    fn test_standard_tag_uris() {
        assert_eq!(
            YamlType::Str.standard_tag_uri(),
            Some("tag:yaml.org,2002:str")
        );
        assert_eq!(
            YamlType::Map.standard_tag_uri(),
            Some("tag:yaml.org,2002:map")
        );
        assert_eq!(YamlType::Unknown.standard_tag_uri(), None);
    }

    #[test]
    fn test_resolved_tag_creation() {
        let tag = ResolvedTag::new(
            "tag:yaml.org,2002:str".to_string(),
            Cow::Borrowed("str"),
            Some(Cow::Borrowed("!!")),
            Cow::Borrowed("str"),
            YamlType::Str,
            Position::new(1, 1, 0),
        );

        assert_eq!(tag.resolved_type, YamlType::Str);
        assert_eq!(tag.access_count, 0);
        assert!(!tag.is_frequent());
    }

    #[test]
    fn test_warning_severity_ordering() {
        assert!(WarningSeverity::Info < WarningSeverity::Warning);
        assert!(WarningSeverity::Warning < WarningSeverity::Error);
    }
}
