//! Content search implementation using grep and parallel directory traversal
//!
//! This module provides the visitor and builder for searching file contents
//! using regex patterns with the `grep` and `ignore` crates.

use super::super::types::{SearchError, SearchOutputMode, SearchResult};
use super::config::{
    LAST_READ_UPDATE_INTERVAL_MS, LAST_READ_UPDATE_MATCH_THRESHOLD, MAX_DETAILED_ERRORS,
    RESULT_BUFFER_SIZE,
};
use crate::search::rg::PatternMatcher;
use ignore::{DirEntry, ParallelVisitor, ParallelVisitorBuilder, WalkBuilder};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Convert MCP API `Engine` to ripgrep's internal `Engine`
fn convert_engine_choice(
    engine: crate::search::types::Engine,
) -> crate::search::rg::flags::lowargs::Engine {
    use crate::search::rg::flags::lowargs::Engine as RG;
    use crate::search::types::Engine as MCP;

    match engine {
        MCP::Auto => RG::Auto,
        MCP::Rust => RG::Default,
        MCP::PCRE2 => RG::PCRE2,
    }
}

/// Parallel visitor builder for content search
pub(super) struct ContentSearchBuilder {
    pub(super) hi_args: Arc<super::super::rg::flags::hiargs::HiArgs>,
    pub(super) matcher: PatternMatcher,
    pub(super) max_results: Option<usize>,
    pub(super) output_mode: SearchOutputMode,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) total_files: Arc<AtomicUsize>,
    pub(super) last_read_time_atomic: Arc<AtomicU64>,
    pub(super) cancellation_rx: watch::Receiver<bool>,
    pub(super) first_result_tx: watch::Sender<bool>,
    pub(super) was_incomplete: Arc<RwLock<bool>>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    // Deduplication for FilesOnly mode
    pub(super) seen_files: Arc<RwLock<HashSet<String>>>,
    // Aggregation for CountPerFile mode
    pub(super) file_counts: Arc<RwLock<HashMap<String, super::super::types::FileCountData>>>,
    pub(super) start_time: Instant,
}

impl<'s> ParallelVisitorBuilder<'s> for ContentSearchBuilder {
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        use super::super::rg::flags::lowargs::SearchMode;

        // Clone Arc (cheap: just pointer copy)
        let hi_args = Arc::clone(&self.hi_args);

        // Clone matcher (cheap: regex already compiled)
        let matcher = self.matcher.clone();

        // Build searcher with error handling
        let searcher = match hi_args.searcher() {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to build searcher: {e}");
                return Box::new(ErrorVisitor {
                    error_message: format!("Searcher initialization failed: {e}"),
                    error_count: Arc::clone(&self.error_count),
                    errors: Arc::clone(&self.errors),
                    was_incomplete: Arc::clone(&self.was_incomplete),
                });
            }
        };

        // Create thread-local buffer for JSON output
        let buffer = Vec::with_capacity(8192);

        // Build printer with thread-local buffer
        let printer = hi_args.printer(SearchMode::Json, buffer);

        // Build SearchWorker with error handling
        let worker = match hi_args.search_worker(matcher, searcher, printer) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to build search worker: {e}");
                return Box::new(ErrorVisitor {
                    error_message: format!("SearchWorker initialization failed: {e}"),
                    error_count: Arc::clone(&self.error_count),
                    errors: Arc::clone(&self.errors),
                    was_incomplete: Arc::clone(&self.was_incomplete),
                });
            }
        };

        // Build HaystackBuilder
        let haystack_builder = super::super::rg::haystack::HaystackBuilder::new();

        Box::new(ContentSearchVisitor {
            worker,
            haystack_builder,
            max_results: self.max_results,
            output_mode: self.output_mode,
            results: Arc::clone(&self.results),
            total_matches: Arc::clone(&self.total_matches),
            total_files: Arc::clone(&self.total_files),
            last_read_time_atomic: Arc::clone(&self.last_read_time_atomic),
            cancellation_rx: self.cancellation_rx.clone(),
            first_result_tx: self.first_result_tx.clone(),
            was_incomplete: Arc::clone(&self.was_incomplete),
            error_count: Arc::clone(&self.error_count),
            errors: Arc::clone(&self.errors),
            seen_files: Arc::clone(&self.seen_files),
            file_counts: Arc::clone(&self.file_counts),
            start_time: self.start_time,
            buffer: Vec::with_capacity(RESULT_BUFFER_SIZE),
            last_update_time: Instant::now(),
            matches_since_update: 0,
        })
    }
}

