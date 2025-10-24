//! File search implementation using glob patterns and parallel directory traversal
//!
//! This module provides the visitor and builder for searching files by name
//! with support for glob patterns and exact matching.

use super::super::types::{SearchError, SearchResult, SearchResultType};
use super::config::{
    DEFAULT_MAX_RESULTS, LAST_READ_UPDATE_INTERVAL_MS, LAST_READ_UPDATE_MATCH_THRESHOLD,
    MAX_DETAILED_ERRORS, RESULT_BUFFER_SIZE,
};
use ignore::{DirEntry, ParallelVisitor, ParallelVisitorBuilder};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Parallel visitor builder for file search
pub(super) struct FileSearchBuilder {
    pub(super) glob_pattern: Option<globset::GlobMatcher>,
    pub(super) pattern: String,
    pub(super) pattern_lower: String,
    pub(super) case_mode: super::super::types::CaseMode,
    pub(super) is_pattern_lowercase: bool,
    pub(super) word_boundary: bool,
    pub(super) max_results: usize,
    pub(super) early_termination: bool,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) last_read_time_atomic: Arc<AtomicU64>,
    pub(super) cancellation_rx: watch::Receiver<bool>,
    pub(super) first_result_tx: watch::Sender<bool>,
    pub(super) was_incomplete: Arc<RwLock<bool>>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    pub(super) start_time: Instant,
}

impl<'s> ParallelVisitorBuilder<'s> for FileSearchBuilder {
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        Box::new(FileSearchVisitor {
            glob_pattern: self.glob_pattern.clone(),
            pattern: self.pattern.clone(),
            pattern_lower: self.pattern_lower.clone(),
            case_mode: self.case_mode,
            is_pattern_lowercase: self.is_pattern_lowercase,
            word_boundary: self.word_boundary,
            max_results: self.max_results,
            early_termination: self.early_termination,
            results: Arc::clone(&self.results),
            total_matches: Arc::clone(&self.total_matches),
            last_read_time_atomic: Arc::clone(&self.last_read_time_atomic),
            cancellation_rx: self.cancellation_rx.clone(),
            first_result_tx: self.first_result_tx.clone(),
            was_incomplete: Arc::clone(&self.was_incomplete),
            error_count: Arc::clone(&self.error_count),
            errors: Arc::clone(&self.errors),
            buffer: Vec::with_capacity(RESULT_BUFFER_SIZE),
            last_update_time: Instant::now(),
            matches_since_update: 0,
            start_time: self.start_time,
        })
    }
}

/// Parallel visitor for file search
pub(super) struct FileSearchVisitor {
    glob_pattern: Option<globset::GlobMatcher>,
    pattern: String,
    pattern_lower: String,
    case_mode: super::super::types::CaseMode,
    is_pattern_lowercase: bool,
    word_boundary: bool,
    max_results: usize,
    early_termination: bool,
    results: Arc<RwLock<Vec<SearchResult>>>,
    total_matches: Arc<AtomicUsize>,
    last_read_time_atomic: Arc<AtomicU64>,
    cancellation_rx: watch::Receiver<bool>,
    first_result_tx: watch::Sender<bool>,
    was_incomplete: Arc<RwLock<bool>>,
    error_count: Arc<AtomicUsize>,
    errors: Arc<RwLock<Vec<SearchError>>>,
    /// Thread-local buffer for batching results
    buffer: Vec<SearchResult>,
    /// Last time we updated the shared `last_read_time`
    /// Used in `maybe_update_last_read_time()` to throttle lock acquisitions
    last_update_time: Instant,
    start_time: Instant,
    /// Number of matches since last update
    /// Used in `maybe_update_last_read_time()` to throttle lock acquisitions
    matches_since_update: usize,
}

