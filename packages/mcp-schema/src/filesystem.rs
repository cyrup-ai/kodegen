//! Filesystem tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// READ FILE
// ============================================================================

/// Arguments for `read_file` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileArgs {
    /// Path to the file to read (or URL if `is_url` is true)
    pub path: String,

    /// Line offset to start reading from (0-based)
    /// Positive: Start from line N (0-based indexing)
    /// Negative: Read last N lines from end (tail behavior)
    #[serde(default)]
    pub offset: i64,

    /// Maximum number of lines to read (None = use tool's default)
    /// Ignored when offset is negative
    #[serde(default)]
    pub length: Option<usize>,

    /// Whether the path is a URL (auto-detected if not specified)
    #[serde(default)]
    pub is_url: bool,
}

/// Prompt arguments for `read_file` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFilePromptArgs {
    /// Optional: specific file type to focus examples on
    #[serde(default)]
    pub file_type: Option<String>,
}

// ============================================================================
// READ MULTIPLE FILES
// ============================================================================

/// Arguments for `read_multiple_files` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadMultipleFilesArgs {
    /// List of file paths to read
    pub paths: Vec<String>,

    /// Line offset for all files (optional)
    /// Positive: Start from line N (0-based indexing)
    /// Negative: Read last N lines from end (tail behavior)
    #[serde(default)]
    pub offset: i64,

    /// Max lines to read per file (optional)
    /// Ignored when offset is negative
    #[serde(default)]
    pub length: Option<usize>,
}

/// Prompt arguments for `read_multiple_files` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadMultipleFilesPromptArgs {}

// ============================================================================
// WRITE FILE
// ============================================================================

/// Arguments for `write_file` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteFileArgs {
    /// Path to the file to write
    pub path: String,

    /// Content to write to the file
    pub content: String,

    /// Write mode: "rewrite" (default) or "append"
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "rewrite".to_string()
}

/// Prompt arguments for `write_file` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteFilePromptArgs {
    #[serde(default)]
    pub example_type: Option<String>,
}

// ============================================================================
// MOVE FILE
// ============================================================================

/// Arguments for `move_file` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MoveFileArgs {
    /// Source path (file or directory to move)
    pub source: String,

    /// Destination path (where to move it)
    pub destination: String,
}

/// Prompt arguments for `move_file` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MoveFilePromptArgs {}

// ============================================================================
// DELETE FILE
// ============================================================================

/// Arguments for `delete_file` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteFileArgs {
    /// Path to the file to delete
    pub path: String,
}

/// Prompt arguments for `delete_file` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteFilePromptArgs {}

// ============================================================================
// DELETE DIRECTORY
// ============================================================================

/// Arguments for `delete_directory` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteDirectoryArgs {
    /// Path to the directory to delete
    pub path: String,

    /// Confirm recursive deletion (must be true)
    #[serde(default)]
    pub recursive: bool,
}

/// Prompt arguments for `delete_directory` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteDirectoryPromptArgs {}

// ============================================================================
// LIST DIRECTORY
// ============================================================================

/// Arguments for `list_directory` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListDirectoryArgs {
    /// Path to the directory to list
    pub path: String,

    /// Include hidden files (starting with .)
    #[serde(default)]
    pub include_hidden: bool,
}

/// Prompt arguments for `list_directory` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListDirectoryPromptArgs {
    #[serde(default)]
    pub show_advanced: Option<bool>,
}

// ============================================================================
// CREATE DIRECTORY
// ============================================================================

/// Arguments for `create_directory` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateDirectoryArgs {
    /// Path to the directory to create
    pub path: String,
}

/// Prompt arguments for `create_directory` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateDirectoryPromptArgs {}

// ============================================================================
// GET FILE INFO
// ============================================================================

/// Arguments for `get_file_info` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetFileInfoArgs {
    /// Path to the file or directory
    pub path: String,
}

/// Prompt arguments for `get_file_info` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetFileInfoPromptArgs {}

// ============================================================================
// EDIT BLOCK
// ============================================================================

