use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_tool::Tool;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use super::{SearchManager, SearchSessionOptions, SearchType, CaseMode, SearchOutputMode, BoundaryMode};

// ============================================================================
// HELPERS
// ============================================================================

fn default_search_type() -> SearchType {
    SearchType::Files
}

// ============================================================================
// TOOL ARGUMENTS
// ============================================================================

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
    pub engine: super::EngineChoice,
    
    /// Preprocessor command to run on files before searching
    /// The command receives the file path as first argument
    /// Example: "pandoc" to search Markdown as plain text
    #[serde(default)]
    pub preprocessor: Option<String>,
    
    /// Glob patterns for files to run through preprocessor
    /// If empty, all files are preprocessed
    /// Example: ["*.md", "*.rst"]
    #[serde(default)]
    pub preprocessor_globs: Vec<String>,
    
    /// Search inside compressed files (.gz, .zip, .bz2, .xz)
    #[serde(default)]
    pub search_zip: bool,
    
    /// Binary file handling mode: "auto", "binary", or "text" (default: "auto")
    /// Matches ripgrep's --binary and -a/--text flags
    /// auto: automatically skip binary files (default rg behavior)
    /// binary: search binary files but suppress binary content (rg --binary)
    /// text: treat all files as text (rg -a/--text)
    #[serde(default)]
    pub binary_mode: super::BinaryMode,
    
    /// Enable multiline mode - allows patterns to match across line boundaries
    /// Matches ripgrep's --multiline flag
    /// When enabled, the '.' metacharacter matches newlines
    /// Essential for complex code structure patterns
    #[serde(default)]
    pub multiline: bool,
    
    /// Skip files larger than this size in bytes (None = unlimited)
    /// Matches ripgrep's --max-filesize flag
    /// Recommended: 1048576 (1MB) to skip minified bundles and lock files
    /// Essential for performance: avoids searching huge generated/minified files
    #[serde(default)]
    pub max_filesize: Option<u64>,
    
    /// Maximum directory depth to traverse (0 = root only, None = unlimited)
    /// Matches ripgrep's --max-depth flag
    /// Essential for performance in monorepos with deep dependency trees
    /// Example: `max_depth=3` searches root + 3 levels of subdirectories
    /// Common values: 1 (root+children), 3-4 (avoid deep `node_modules`)
    #[serde(default)]
    pub max_depth: Option<u32>,

    /// Return only the matched portion of text, not the entire line
    /// Matches ripgrep's --only-matching flag
    /// Perfect for data extraction (URLs, function names, patterns)
    #[serde(default)]
    pub only_matching: bool,

    /// List all files without searching (like rg --files)
    /// When true, lists all files that would be searched (respecting gitignore, types, etc.)
    /// Ignores the pattern parameter - this is pure file discovery, not pattern matching
    /// Useful for AI agents to discover what files exist in a project
    #[serde(default)]
    pub list_files_only: bool,

    /// Sort results by specified criterion: "path", "modified", "accessed", or "created"
    /// When enabled, all results are collected before sorting (disables streaming)
    /// Platform support varies: modified (all), accessed (most), created (Windows/some Unix)
    /// Files with missing timestamps are sorted to end (ascending) or beginning (descending)
    #[serde(default)]
    pub sort_by: Option<super::SortBy>,

    /// Sort direction: "ascending" or "descending" (default: ascending if `sort_by` specified)
    /// ascending: oldest first (time) or A-Z (path)
    /// descending: newest first (time) or Z-A (path)
    #[serde(default)]
    pub sort_direction: Option<super::SortDirection>,

    /// Text encoding (default: auto-detect)
    /// Examples: "auto", "utf8", "utf16le", "utf16be", "latin1", "shiftjis", "gb2312", "euckr"
    /// Ripgrep encoding names: <https://docs.rs/encoding_rs/latest/encoding_rs/#statics>
    #[serde(default)]
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct StartSearchPromptArgs {}

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct StartSearchTool {
    manager: Arc<SearchManager>,
}