impl FileSearchVisitor {
    /// Check if pattern matches with word boundaries
    ///
    /// Word boundaries are: '.', '-', '_', '/', or start/end of string
    /// E.g., "lib" matches "lib.rs" but not "libtest.rs"
    fn matches_with_word_boundary(
        file_name: &str,
        pattern: &str,
        pattern_lower: &str,
        case_mode: super::super::types::CaseMode,
        is_pattern_lowercase: bool,
    ) -> bool {
        use super::super::types::CaseMode;

        /// Check if character is a word boundary separator
        fn is_boundary(c: char) -> bool {
            matches!(c, '.' | '-' | '_' | '/')
        }

        // Determine comparison strings based on case mode
        let (search_in, search_for) = match case_mode {
            CaseMode::Insensitive => (file_name.to_lowercase(), pattern_lower.to_string()),
            CaseMode::Smart => {
                if is_pattern_lowercase {
                    (file_name.to_lowercase(), pattern_lower.to_string())
                } else {
                    (file_name.to_string(), pattern.to_string())
                }
            }
            CaseMode::Sensitive => (file_name.to_string(), pattern.to_string()),
        };

        // Find all occurrences of the pattern
        let mut start = 0;
        while let Some(pos) = search_in[start..].find(&search_for) {
            let match_pos = start + pos;
            let match_end = match_pos + search_for.len();

            // Check if match is at start or preceded by boundary
            let before_ok = match_pos == 0 || {
                search_in[..match_pos]
                    .chars()
                    .last()
                    .is_some_and(is_boundary)
            };

            // Check if match is at end or followed by boundary
            let after_ok = match_end == search_in.len() || {
                search_in[match_end..]
                    .chars()
                    .next()
                    .is_some_and(is_boundary)
            };

            // If both boundaries are satisfied, we have a match
            if before_ok && after_ok {
                return true;
            }

            // Move past this occurrence and continue searching
            start = match_pos + 1;
        }

        false
    }

    /// Check if this is an exact match (not a partial/wildcard match)
    ///
    /// Returns true only when:
    /// - Glob pattern has no wildcards and matches filename exactly, OR
    /// - Literal pattern equals filename exactly (respecting `case_mode`)
    fn is_exact_match(&self, file_name: &str) -> bool {
        use super::super::types::CaseMode;

        // Word boundary mode: must match entire filename
        if self.word_boundary {
            if let Some(ref glob) = self.glob_pattern {
                return glob.is_match(file_name);
            }
            return match self.case_mode {
                CaseMode::Insensitive => file_name.eq_ignore_ascii_case(&self.pattern),
                CaseMode::Smart => {
                    if self.is_pattern_lowercase {
                        file_name.eq_ignore_ascii_case(&self.pattern)
                    } else {
                        file_name == self.pattern
                    }
                }
                CaseMode::Sensitive => file_name == self.pattern,
            };
        }

        // Original logic: check for exact match in non-word-boundary mode
        if let Some(ref glob) = self.glob_pattern {
            // Check if glob pattern has no wildcards
            let has_wildcards = self.pattern.contains('*')
                || self.pattern.contains('?')
                || self.pattern.contains('[');

            // Not exact if pattern contains wildcards
            if has_wildcards {
                return false;
            }

            // Exact match if no wildcards and pattern matches
            glob.is_match(file_name)
        } else {
            // For literal/substring matching, exact means equality
            match self.case_mode {
                CaseMode::Insensitive => file_name.eq_ignore_ascii_case(&self.pattern),
                CaseMode::Smart => {
                    // Smart: case-insensitive if pattern is all lowercase
                    if self.is_pattern_lowercase {
                        file_name.eq_ignore_ascii_case(&self.pattern)
                    } else {
                        file_name == self.pattern
                    }
                }
                CaseMode::Sensitive => file_name == self.pattern,
            }
        }
    }

    /// Track a directory traversal error
    fn track_error(&self, error: &ignore::Error) {
        self.error_count.fetch_add(1, Ordering::SeqCst);

        log::debug!("File search error: {error}");

        // Check if we should store BEFORE allocating
        let should_store = {
            let errors = self.errors.blocking_read();
            errors.len() < MAX_DETAILED_ERRORS
        };

        if should_store {
            // Only allocate if we're going to use it
            let error_str = error.to_string();
            let path_str = error_str
                .split(':')
                .next()
                .unwrap_or("<unknown>")
                .to_string();

            let mut errors = self.errors.blocking_write();
            // Double-check in case another thread added while we were allocating
            if errors.len() < MAX_DETAILED_ERRORS {
                errors.push(SearchError {
                    path: path_str,
                    message: error_str,
                    error_type: Self::categorize_error(error),
                });
            }
        }
    }

