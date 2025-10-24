use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::time::{Instant, SystemTime};
use tokio::sync::{RwLock, watch};

/// Data stored per file in `CountPerFile` mode
#[derive(Debug, Clone)]
pub struct FileCountData {
    pub count: usize,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
}

/// Search type enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    Files,
    Content,
}

/// Case matching mode for searches
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum CaseMode {
    /// Case-sensitive matching (default)
    #[default]
    Sensitive,
    /// Case-insensitive matching
    Insensitive,
    /// Smart case: insensitive if pattern is all lowercase, sensitive otherwise
    Smart,
}

/// Boundary mode for pattern matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryMode {
    /// Match only when pattern appears as complete line (^pattern$)
    /// Example: "error" matches "error" but not "this error happened"
    Line,
    /// Match only when pattern is surrounded by word boundaries (\bpattern\b)
    /// Example: "test" matches "`test()`" but not "testing"
    Word,
}

/// Regex engine choice for content search
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum EngineChoice {
    /// Automatically choose best engine (tries Rust, falls back to PCRE2)
    #[default]
    Auto,
    /// Use Rust regex engine only
    Rust,
    /// Use PCRE2 regex engine (supports backreferences, look-around)
    PCRE2,
}

/// Binary file handling mode (matches ripgrep's --binary and -a/--text flags)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum BinaryMode {
    /// Automatically skip binary files (default, no flag in rg)
    #[default]
    Auto,
    /// Search binary files but suppress binary content (rg --binary)
    Binary,
    /// Treat all files as text (rg -a/--text)
    Text,
}

/// Sort criterion for search results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    /// Sort alphabetically by file path (A-Z or Z-A)
    Path,
    /// Sort by last modified time (recent first or oldest first)
    Modified,
    /// Sort by last accessed time (if available on platform)
    Accessed,
    /// Sort by creation time (if available on platform)
    Created,
}

/// Sort direction for search results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    /// Ascending order: oldest first (time) or A-Z (path)
    Ascending,
    /// Descending order: newest first (time) or Z-A (path)
    Descending,
}

/// Search output mode - determines how results are formatted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchOutputMode {
    /// Full match details including file path, line number, and match content (default)
    #[default]
    Full,
    /// Only return unique file paths that contain matches (like rg -l)
    /// line field will be None, match field will be None
    FilesOnly,
    /// Return file paths with match counts (like rg -c)
    /// line field contains the count, match field will be None
    CountPerFile,
}

/// Search session options
#[derive(Debug, Clone)]
pub struct SearchSessionOptions {
    pub root_path: String,
    pub pattern: String,
    pub search_type: SearchType,
    pub file_pattern: Option<String>,
    /// File types to include (rg --type)
    pub r#type: Vec<String>,
    /// File types to exclude (rg --type-not)
    pub type_not: Vec<String>,
    /// Case matching mode (default: Sensitive)
    pub case_mode: CaseMode,
    pub max_results: Option<u32>,
    pub include_hidden: bool,
    /// Disable all ignore files - matches ripgrep's --no-ignore
    pub no_ignore: bool,
    pub context: u32,
    pub before_context: Option<u32>,
    pub after_context: Option<u32>,
    pub timeout_ms: Option<u64>,
    pub early_termination: Option<bool>,
    pub literal_search: bool,
    /// Boundary mode for pattern matching (default: None)
    /// - None: Match pattern anywhere (substring matching)
    /// - `Some(BoundaryMode::Word)`: Match whole words only (\bpattern\b)
    /// - `Some(BoundaryMode::Line)`: Match complete lines only (^pattern$)
    pub boundary_mode: Option<BoundaryMode>,
    /// Output mode - determines result format (default: Full)
    pub output_mode: SearchOutputMode,
    /// Invert match - show lines/files that DON'T match the pattern
    pub invert_match: bool,
    /// Regex engine choice (default: Auto)
    pub engine: EngineChoice,
    /// Preprocessor command to run on files before searching
    pub preprocessor: Option<String>,
    /// Glob patterns for files to run through preprocessor
    pub preprocessor_globs: Vec<String>,
    /// Enable searching inside compressed files (.gz, .zip, .bz2, .xz)
    pub search_zip: bool,
    /// Binary file handling mode (default: Auto)
    /// Matches ripgrep's --binary and -a/--text flags
    pub binary_mode: BinaryMode,
    /// Enable multiline pattern matching (rg --multiline)
    pub multiline: bool,
    /// Skip files larger than this size in bytes (None = unlimited)
    pub max_filesize: Option<u64>,
    /// Maximum directory depth to traverse (None = unlimited)
    /// Matches ripgrep's --max-depth flag
    /// 0 = root only, 1 = root + immediate children, etc.
    /// Essential for performance in monorepos with deep dependency trees
    pub max_depth: Option<usize>,
    /// Return only the matched portion of text, not the entire line
    pub only_matching: bool,
    /// List all files without searching (like rg --files)
    pub list_files_only: bool,
    /// Sort results by specified criterion (None = no sorting, filesystem order)
    pub sort_by: Option<SortBy>,
    /// Sort direction (default: Ascending if `sort_by` is specified)
    pub sort_direction: Option<SortDirection>,
    /// Text encoding (None = auto-detect)
    pub encoding: Option<String>,
}