impl StartSearchTool {
    #[must_use]
    pub fn new(manager: Arc<SearchManager>) -> Self {
        Self { manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for StartSearchTool {
    type Args = StartSearchArgs;
    type PromptArgs = StartSearchPromptArgs;


    fn name() -> &'static str {
        "start_search"
    }

    fn description() -> &'static str {
        "Start a streaming search that can return results progressively.\n\n\
         SEARCH STRATEGY GUIDE:\n\
         Choose the right search type based on what the user is looking for:\n\n\
         USE search_type=\"files\" WHEN:\n\
         - User asks for specific files: \"find package.json\", \"locate config files\"\n\
         - Pattern looks like a filename: \"*.js\", \"README.md\", \"test-*.tsx\"\n\
         - User wants to find files by name/extension: \"all TypeScript files\", \"Python scripts\"\n\
         - Looking for configuration/setup files: \".env\", \"dockerfile\", \"tsconfig.json\"\n\n\
         USE search_type=\"content\" WHEN:\n\
         - User asks about code/logic: \"authentication logic\", \"error handling\", \"API calls\"\n\
         - Looking for functions/variables: \"getUserData function\", \"useState hook\"\n\
         - Searching for text/comments: \"TODO items\", \"FIXME comments\", \"documentation\"\n\
         - Finding patterns in code: \"console.log statements\", \"import statements\"\n\
         - User describes functionality: \"components that handle login\", \"files with database queries\"\n\n\
         WHEN UNSURE OR USER REQUEST IS AMBIGUOUS:\n\
         Run TWO searches in parallel - one for files and one for content:\n\n\
         Example approach for ambiguous queries like \"find authentication stuff\":\n\
         1. Start file search: search_type=\"files\", pattern=\"auth\"\n\
         2. Simultaneously start content search: search_type=\"content\", pattern=\"authentication\"\n\
         3. Present combined results: \"Found 3 auth-related files and 8 files containing authentication code\"\n\n\
         SEARCH TYPES:\n\
         - search_type=\"files\": Find files by name (pattern matches file names)\n\
         - search_type=\"content\": Search inside files for text patterns\n\n\
         PATTERN MATCHING MODES:\n\
         - Default (literal_search=false): Patterns are regex (matches ripgrep behavior)\n\
         - Literal mode (literal_search=true): Patterns are treated as exact strings\n\
         - Smart case (case_mode=\"smart\"): Auto case-insensitive for all-lowercase patterns\n\
         - Boundary modes (boundary_mode parameter):\n\
           * null/omitted: Match pattern anywhere (substring matching, default)\n\
           * \"word\": Match whole words only (uses \\b anchors)\n\
             - Content: 'test' matches 'test()' but not 'testing'\n\
             - Files: 'lib' matches 'lib.rs' but not 'libtest.rs'\n\
           * \"line\": Match complete lines only (uses ^ and $ anchors)\n\
             - Content: 'error' matches 'error' alone but not 'this error happened'\n\
             - Files: Less useful but supported\n\
         Note: Simple strings like \"start_crawl\" work as regex and will match literally\n\n\
         IMPORTANT PARAMETERS:\n\
         - pattern: What to search for (file names OR content text)\n\
         - literal_search: Use exact string matching instead of regex (default: false)\n\
         - boundary_mode: \"word\", \"line\", or null for pattern boundaries (default: null)\n\
         - multiline (default: false): Enable multiline pattern matching (rg --multiline)\n\
           * Allows patterns to span multiple lines\n\
           * Makes '.' match newline characters\n\
           * Essential for structural code analysis\n\
         - case_mode: \"sensitive\", \"insensitive\", or \"smart\" (default: \"sensitive\")\n\
           Smart case: case-insensitive if pattern is all lowercase, sensitive otherwise\n\
         - file_pattern: Optional filter to limit search to specific file types (e.g., \"*.js\", \"package.json\")\n\
         - files_with_matches: Return only file paths containing matches, not line details (rg -l)\n\
           Only works with search_type=\"content\". Stops after first match per file for performance.\n\
         - early_termination: Stop search early when exact filename match is found (optional: defaults to true for file searches, false for content searches)\n\
         - only_matching: Return only the matched portion of text, not entire lines (rg -o)\n\
           Only works with search_type=\"content\". Perfect for data extraction.\n\
           Examples: Extract URLs, function names, version numbers, email addresses\n\
         - max_depth: Limit directory traversal depth (default: unlimited)\n\
           * Essential for performance in monorepos with deep dependency trees (node_modules, vendor, target)\n\
           * Example: max_depth=3 searches root + 3 levels, skipping deeper directories\n\
           * Common values: 1 (root+children only), 3-4 (avoid deep node_modules/dependencies)\n\
           * Matches ripgrep's --max-depth flag\n\
           * Can provide 10-25x speedup by avoiding irrelevant deep directories\n\n\
         - max_filesize: Skip files larger than specified size in bytes (default: None/unlimited)\n\
           * Matches ripgrep's --max-filesize flag\n\
           * Essential for performance: avoids huge minified bundles, lock files, generated code\n\
           * Recommended: 1048576 (1MB) for most searches\n\
           * Skips: bundle.min.js (15MB), package-lock.json (12MB), Cargo.lock (1-10MB)\n\
           * Common values:\n\
             - 102400 (100KB): Ultra-fast, only small source files\n\
             - 1048576 (1MB): Recommended - Skip minified bundles and lock files\n\
             - 5242880 (5MB): Conservative - Allow large source, skip huge bundles\n\
           * Can provide 10-30x speedup by avoiding huge files that waste search time\n\
           * Use with max_depth for maximum performance in large projects\n\
         - encoding: Text encoding for file content (default: \"auto\")\n\
           * Supports any encoding_rs name: utf8, utf16le, utf16be, latin1, shiftjis, gb2312, euckr, etc.\n\
           * Use when: Mojibake in results, legacy codebases, international projects\n\
           * Examples: encoding=\"utf16le\" for Windows files, encoding=\"shiftjis\" for Japanese code\n\n\
         DECISION EXAMPLES:\n\
         - \"find package.json\" → search_type=\"files\", pattern=\"package.json\" (specific file)\n\
         - \"find authentication components\" → search_type=\"content\", pattern=\"authentication\" (looking for functionality)\n\
         - \"locate all React components\" → search_type=\"files\", pattern=\"*.tsx\" or \"*.jsx\" (file pattern)\n\
         - \"find TODO comments\" → search_type=\"content\", pattern=\"TODO\" (text in files)\n\
         - \"show me login files\" → AMBIGUOUS → run both: files with \"login\" AND content with \"login\"\n\
         - \"find config\" → AMBIGUOUS → run both: config files AND files containing config code\n\n\
         COMPREHENSIVE SEARCH EXAMPLES:\n\
         - Find package.json files: search_type=\"files\", pattern=\"package.json\"\n\
         - Find all JS files: search_type=\"files\", pattern=\"*.js\"\n\
         - Search for TODO in code: search_type=\"content\", pattern=\"TODO\", file_pattern=\"*.js|*.ts\"\n\
         - Search for exact code: search_type=\"content\", pattern=\"toast.error('test')\", literal_search=true\n\
         - Search whole words: search_type=\"content\", pattern=\"test\", boundary_mode=\"word\"\n\
           (matches 'test()' and 'test ' but not 'testing' or 'attest')\n\
         - Find exact filename: search_type=\"files\", pattern=\"lib\", boundary_mode=\"word\"\n\
           (matches 'lib.rs' but not 'libtest.rs')\n\
         - Match complete lines: search_type=\"content\", pattern=\"error\", boundary_mode=\"line\"\n\
           (matches 'error' alone but not 'this error happened' or '  error  ')\n\
         - Ambiguous request \"find auth stuff\": Run two searches:\n\
           1. search_type=\"files\", pattern=\"auth\"\n\
           2. search_type=\"content\", pattern=\"authentication\"\n\
         - Extract URLs: search_type=\"content\", pattern=\"https?://[^\\\\s]+\", only_matching=true\n\
           (returns just \"https://example.com\" not full line)\n\
         - Extract function names: search_type=\"content\", pattern=\"fn (\\\\w+)\\\\(\", only_matching=true\n\
         - Extract version numbers: search_type=\"content\", pattern=\"\\\\d+\\\\.\\\\d+\\\\.\\\\d+\", only_matching=true\n\n\
         PRO TIP: When user requests are ambiguous about whether they want files or content,\n\
         run both searches concurrently and combine results for comprehensive coverage.\n\n\
         Unlike regular search tools, this starts a background search process and returns\n\
         immediately with a session ID. Use get_more_search_results to get results as they\n\
         come in, and stop_search to stop the search early if needed.\n\n\
         Perfect for large directories where you want to see results immediately and\n\
         have the option to cancel if the search takes too long or you find what you need.\n\n\
         IMPORTANT: Always use absolute paths for reliability. Paths are automatically normalized regardless of slash direction. Relative paths may fail as they depend on the current working directory. Tilde paths (~/...) might not work in all contexts. Unless the user explicitly asks for relative paths, use absolute paths."
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn open_world() -> bool {
        false
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Handle backward compatibility: ignore_case overrides case_mode if present
        let case_mode = if let Some(ignore_case) = args.ignore_case {
            if ignore_case {
                CaseMode::Insensitive
            } else {
                CaseMode::Sensitive
            }
        } else {
            args.case_mode
        };
        
        // Handle backward compatibility: word_boundary overrides boundary_mode if present
        let boundary_mode = if let Some(true) = args.word_boundary {
            log::warn!("word_boundary is deprecated, use boundary_mode='word' instead");
            Some(BoundaryMode::Word)
        } else {
            // Parse boundary_mode string to enum
            match args.boundary_mode.as_deref() {
                None => None,
                Some("word") => Some(BoundaryMode::Word),
                Some("line") => Some(BoundaryMode::Line),
                Some(other) => {
                    return Err(McpError::InvalidArguments(
                        format!(
                            "Invalid boundary_mode '{other}'. Must be 'word', 'line', or null"
                        )
                    ));
                }
            }
        };
        
        // Validate files_with_matches only works with content search (deprecated parameter)
        if args.files_with_matches == Some(true) && args.search_type != SearchType::Content {
            return Err(McpError::InvalidArguments(
                "files_with_matches can only be used with search_type 'content'".to_string()
            ));
        }
        
        // Validate only_matching only works with content search
        if args.only_matching && args.search_type != SearchType::Content {
            return Err(McpError::InvalidArguments(
                "only_matching can only be used with search_type 'content'".to_string()
            ));
        }
        
        // Warn if only_matching + invert_match (illogical combination)
        if args.only_matching && args.invert_match {
            log::warn!("only_matching + invert_match: nothing to extract from non-matches, ignoring only_matching");
        }
        // Handle deprecated files_with_matches - convert to output_mode
        let output_mode = if let Some(true) = args.files_with_matches {
            log::warn!("files_with_matches is deprecated, use output_mode='files_only' instead");
            SearchOutputMode::FilesOnly
        } else {
            args.output_mode
        };
        
        // Warn if only_matching with non-Full output mode (only_matching has no effect)
        if args.only_matching && output_mode != SearchOutputMode::Full {
            log::warn!("only_matching with output_mode={output_mode:?}: non-Full modes don't have match text, ignoring only_matching");
        }
        
        let options = SearchSessionOptions {
            root_path: args.path,
            pattern: args.pattern,
            search_type: args.search_type,
            file_pattern: args.file_pattern,
            r#type: args.r#type,
            type_not: args.type_not,
            case_mode,  // Changed from ignore_case
            max_results: args.max_results,
            include_hidden: args.include_hidden,
            no_ignore: args.no_ignore,
            context: args.context,
            before_context: args.before_context,
            after_context: args.after_context,
            timeout_ms: args.timeout_ms,
            early_termination: args.early_termination,
            literal_search: args.literal_search,
            boundary_mode,  // Changed from word_boundary
            output_mode,
            invert_match: args.invert_match,
            engine: args.engine,
            preprocessor: args.preprocessor,
            preprocessor_globs: args.preprocessor_globs,
            search_zip: args.search_zip,
            binary_mode: args.binary_mode,
            multiline: args.multiline,
            max_filesize: args.max_filesize,
            max_depth: args.max_depth.map(|d| d as usize),
            only_matching: args.only_matching,
            list_files_only: args.list_files_only,
            sort_by: args.sort_by,
            sort_direction: args.sort_direction,
            encoding: args.encoding,
        };
        
        let response = self.manager.start_search(options).await?;
        
        Ok(json!({
            "session_id": response.session_id,
            "is_complete": response.is_complete,
            "is_error": response.is_error,
            "results": response.results,
            "total_results": response.total_results,
            "runtime_ms": response.runtime_ms,
            "error_count": response.error_count,
            "max_results": response.max_results,
            "results_limited": response.results_limited,
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I use streaming search?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The start_search tool starts a background search that returns results progressively:\n\n\
                     1. File search:\n\
                        start_search({\n\
                          \"path\": \"/path/to/search\",\n\
                          \"pattern\": \"package.json\",\n\
                          \"search_type\": \"files\"\n\
                        })\n\n\
                     2. Content search:\n\
                        start_search({\n\
                          \"path\": \".\",\n\
                          \"pattern\": \"TODO\",\n\
                          \"search_type\": \"content\",\n\
                          \"file_pattern\": \"*.rs\"\n\
                        })\n\n\
                     Returns session_id immediately. Use get_more_search_results to fetch results."
                ),
            },
        ])
    }
}