    fn categorize_error(error: &ignore::Error) -> String {
        let err_str = error.to_string().to_lowercase();
        if err_str.contains("permission denied") {
            "permission_denied".to_string()
        } else if err_str.contains("broken pipe") || err_str.contains("i/o error") {
            "io_error".to_string()
        } else if err_str.contains("invalid") {
            "invalid_path".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Flush buffered results to shared storage
    fn flush_buffer(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        // Check if this is the first batch of results
        let was_empty = self.results.blocking_read().is_empty();

        // Single lock acquisition for entire buffer
        {
            let mut results_guard = self.results.blocking_write();
            results_guard.extend(self.buffer.drain(..));
        }

        // Signal first result if this was the first batch
        if was_empty {
            let _ = self.first_result_tx.send(true);
        }

        // Update last read time once per flush
        {
            let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
            self.last_read_time_atomic
                .store(elapsed_micros, Ordering::Relaxed);
        }
    }

    /// Add result to buffer, flush if full
    fn add_result(&mut self, result: SearchResult) {
        self.buffer.push(result);

        if self.buffer.len() >= RESULT_BUFFER_SIZE {
            self.flush_buffer();
        }
    }

    /// Update `last_read_time` if throttle threshold exceeded
    fn maybe_update_last_read_time(&mut self) {
        self.matches_since_update += 1;

        let now = Instant::now();
        let time_since_update = now.duration_since(self.last_update_time);

        let should_update = time_since_update.as_millis() as u64 >= LAST_READ_UPDATE_INTERVAL_MS
            || self.matches_since_update >= LAST_READ_UPDATE_MATCH_THRESHOLD;

        if should_update {
            let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
            self.last_read_time_atomic
                .store(elapsed_micros, Ordering::Relaxed);
            self.last_update_time = now;
            self.matches_since_update = 0;
        }
    }

    /// Force update `last_read_time` (used in Drop)
    fn force_update_last_read_time(&mut self) {
        let now = Instant::now();
        let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
        self.last_read_time_atomic
            .store(elapsed_micros, Ordering::Relaxed);
        self.last_update_time = now;
        self.matches_since_update = 0;
    }
}

impl ParallelVisitor for FileSearchVisitor {
    fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        // Check for cancellation
        if *self.cancellation_rx.borrow() {
            *self.was_incomplete.blocking_write() = true;
            return ignore::WalkState::Quit;
        }

        // Check if we've reached max results
        if self.total_matches.load(Ordering::SeqCst) >= self.max_results {
            return ignore::WalkState::Quit;
        }

        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                self.track_error(&err);
                return ignore::WalkState::Continue;
            }
        };

        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check if filename matches pattern
        // Try glob pattern first, then fall back to substring or word boundary matching
        let matches = if let Some(ref glob) = self.glob_pattern {
            glob.is_match(file_name)
        } else {
            use super::super::types::CaseMode;
            if self.word_boundary {
                // Word boundary: pattern must be surrounded by word boundaries
                // Boundaries are: '.', '-', '_', '/', or start/end of string
                Self::matches_with_word_boundary(
                    file_name,
                    &self.pattern,
                    &self.pattern_lower,
                    self.case_mode,
                    self.is_pattern_lowercase,
                )
            } else {
                // Substring match (current behavior)
                let file_name_lower = file_name.to_lowercase();
                match self.case_mode {
                    CaseMode::Insensitive => file_name_lower.contains(&self.pattern_lower),
                    CaseMode::Smart => {
                        // Smart: case-insensitive if pattern is all lowercase
                        if self.is_pattern_lowercase {
                            file_name_lower.contains(&self.pattern_lower)
                        } else {
                            file_name.contains(&self.pattern)
                        }
                    }
                    CaseMode::Sensitive => file_name.contains(&self.pattern),
                }
            }
        };

        if matches {
            // Atomically reserve a slot before processing
            match self
                .total_matches
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                    if current < self.max_results {
                        Some(current + 1) // Reserve slot atomically
                    } else {
                        None // Limit reached
                    }
                }) {
                Ok(_) => {
                    // Slot reserved - collect metadata and add result
                    let entry_metadata = entry.metadata().ok();
                    let modified = entry_metadata.as_ref().and_then(|m| m.modified().ok());
                    let accessed = entry_metadata.as_ref().and_then(|m| m.accessed().ok());
                    let created = entry_metadata.as_ref().and_then(|m| m.created().ok());

                    let search_result = SearchResult {
                        file: path.display().to_string(),
                        line: None,
                        r#match: None,
                        r#type: SearchResultType::File,
                        is_context: false,
                        is_binary: None,
                        binary_suppressed: None,
                        modified,
                        accessed,
                        created,
                    };

                    // Use buffer instead of direct push
                    self.add_result(search_result);

                    // Update last_read_time with throttling
                    self.maybe_update_last_read_time();

                    // Check for early termination on exact match
                    if self.early_termination && self.is_exact_match(file_name) {
                        return ignore::WalkState::Quit;
                    }
                }
                Err(_) => {
                    // Limit reached - quit searching
                    return ignore::WalkState::Quit;
                }
            }
        }

        ignore::WalkState::Continue
    }
}

