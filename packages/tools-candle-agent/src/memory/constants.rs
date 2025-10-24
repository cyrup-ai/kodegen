//! Constants used throughout the memory module to avoid allocations
//!
//! This module provides compile-time constants for commonly used strings
//! and values to ensure zero-allocation access patterns.

/// Task type for search operations
pub const SEARCH_TASK: &str = "search";

/// Task type for indexing operations
pub const INDEX_TASK: &str = "index";

/// Task type for retrieval operations
pub const RETRIEVAL_TASK: &str = "retrieval";

/// Default similarity threshold for vector search
pub const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.7;

/// Default search limit
pub const DEFAULT_SEARCH_LIMIT: usize = 10;

/// Memory type constants
pub const MEMORY_TYPE_SEMANTIC: &str = "semantic";
pub const MEMORY_TYPE_EPISODIC: &str = "episodic";
pub const MEMORY_TYPE_PROCEDURAL: &str = "procedural";
pub const MEMORY_TYPE_WORKING: &str = "working";

/// Relationship type constants
pub const RELATIONSHIP_TYPE_RELATED_TO: &str = "related_to";
pub const RELATIONSHIP_TYPE_DEPENDS_ON: &str = "depends_on";
pub const RELATIONSHIP_TYPE_FOLLOWS: &str = "follows";
pub const RELATIONSHIP_TYPE_CONTAINS: &str = "contains";

/// Error messages to avoid format! allocations
pub const ERROR_VECTOR_NOT_FOUND: &str = "Vector not found";
pub const ERROR_MEMORY_NOT_FOUND: &str = "Memory not found";
pub const ERROR_INVALID_VECTOR_ID: &str = "Invalid vector ID";
pub const ERROR_INVALID_MEMORY_ID: &str = "Invalid memory ID";
pub const ERROR_INVALID_TRANSITION_TYPE: &str = "Invalid transition type";
pub const ERROR_CLEAR_REQUIRES_MUTABLE: &str = "Clear operation requires mutable access";

/// Metadata keys
pub const METADATA_SCHEMA_VERSION: &str = "schema_version";
pub const METADATA_PREFIX: &str = "metadata_";

/// Empty string constant to avoid allocations
pub const EMPTY_STRING: &str = "";