/// Search result type
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchResultType {
    File,
    Content,
    FileList,
}

/// Single search result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    /// File path
    pub file: String,

    /// Line number (content search only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,

    /// Matching line content (content search only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<String>,

    /// Result type
    pub r#type: SearchResultType,

    /// True if this is a context line, false if actual match
    #[serde(default)]
    pub is_context: bool,

    /// Whether this result came from a binary file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_binary: Option<bool>,

    /// Whether binary content was suppressed in this result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_suppressed: Option<bool>,

    /// File modification time (if available and sorting is enabled)
    #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
    #[schemars(skip)]
    pub modified: Option<SystemTime>,

    /// File access time (if available and sorting is enabled)
    #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
    #[schemars(skip)]
    pub accessed: Option<SystemTime>,

    /// File creation time (if available and sorting is enabled)
    #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
    #[schemars(skip)]
    pub created: Option<SystemTime>,
}

/// Error that occurred during search
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchError {
    /// Path where error occurred
    pub path: String,

    /// Error message
    pub message: String,

    /// Error category for filtering
    #[serde(rename = "type")]
    pub error_type: String,
}

/// Active search session
pub struct SearchSession {
    pub id: String,
    pub cancellation_tx: watch::Sender<bool>,
    pub first_result_tx: watch::Sender<bool>,
    pub results: Arc<RwLock<Vec<SearchResult>>>,
    pub is_complete: Arc<AtomicBool>,
    pub is_error: Arc<RwLock<bool>>,
    pub error: Arc<RwLock<Option<String>>>,
    pub total_matches: Arc<AtomicUsize>,
    pub total_files: Arc<AtomicUsize>,
    pub last_read_time_atomic: Arc<AtomicU64>,
    pub start_time: Instant,
    pub was_incomplete: Arc<RwLock<bool>>,
    pub search_type: SearchType,
    pub pattern: String,
    /// Timeout in milliseconds (if specified)
    pub timeout_ms: Option<u64>,
    /// Count of errors encountered during search (lock-free atomic)
    pub error_count: Arc<AtomicUsize>,
    /// Detailed error list (limited to first 100 to prevent memory bloat)
    pub errors: Arc<RwLock<Vec<SearchError>>>,
    /// Effective maximum results for this search (after applying defaults/caps)
    pub max_results: usize,
    /// Output mode for this search
    pub output_mode: SearchOutputMode,
    /// Deduplication set for `FilesOnly` mode
    pub seen_files: Arc<RwLock<HashSet<String>>>,
    /// Count aggregation for `CountPerFile` mode
    pub file_counts: Arc<RwLock<HashMap<String, FileCountData>>>,
}

/// Response for `start_search`
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct StartSearchResponse {
    pub session_id: String,
    pub is_complete: bool,
    pub is_error: bool,
    pub results: Vec<SearchResult>,
    pub total_results: usize,
    pub runtime_ms: u64,
    /// Number of errors encountered during search
    #[serde(default)]
    pub error_count: usize,
    /// Effective maximum results for this search (after applying defaults/caps)
    pub max_results: usize,
    /// True if results were truncated due to hitting `max_results` limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results_limited: Option<bool>,
}

/// Response for `get_more_search_results`
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GetMoreSearchResultsResponse {
    pub session_id: String,
    pub results: Vec<SearchResult>,
    pub returned_count: usize,
    pub total_results: usize,
    pub total_matches: usize,
    pub is_complete: bool,
    pub is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub has_more_results: bool,
    pub runtime_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was_incomplete: Option<bool>,
    /// Number of errors encountered during search
    #[serde(default)]
    pub error_count: usize,
    /// Detailed error list (limited to first 100)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<SearchError>,
    /// True if results were truncated due to hitting `max_results` limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results_limited: Option<bool>,
}

/// Session information for `list_searches` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchSessionInfo {
    /// Unique session ID
    pub id: String,

    /// Search type: "files" or "content"
    pub search_type: String,

    /// Search pattern
    pub pattern: String,

    /// Whether search has completed
    pub is_complete: bool,

    /// Whether search encountered errors
    pub is_error: bool,

    /// Runtime in milliseconds
    pub runtime_ms: u64,

    /// Total results found so far
    pub total_results: usize,

    /// Timeout in milliseconds (if specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,

    /// Whether search was stopped due to timeout or cancellation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was_incomplete: Option<bool>,
}