impl Drop for FileSearchVisitor {
    fn drop(&mut self) {
        // Flush any remaining buffered results
        // This is CRITICAL - prevents losing the last batch of results
        self.flush_buffer();
        // Ensure final last_read_time update
        self.force_update_last_read_time();
    }
}

/// Execute file search using walker with parallel directory traversal
pub(super) fn execute(
    options: &super::super::types::SearchSessionOptions,
    root: &std::path::PathBuf,
    ctx: &mut super::context::SearchContext,
) {
    use super::super::rg::flags::hiargs::HiArgs;
    use super::super::rg::flags::lowargs::{LowArgs, Mode, PatternSource, SearchMode};
    use super::utils::{build_type_changes, configure_walker, convert_case_mode};
    use ignore::WalkBuilder;
    use std::sync::Arc;

    // Try to compile as glob pattern first
    let glob_pattern = if options.literal_search {
        None
    } else {
        globset::Glob::new(&options.pattern)
            .ok()
            .map(|g| g.compile_matcher())
    };
    let pattern_lower = options.pattern.to_lowercase();

    // Precompute smart case flag once (performance optimization)
    let is_pattern_lowercase = options.pattern.chars().all(|c| !c.is_uppercase());

    let max_results = options.max_results.unwrap_or(DEFAULT_MAX_RESULTS as u32) as usize;

    // Build type_changes for ripgrep
    let type_changes = build_type_changes(options);

    // Build LowArgs for type filtering
    let low_args = LowArgs {
        patterns: vec![PatternSource::Regexp(options.pattern.clone())],
        case: convert_case_mode(options.case_mode),
        fixed_strings: options.literal_search,
        hidden: options.include_hidden,
        invert_match: options.invert_match,
        mode: Mode::Search(SearchMode::Standard),
        type_changes,
        // Match ripgrep's --no-ignore flag behavior exactly
        no_ignore_vcs: options.no_ignore,
        no_ignore_exclude: options.no_ignore,
        no_ignore_global: options.no_ignore,
        no_ignore_parent: options.no_ignore,
        no_ignore_dot: options.no_ignore,
        ..Default::default()
    };

    // Build HiArgs for types
    let hi_args = match HiArgs::from_low_args(low_args) {
        Ok(h) => Arc::new(h),
        Err(e) => {
            log::error!("Failed to build HiArgs: {e}");
            ctx.is_complete.store(true, Ordering::Release);
            return;
        }
    };

    // Build directory walker with gitignore support and parallel traversal
    let mut walker = WalkBuilder::new(root);
    configure_walker(&mut walker, &hi_args);

    // Use HiArgs.types() - handles built-in types + file_pattern
    walker.types(hi_args.types().clone());

    // Build the parallel visitor
    let mut builder = FileSearchBuilder {
        glob_pattern,
        pattern: options.pattern.clone(),
        pattern_lower,
        case_mode: options.case_mode,
        is_pattern_lowercase,
        word_boundary: matches!(
            options.boundary_mode,
            Some(super::super::types::BoundaryMode::Word)
        ),
        max_results,
        early_termination: options.early_termination.unwrap_or(false),
        results: Arc::clone(&ctx.results),
        total_matches: Arc::clone(&ctx.total_matches),
        last_read_time_atomic: Arc::clone(&ctx.last_read_time_atomic),
        cancellation_rx: ctx.cancellation_rx.clone(),
        first_result_tx: ctx.first_result_tx.clone(),
        was_incomplete: Arc::clone(&ctx.was_incomplete),
        error_count: Arc::clone(&ctx.error_count),
        errors: Arc::clone(&ctx.errors),
        start_time: ctx.start_time,
    };

    // Execute parallel search
    walker.build_parallel().visit(&mut builder);

    // Log error summary if any errors occurred
    let error_count = ctx.error_count.load(Ordering::SeqCst);
    if error_count > 0 {
        log::info!(
            "File search completed with {} errors. Pattern: '{}', Path: {}",
            error_count,
            options.pattern,
            root.display()
        );
    }

    // Mark complete
    ctx.is_complete.store(true, Ordering::Release);
}