/// Parallel visitor for content search
pub(super) struct ContentSearchVisitor {
    worker: super::super::rg::search::SearchWorker<Vec<u8>>,
    haystack_builder: super::super::rg::haystack::HaystackBuilder,
    max_results: Option<usize>,
    output_mode: SearchOutputMode,
    results: Arc<RwLock<Vec<SearchResult>>>,
    total_matches: Arc<AtomicUsize>,
    total_files: Arc<AtomicUsize>,
    last_read_time_atomic: Arc<AtomicU64>,
    cancellation_rx: watch::Receiver<bool>,
    first_result_tx: watch::Sender<bool>,
    was_incomplete: Arc<RwLock<bool>>,
    error_count: Arc<AtomicUsize>,
    errors: Arc<RwLock<Vec<SearchError>>>,
    seen_files: Arc<RwLock<HashSet<String>>>,
    file_counts: Arc<RwLock<HashMap<String, super::super::types::FileCountData>>>,
    start_time: Instant,
    /// Thread-local buffer for batching results before flushing to shared storage
    buffer: Vec<SearchResult>,
    /// Last time we updated the shared `last_read_time` (for throttling)
    last_update_time: Instant,
    /// Number of matches since last timestamp update (for throttling)
    matches_since_update: usize,
}

impl ContentSearchVisitor {
    /// Track a directory traversal error
    fn track_error(&self, error: &ignore::Error) {
        // Increment atomic counter (lock-free)
        self.error_count.fetch_add(1, Ordering::SeqCst);

        // Log at debug level
        log::debug!("Search error: {error}");

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
                    error_type: Self::categorize_ignore_error(error),
                });
            }
        }
    }

    /// Categorize `ignore::Error` for user-friendly display
    fn categorize_ignore_error(error: &ignore::Error) -> String {
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
    ///
    /// Acquires write lock ONCE for entire buffer batch.
    /// This is the core optimization: batch writes reduce lock contention.
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

    /// Add result to thread-local buffer, flush if full
    ///
    /// This replaces direct `results.push()` calls to avoid per-match locking.
    fn add_result(&mut self, result: SearchResult) {
        self.buffer.push(result);

        // Flush when buffer reaches capacity
        if self.buffer.len() >= RESULT_BUFFER_SIZE {
            self.flush_buffer();
        }
    }

    /// Update `last_read_time` if throttle threshold exceeded
    ///
    /// Prevents excessive atomic stores by updating only every N matches or T milliseconds.
    fn maybe_update_last_read_time(&mut self) {
        self.matches_since_update += 1;

        let now = Instant::now();
        let time_since_update = now.duration_since(self.last_update_time);

        // Update if time threshold OR match count threshold exceeded
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

impl ParallelVisitor for ContentSearchVisitor {
    fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        use super::super::rg::json_output::parse_json_buffer;

        log::debug!("ContentSearchVisitor::visit() called");

        // Check for cancellation
        if *self.cancellation_rx.borrow() {
            *self.was_incomplete.blocking_write() = true;
            return ignore::WalkState::Quit;
        }

        // Check if we've reached max results (mode-aware)
        if let Some(max) = self.max_results {
            let current_count = match self.output_mode {
                SearchOutputMode::CountPerFile => {
                    // In CountPerFile mode, limit by unique files, not matches
                    self.total_files.load(Ordering::SeqCst)
                }
                _ => {
                    // In Full/FilesOnly modes, limit by matches
                    self.total_matches.load(Ordering::SeqCst)
                }
            };

            if current_count >= max {
                return ignore::WalkState::Quit;
            }
        }

        // Track directory traversal errors
        if let Err(ref err) = entry {
            self.track_error(err);
            return ignore::WalkState::Continue;
        }

        // Extract Ok value - safe because we checked for Err above
        let dent = match entry {
            Ok(d) => d,
            Err(_) => unreachable!("Error case handled above"),
        };

        // Collect file metadata before searching (for sorting support)
        let file_metadata = dent.metadata().ok();
        let modified = file_metadata.as_ref().and_then(|m| m.modified().ok());
        let accessed = file_metadata.as_ref().and_then(|m| m.accessed().ok());
        let created = file_metadata.as_ref().and_then(|m| m.created().ok());

        // Build haystack from directory entry
        if let Some(haystack) = self.haystack_builder.build(dent) {
            log::debug!("Haystack built for file: {:?}", haystack.path());
            // Execute ripgrep search using full stack
            match self.worker.search(&haystack) {
                Ok(_search_result) => {
                    log::debug!("Search executed successfully");
                    // Parse JSON Lines to SearchResult
                    let results_parsed = {
                        // Access JSON buffer from printer (mutable borrow scope)
                        let buffer = self.worker.printer().get_mut();
                        log::debug!("JSON buffer length: {} bytes", buffer.len());
                        if log::log_enabled!(log::Level::Debug) && !buffer.is_empty() {
                            log::debug!("JSON buffer content: {}", String::from_utf8_lossy(buffer));
                        }
                        let parsed = parse_json_buffer(buffer);
                        buffer.clear(); // Clear immediately after parsing
                        parsed
                    }; // Mutable borrow of self.worker released here

                    if let Ok(mut results) = results_parsed {
                        log::debug!("Parsed {} results from JSON", results.len());
                        // Attach file metadata to all results from this file
                        for result in &mut results {
                            result.modified = modified;
                            result.accessed = accessed;
                            result.created = created;
                        }

                        for (i, result) in results.into_iter().enumerate() {
                            // Check cancellation every 100 results to balance responsiveness vs overhead
                            if i % 100 == 0 && *self.cancellation_rx.borrow() {
                                *self.was_incomplete.blocking_write() = true;
                                return ignore::WalkState::Quit;
                            }

                            // Mode-first branching: check output mode BEFORE reservation
                            match self.output_mode {
                                SearchOutputMode::Full => {
                                    // Full mode: Always adds result
                                    // Reserve slot, then add
                                    match self.total_matches.fetch_update(
                                        Ordering::SeqCst,
                                        Ordering::SeqCst,
                                        |current| {
                                            if let Some(max) = self.max_results {
                                                if current < max {
                                                    Some(current + 1)
                                                } else {
                                                    None
                                                }
                                            } else {
                                                Some(current + 1)
                                            }
                                        },
                                    ) {
                                        Ok(_) => {
                                            // Use buffered approach for better performance
                                            self.add_result(result);
                                            self.maybe_update_last_read_time();
                                        }
                                        Err(_) => {
                                            // Limit reached
                                            return ignore::WalkState::Quit;
                                        }
                                    }
                                }

                                SearchOutputMode::FilesOnly => {
                                    // FilesOnly mode: Deduplicate BEFORE reserving
                                    let mut seen = self.seen_files.blocking_write();
                                    if !seen.contains(&result.file) {
                                        // File not seen yet - try to reserve
                                        match self.total_matches.fetch_update(
                                            Ordering::SeqCst,
                                            Ordering::SeqCst,
                                            |current| {
                                                if let Some(max) = self.max_results {
                                                    if current < max {
                                                        Some(current + 1)
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    Some(current + 1)
                                                }
                                            },
                                        ) {
                                            Ok(_) => {
                                                // Reserved successfully - mark as seen
                                                seen.insert(result.file.clone());
                                                drop(seen); // Release lock before next operation

                                                // Add deduplicated result
                                                let file_result = SearchResult {
                                                    file: result.file,
                                                    line: None,
                                                    r#match: None,
                                                    r#type: result.r#type,
                                                    is_context: false,
                                                    is_binary: result.is_binary,
                                                    binary_suppressed: result.binary_suppressed,
                                                    modified: result.modified,
                                                    accessed: result.accessed,
                                                    created: result.created,
                                                };
                                                // Use buffered approach for better performance
                                                self.add_result(file_result);
                                                self.maybe_update_last_read_time();
                                            }
                                            Err(_) => {
                                                // Hit limit - quit immediately
                                                return ignore::WalkState::Quit;
                                            }
                                        }
                                    }
                                    // else: already seen this file, skip entirely
                                }

                                SearchOutputMode::CountPerFile => {
                                    // CountPerFile mode: Use total_files for limiting
                                    // DO NOT touch total_matches during search
                                    // (finalization at line 604 will set total_matches = total_files)

                                    // Check if this is a new file (read-only check)
                                    let is_new_file = {
                                        let counts = self.file_counts.blocking_read();
                                        !counts.contains_key(&result.file)
                                    };

                                    if is_new_file {
                                        // New file - atomically reserve a slot in total_files
                                        match self.total_files.fetch_update(
                                            Ordering::SeqCst,
                                            Ordering::SeqCst,
                                            |current| {
                                                if let Some(max) = self.max_results {
                                                    if current < max {
                                                        Some(current + 1) // Reserve slot for this file
                                                    } else {
                                                        None // Hit limit - reject
                                                    }
                                                } else {
                                                    Some(current + 1) // No limit
                                                }
                                            },
                                        ) {
                                            Ok(_) => {
                                                // Successfully reserved - add to HashMap
                                                let mut counts = self.file_counts.blocking_write();
                                                counts
                                                    .entry(result.file.clone())
                                                    .and_modify(|data| data.count += 1)
                                                    .or_insert(
                                                        super::super::types::FileCountData {
                                                            count: 1,
                                                            modified: result.modified,
                                                            accessed: result.accessed,
                                                            created: result.created,
                                                        },
                                                    );
                                                let elapsed_micros =
                                                    self.start_time.elapsed().as_micros() as u64;
                                                self.last_read_time_atomic
                                                    .store(elapsed_micros, Ordering::Relaxed);
                                            }
                                            Err(_) => {
                                                // Hit file limit - stop search immediately
                                                return ignore::WalkState::Quit;
                                            }
                                        }
                                    } else {
                                        // Existing file - just increment its match count (no limit check needed)
                                        let mut counts = self.file_counts.blocking_write();
                                        if let Some(data) = counts.get_mut(&result.file) {
                                            data.count += 1;
                                            let elapsed_micros =
                                                self.start_time.elapsed().as_micros() as u64;
                                            self.last_read_time_atomic
                                                .store(elapsed_micros, Ordering::Relaxed);
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Err(e) = results_parsed {
                        log::error!(
                            "JSON parsing error for {}: {}",
                            haystack.path().display(),
                            e
                        );
                    }
                }
                Err(e) => {
                    log::warn!("Search error for {}: {}", haystack.path().display(), e);
                }
            }
        }

        ignore::WalkState::Continue
    }
}

impl Drop for ContentSearchVisitor {
    fn drop(&mut self) {
        // CRITICAL: Flush any remaining buffered results
        // Without this, the last batch of results would be lost!
        self.flush_buffer();

        // Ensure final last_read_time update
        self.force_update_last_read_time();
    }
}

/// Fallback visitor used when per-thread initialization fails.
/// Records the error and immediately terminates the search gracefully.
struct ErrorVisitor {
    error_message: String,
    error_count: Arc<AtomicUsize>,
    errors: Arc<RwLock<Vec<SearchError>>>,
    was_incomplete: Arc<RwLock<bool>>,
}

impl ParallelVisitor for ErrorVisitor {
    fn visit(&mut self, _entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        // Only the first thread to encounter this error records it
        // This prevents duplicate error messages from multiple threads
        if self.error_count.fetch_add(1, Ordering::SeqCst) == 0 {
            let mut errors = self.errors.blocking_write();
            errors.push(SearchError {
                path: "<initialization>".to_string(),
                message: self.error_message.clone(),
                error_type: "initialization_error".to_string(),
            });
            *self.was_incomplete.blocking_write() = true;
        }

        // Immediately quit to prevent further thread spawning
        ignore::WalkState::Quit
    }
}

/// Execute content search using grep libraries with parallel directory traversal
pub(super) fn execute(
    options: &super::super::types::SearchSessionOptions,
    root: &std::path::PathBuf,
    ctx: &mut super::context::SearchContext,
) {
    // Build LowArgs from SearchSessionOptions
    use super::super::rg::flags::hiargs::HiArgs;
    use super::super::rg::flags::lowargs::{
        BinaryMode as RgBinaryMode, BoundaryMode as RgBoundaryMode, ContextMode, EncodingMode,
        LowArgs, Mode, PatternSource, SearchMode,
    };
    use super::super::types::{BinaryMode, BoundaryMode};
    use super::utils::{build_type_changes, convert_case_mode};

    let mut context = ContextMode::default();

    // Set -C (context) first
    if options.context > 0 {
        context.set_both(options.context as usize);
    }

    // Override with -B (before_context) if specified
    if let Some(before) = options.before_context {
        context.set_before(before as usize);
    }

    // Override with -A (after_context) if specified
    if let Some(after) = options.after_context {
        context.set_after(after as usize);
    }

    // Convert MCP type params to ripgrep TypeChange format
    let type_changes = build_type_changes(options);

    // Map MCP BinaryMode to ripgrep's internal BinaryMode
    // Matches ripgrep's --binary and -a/--text flags
    let binary_mode = match options.binary_mode {
        BinaryMode::Auto => RgBinaryMode::Auto, // Default: skip binaries
        BinaryMode::Binary => RgBinaryMode::SearchAndSuppress, // --binary: search but suppress
        BinaryMode::Text => RgBinaryMode::AsText, // -a/--text: treat as text
    };

    // Convert encoding string to EncodingMode for rg
    let encoding_mode = match options.encoding.as_deref() {
        None | Some("auto") => EncodingMode::Auto,
        Some("none") => EncodingMode::Disabled,
        Some(enc_str) => match grep::searcher::Encoding::new(enc_str) {
            Ok(enc) => EncodingMode::Some(enc),
            Err(e) => {
                log::warn!("Invalid encoding '{enc_str}': {e}, using auto");
                EncodingMode::Auto
            }
        },
    };

    log::debug!(
        "content_search: case_mode from options = {:?}",
        options.case_mode
    );
    let mut low_args = LowArgs {
        patterns: vec![PatternSource::Regexp(options.pattern.clone())],
        positional: vec![root.as_os_str().to_owned()],
        case: convert_case_mode(options.case_mode),
        boundary: options.boundary_mode.map(|mode| match mode {
            BoundaryMode::Word => RgBoundaryMode::Word,
            BoundaryMode::Line => RgBoundaryMode::Line,
        }),
        fixed_strings: options.literal_search,
        context,
        max_count: if ctx.output_mode == SearchOutputMode::FilesOnly {
            Some(1) // Limit to 1 match per file for FilesOnly mode (optimization)
        } else {
            options.max_results.map(u64::from)
        },
        max_filesize: options.max_filesize,
        max_depth: options.max_depth,
        hidden: options.include_hidden,
        line_number: Some(true),
        invert_match: options.invert_match,
        mode: Mode::Search(SearchMode::Json),
        only_matching: options.only_matching,
        type_changes,
        multiline: options.multiline,
        multiline_dotall: options.multiline,
        binary: binary_mode,
        encoding: encoding_mode,
        engine: convert_engine_choice(options.engine),
        // Match ripgrep's --no-ignore flag behavior exactly
        no_ignore_vcs: options.no_ignore,
        no_ignore_exclude: options.no_ignore,
        no_ignore_global: options.no_ignore,
        no_ignore_parent: options.no_ignore,
        no_ignore_dot: options.no_ignore,
        ..Default::default()
    };

    // Bridge SearchMode to SearchOutputMode for ripgrep CLI compatibility
    // Maps ripgrep's SearchMode enum variants to MCP's SearchOutputMode
    let effective_output_mode = match low_args.mode {
        Mode::Search(SearchMode::Count | SearchMode::CountMatches) => {
            // -c/--count flag
            SearchOutputMode::CountPerFile
        }
        _ => {
            // For Standard and Json modes, use the MCP-provided output mode
            ctx.output_mode
        }
    };

    // Update max_count based on effective output mode
    if effective_output_mode == SearchOutputMode::FilesOnly && low_args.max_count.is_none() {
        // Optimization: stop after first match per file for FilesOnly mode
        low_args.max_count = Some(1);
    }

    // Build HiArgs ONCE (expensive config processing)
    let hi_args = match HiArgs::from_low_args(low_args) {
        Ok(h) => h,
        Err(e) => {
            *ctx.error.blocking_write() = Some(format!("Failed to build HiArgs: {e}"));
            *ctx.is_error.blocking_write() = true;
            ctx.is_complete.store(true, Ordering::Release);
            return;
        }
    };

    // Build matcher ONCE (expensive: regex/PCRE2 compilation)
    let matcher = match hi_args.matcher() {
        Ok(m) => m,
        Err(e) => {
            *ctx.error.blocking_write() = Some(format!("Failed to build matcher: {e}"));
            *ctx.is_error.blocking_write() = true;
            ctx.is_complete.store(true, Ordering::Release);
            return;
        }
    };

    // Wrap HiArgs in Arc for cheap sharing across threads
    let hi_args = Arc::new(hi_args);

    // Build directory walker with gitignore support and parallel traversal
    let mut walker = WalkBuilder::new(root);
    super::utils::configure_walker(&mut walker, &hi_args);

    // Use HiArgs.types() - includes built-in types + file_pattern + type/type_not
    walker.types(hi_args.types().clone());

    // Build the parallel visitor
    let mut builder = ContentSearchBuilder {
        hi_args,
        matcher,
        max_results: options.max_results.map(|m| m as usize),
        output_mode: effective_output_mode,
        results: Arc::clone(&ctx.results),
        total_matches: Arc::clone(&ctx.total_matches),
        total_files: Arc::clone(&ctx.total_files),
        last_read_time_atomic: Arc::clone(&ctx.last_read_time_atomic),
        cancellation_rx: ctx.cancellation_rx.clone(),
        first_result_tx: ctx.first_result_tx.clone(),
        was_incomplete: Arc::clone(&ctx.was_incomplete),
        error_count: Arc::clone(&ctx.error_count),
        errors: Arc::clone(&ctx.errors),
        seen_files: Arc::clone(&ctx.seen_files),
        file_counts: Arc::clone(&ctx.file_counts),
        start_time: ctx.start_time,
    };

    // Execute parallel search
    walker.build_parallel().visit(&mut builder);

    // Finalize CountPerFile mode - convert HashMap counts to SearchResults
    if effective_output_mode == SearchOutputMode::CountPerFile {
        use super::super::types::SearchResultType;

        // Phase 1: Build results Vec while holding only read lock on file_counts
        let results_to_add: Vec<SearchResult> = {
            let counts = ctx.file_counts.blocking_read();
            let mut results = Vec::with_capacity(counts.len());

            for (file, data) in counts.iter() {
                results.push(SearchResult {
                    file: file.clone(),
                    line: Some(data.count as u32), // Count stored in line field
                    r#match: None,
                    r#type: SearchResultType::Content,
                    is_context: false,
                    is_binary: None,
                    binary_suppressed: None,
                    modified: data.modified,
                    accessed: data.accessed,
                    created: data.created,
                });
            }

            results
        }; // Read lock on file_counts released here

        // Phase 2: Swap into results with brief write lock
        {
            let mut results_guard = ctx.results.blocking_write();

            // CountPerFile accumulates in file_counts HashMap, not results vec
            debug_assert!(
                results_guard.is_empty(),
                "CountPerFile: results vec should be empty before finalization"
            );

            *results_guard = results_to_add;

            // Update total_matches to reflect number of unique files with counts
            // In CountPerFile mode, total_matches should reflect unique file count for API consistency
            ctx.total_matches
                .store(ctx.total_files.load(Ordering::SeqCst), Ordering::SeqCst);
        } // Write lock on results released here
    }

    // Log error summary if any errors occurred
    let error_count = ctx.error_count.load(Ordering::SeqCst);
    if error_count > 0 {
        log::info!(
            "Content search completed with {} errors. Pattern: '{}', Path: {}",
            error_count,
            options.pattern,
            root.display()
        );
    }

    // Mark complete
    ctx.is_complete.store(true, Ordering::Release);
}