/// Arguments for `edit_block` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditBlockArgs {
    /// Path to the file to edit
    pub file_path: String,

    /// The exact string to search for and replace
    pub old_string: String,

    /// The replacement string
    pub new_string: String,

    /// Expected number of replacements (defaults to 1)
    #[serde(default = "default_expected_replacements")]
    pub expected_replacements: usize,
}

fn default_expected_replacements() -> usize {
    1
}

/// Prompt arguments for `edit_block` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditBlockPromptArgs {}

// ============================================================================
// SEARCH TYPES AND ENUMS
// ============================================================================

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

// ============================================================================
// START SEARCH
// ============================================================================

fn default_search_type() -> SearchType {
    SearchType::Files
}

/// Arguments for `start_search` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct StartSearchArgs {
    /// Root directory to search
    pub path: String,

    /// Pattern to search for
    pub pattern: String,

    /// Search type: "files" or "content"
    #[serde(default = "default_search_type")]
    pub search_type: SearchType,

    /// File pattern filter (e.g., "*.rs", "*.{ts,js}")
    #[serde(default)]
    pub file_pattern: Option<String>,

    /// File types to include using ripgrep's built-in definitions (rg --type)
    /// Examples: ["rust", "python", "javascript", "markdown"]
    /// Combines with `file_pattern` if both specified
    /// Can be specified multiple times: ["rust", "python"]
    #[serde(default, rename = "type")]
    pub r#type: Vec<String>,

    /// File types to exclude using ripgrep's built-in definitions (rg --type-not)
    /// Examples: ["test", "json", "minified"]
    /// Can be specified multiple times: ["test", "minified"]
    #[serde(default)]
    pub type_not: Vec<String>,

    /// Case matching mode: "sensitive", "insensitive", or "smart" (default: "sensitive")
    /// Smart case: case-insensitive if pattern is all lowercase, sensitive otherwise
    #[serde(default)]
    pub case_mode: CaseMode,

    /// DEPRECATED: Use `case_mode` instead. Provided for backward compatibility.
    /// If set, overrides `case_mode`: true → Insensitive, false → Sensitive
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_case: Option<bool>,

    /// Maximum number of results
    #[serde(default)]
    pub max_results: Option<u32>,

    /// Include hidden files
    #[serde(default)]
    pub include_hidden: bool,

    /// Disable all ignore files (.gitignore, .ignore, etc.)
    /// Matches ripgrep's --no-ignore flag
    #[serde(default)]
    pub no_ignore: bool,

    /// Number of context lines (rg -C / rg --context)
    /// Sets both before and after context to same value
    /// If `before_context` or `after_context` specified, they override this
    #[serde(default)]
    pub context: u32,

    /// Number of lines before each match (rg -B / rg --before-context)
    /// Overrides context if specified
    #[serde(default)]
    pub before_context: Option<u32>,

    /// Number of lines after each match (rg -A / rg --after-context)
    /// Overrides context if specified
    #[serde(default)]
    pub after_context: Option<u32>,

    /// Timeout in milliseconds
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Stop early when exact filename match found (files only)
    #[serde(default)]
    pub early_termination: Option<bool>,

    /// Force literal string matching instead of regex (default: false)
    #[serde(default)]
    pub literal_search: bool,

    /// Boundary mode for pattern matching: "word", "line", or null (default: null)
    /// - null/omitted: Match pattern anywhere (substring matching)
    /// - "word": Match whole words only - uses \b anchors
    ///   * Content search: "test" matches "`test()`" but not "testing"
    ///   * File search: "lib" matches "lib.rs" but not "libtest.rs"
    /// - "line": Match complete lines only - uses ^ and $ anchors
    ///   * Content search: "error" matches "error" (alone) but not "this error happened"
    ///   * File search: Less useful, but supported for API completeness
    ///
    /// Replaces the deprecated `word_boundary` boolean parameter
    #[serde(default)]
    pub boundary_mode: Option<String>,

    /// DEPRECATED: Use `boundary_mode="word`" instead. Provided for backward compatibility.
    /// If set to true, overrides `boundary_mode` to "word"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word_boundary: Option<bool>,

    /// Output mode: "full", "`files_only`", or "`count_per_file`" (default: "full")
    /// full: Complete match details with file, line, and content
    /// `files_only`: Only unique file paths (like rg -l)
    /// `count_per_file`: File paths with match counts (like rg -c)
    #[serde(default)]
    pub output_mode: SearchOutputMode,

    /// DEPRECATED: Use `output_mode="files_only`" instead. Provided for backward compatibility.
    /// If set to true, overrides `output_mode` to `FilesOnly`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files_with_matches: Option<bool>,

    /// Invert match - show lines/files that DON'T match the pattern
    /// Matches ripgrep's --invert-match flag
    /// Essential for negative searches and gap analysis
    #[serde(default)]
    pub invert_match: bool,

    /// Regex engine choice: "auto", "rust", or "pcre2" (default: "auto")
    /// Auto tries Rust first, falls back to PCRE2 on syntax errors
    /// PCRE2 supports backreferences and look-around assertions
    #[serde(default)]
    pub engine: EngineChoice,

    /// Preprocessor command to run on files before searching
    /// The command receives the file path as first argument
    /// Example: "pandoc" to search Markdown as plain text
    #[serde(default)]
    pub preprocessor: Option<String>,

    /// Glob patterns for files to run through preprocessor
    #[serde(default)]
    pub preprocessor_globs: Vec<String>,

    /// Enable searching inside compressed files (.gz, .zip, .bz2, .xz)
    #[serde(default)]
    pub search_zip: bool,

    /// Binary file handling mode (default: Auto)
    /// Matches ripgrep's --binary and -a/--text flags
    #[serde(default)]
    pub binary_mode: BinaryMode,

    /// Enable multiline pattern matching (rg --multiline)
    #[serde(default)]
    pub multiline: bool,

    /// Skip files larger than this size in bytes (None = unlimited)
    #[serde(default)]
    pub max_filesize: Option<u64>,

    /// Maximum directory depth to traverse (None = unlimited)
    /// Matches ripgrep's --max-depth flag
    /// 0 = root only, 1 = root + immediate children, etc.
    /// Essential for performance in monorepos with deep dependency trees
    #[serde(default)]
    pub max_depth: Option<usize>,

    /// Return only the matched portion of text, not the entire line
    #[serde(default)]
    pub only_matching: bool,

    /// List all files without searching (like rg --files)
    #[serde(default)]
    pub list_files_only: bool,

    /// Sort results by specified criterion (None = no sorting, filesystem order)
    #[serde(default)]
    pub sort_by: Option<SortBy>,

    /// Sort direction (default: Ascending if `sort_by` is specified)
    #[serde(default)]
    pub sort_direction: Option<SortDirection>,

    /// Text encoding (None = auto-detect)
    #[serde(default)]
    pub encoding: Option<String>,
}

/// Prompt arguments for `start_search` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct StartSearchPromptArgs {}

// ============================================================================
// GET MORE SEARCH RESULTS
// ============================================================================

fn default_length() -> usize {
    100
}

/// Arguments for `get_more_search_results` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetMoreSearchResultsArgs {
    /// Search session ID from `start_search`
    pub session_id: String,

    /// Start result index (default: 0)
    /// Positive: Start from result N (0-based)
    /// Negative: Read last N results (tail behavior)
    #[serde(default)]
    pub offset: i64,

    /// Max results to read (default: 100)
    /// Ignored when offset is negative
    #[serde(default = "default_length")]
    pub length: usize,
}

/// Prompt arguments for `get_more_search_results` tool
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetMoreSearchResultsPromptArgs {}

// ============================================================================
// STOP SEARCH
// ============================================================================

/// Arguments for `stop_search` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct StopSearchArgs {
    /// Search session ID to stop
    pub session_id: String,
}

/// Prompt arguments for `stop_search` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct StopSearchPromptArgs {}

// ============================================================================
// LIST SEARCHES
// ============================================================================

/// Arguments for `list_searches` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ListSearchesArgs {}

/// Prompt arguments for `list_searches` tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ListSearchesPromptArgs {}
