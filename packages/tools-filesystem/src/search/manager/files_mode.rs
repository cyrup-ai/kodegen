//!
//! This module provides file listing functionality for `Mode::Files`.
//! Lists all files that would be searched without actually searching.

use super::super::rg::flags::lowargs::CaseMode as RgCaseMode;
use super::super::rg::flags::{
    hiargs::HiArgs,
    lowargs::{LowArgs, Mode, TypeChange},
};
use super::super::types::SearchSessionOptions;
use super::super::types::{SearchError, SearchResult, SearchResultType};
use super::config::{
    DEFAULT_MAX_RESULTS, LAST_READ_UPDATE_INTERVAL_MS, LAST_READ_UPDATE_MATCH_THRESHOLD,
    MAX_DETAILED_ERRORS, RESULT_BUFFER_SIZE,
};
use super::context::SearchContext;

use ignore::{DirEntry, ParallelVisitor, ParallelVisitorBuilder, WalkBuilder};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Parallel visitor builder for files mode
pub(super) struct FilesListerBuilder {
    pub(super) max_results: usize,
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

impl<'s> ParallelVisitorBuilder<'s> for FilesListerBuilder {
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        Box::new(FilesListerVisitor {
            max_results: self.max_results,
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

/// Parallel visitor for files mode
pub(super) struct FilesListerVisitor {
    max_results: usize,
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
    last_update_time: Instant,
    /// Number of matches since last update
    matches_since_update: usize,
    start_time: Instant,
}

impl FilesListerVisitor {
    /// Update `last_read_time` if enough time has passed or enough matches accumulated
    fn maybe_update_last_read_time(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update_time);

        if elapsed.as_millis() >= u128::from(LAST_READ_UPDATE_INTERVAL_MS)
            || self.matches_since_update >= LAST_READ_UPDATE_MATCH_THRESHOLD
        {
            let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
            self.last_read_time_atomic
                .store(elapsed_micros, Ordering::Relaxed);
            self.last_update_time = now;
            self.matches_since_update = 0;
        }
    }

    /// Flush buffered results to shared results
    fn flush_buffer(&mut self) {
        if !self.buffer.is_empty() {
            // Check if this is the first batch of results
            let was_empty = self.results.blocking_read().is_empty();

            let mut results = self.results.blocking_write();
            results.extend(self.buffer.drain(..));
            drop(results); // Release lock before calling maybe_update_last_read_time

            // Signal first result if this was the first batch
            if was_empty {
                let _ = self.first_result_tx.send(true);
            }

            self.maybe_update_last_read_time();
        }
    }

    /// Add a file to the buffer
    fn add_file(&mut self, entry: &DirEntry) {
        let entry_metadata = entry.metadata().ok();
        let modified = entry_metadata.as_ref().and_then(|m| m.modified().ok());
        let accessed = entry_metadata.as_ref().and_then(|m| m.accessed().ok());
        let created = entry_metadata.as_ref().and_then(|m| m.created().ok());

        let result = SearchResult {
            file: entry.path().display().to_string(),
            line: None,
            r#match: None,
            r#type: SearchResultType::FileList,
            is_context: false,
            is_binary: None,
            binary_suppressed: None,
            modified,
            accessed,
            created,
        };

        self.buffer.push(result);
        self.matches_since_update += 1;

        // Flush when buffer is full
        if self.buffer.len() >= RESULT_BUFFER_SIZE {
            self.flush_buffer();
        }
    }
}

impl Drop for FilesListerVisitor {
    fn drop(&mut self) {
        // Flush any remaining buffered results
        // This is CRITICAL - prevents losing the last batch of results
        self.flush_buffer();
        // Ensure final last_read_time update
        let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
        self.last_read_time_atomic
            .store(elapsed_micros, Ordering::Relaxed);
    }
}

impl ParallelVisitor for FilesListerVisitor {
    fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        // Check cancellation
        if *self.cancellation_rx.borrow() {
            self.flush_buffer();
            *self.was_incomplete.blocking_write() = true;
            return ignore::WalkState::Quit;
        }

        // Check max results
        let current_total = self.total_matches.load(Ordering::Relaxed);
        if current_total >= self.max_results {
            self.flush_buffer();
            *self.was_incomplete.blocking_write() = true;
            return ignore::WalkState::Quit;
        }

        match entry {
            Ok(entry) => {
                // Only process files, not directories
                if let Some(file_type) = entry.file_type()
                    && file_type.is_file()
                {
                    self.add_file(&entry);
                    self.total_matches.fetch_add(1, Ordering::Relaxed);
                }
                ignore::WalkState::Continue
            }
            Err(err) => {
                // Record error
                self.error_count.fetch_add(1, Ordering::Relaxed);

                let mut errors = self.errors.blocking_write();
                if errors.len() < MAX_DETAILED_ERRORS {
                    errors.push(SearchError {
                        path: "unknown".to_string(),
                        message: err.to_string(),
                        error_type: "walk_error".to_string(),
                    });
                }
                drop(errors);

                ignore::WalkState::Continue
            }
        }
    }
}

/// Execute files mode: list all files that would be searched
pub(super) fn execute(options: &SearchSessionOptions, root: &Path, ctx: &mut SearchContext) {
    let max_results = options.max_results.unwrap_or(DEFAULT_MAX_RESULTS as u32) as usize;

    // Build type changes for filtering
    let mut type_changes = Vec::new();
    for type_name in &options.r#type {
        type_changes.push(TypeChange::Select {
            name: type_name.clone(),
        });
    }
    for type_name in &options.type_not {
        type_changes.push(TypeChange::Negate {
            name: type_name.clone(),
        });
    }

    // Build LowArgs for type filtering
    // Note: We use a dummy pattern since we're not actually searching
    let low_args = LowArgs {
        patterns: vec![],
        case: RgCaseMode::Sensitive,
        fixed_strings: false,
        hidden: options.include_hidden,
        invert_match: false,
        mode: Mode::Files,
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
    super::utils::configure_walker(&mut walker, &hi_args);

    // Use HiArgs.types() - handles built-in types
    walker.types(hi_args.types().clone());

    // Build the parallel visitor
    let mut builder = FilesListerBuilder {
        max_results,
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

    // Execute parallel walk
    walker.build_parallel().visit(&mut builder);

    // Log error summary if any errors occurred
    let error_count = ctx.error_count.load(Ordering::SeqCst);
    if error_count > 0 {
        log::info!(
            "Files mode completed with {} errors. Path: {}",
            error_count,
            root.display()
        );
    }

    // Mark complete
    ctx.is_complete.store(true, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::super::super::types::{CaseMode, SearchType};
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tempfile::TempDir;

    #[test]
    fn test_files_mode_basic() {
        // Create temp directory with some files
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Create test files
        fs::write(temp_path.join("file1.txt"), "content1").expect("Failed to write file1");
        fs::write(temp_path.join("file2.rs"), "content2").expect("Failed to write file2");

        // Create subdirectory with file
        fs::create_dir(temp_path.join("subdir")).expect("Failed to create subdir");
        fs::write(temp_path.join("subdir/file3.md"), "content3").expect("Failed to write file3");

        // Create options for files mode
        let options = SearchSessionOptions {
            root_path: temp_path.to_string_lossy().to_string(),
            pattern: String::new(), // Pattern ignored in files mode
            search_type: SearchType::Files,
            file_pattern: None,
            r#type: vec![],
            type_not: vec![],
            case_mode: CaseMode::Sensitive,
            max_results: Some(100),
            include_hidden: false,
            no_ignore: false,
            context: 0,
            before_context: None,
            after_context: None,
            timeout_ms: None,
            early_termination: None,
            literal_search: false,
            boundary_mode: None,
            output_mode: super::super::super::types::SearchOutputMode::Full,
            invert_match: false,
            engine: super::super::super::types::EngineChoice::Auto,
            preprocessor: None,
            preprocessor_globs: vec![],
            search_zip: false,
            binary_mode: super::super::super::types::BinaryMode::Auto,
            multiline: false,
            max_filesize: None,
            max_depth: None,
            only_matching: false,
            list_files_only: true,
            sort_by: None,
            sort_direction: None,
            encoding: None,
        };

        // Create context
        let (_tx, rx) = watch::channel(false);
        let (first_result_tx, _first_result_rx) = watch::channel(false);
        let start_time = Instant::now();
        let mut ctx = SearchContext {
            results: Arc::new(RwLock::new(Vec::new())),
            is_complete: Arc::new(AtomicBool::new(false)),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            last_read_time_atomic: Arc::new(AtomicU64::new(0)),
            cancellation_rx: rx,
            first_result_tx,
            was_incomplete: Arc::new(RwLock::new(false)),
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            is_error: Arc::new(RwLock::new(false)),
            error: Arc::new(RwLock::new(None)),
            output_mode: super::super::super::types::SearchOutputMode::Full,
            seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
            file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            start_time,
        };

        // Execute files mode
        execute(&options, temp_path, &mut ctx);

        // Verify results
        let results = ctx.results.blocking_read();
        assert_eq!(results.len(), 3, "Should find 3 files");

        // Verify all results are FileList type
        for result in results.iter() {
            assert!(matches!(result.r#type, SearchResultType::FileList));
            assert!(result.line.is_none());
            assert!(result.r#match.is_none());
        }

        // Verify completion
        assert!(ctx.is_complete.load(Ordering::Acquire));
    }

    #[test]
    fn test_files_mode_respects_gitignore() {
        // Create temp directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Create .git directory to make it a git repository (required for gitignore)
        fs::create_dir(temp_path.join(".git")).expect("Failed to create .git dir");

        // Create test files
        fs::write(temp_path.join("tracked.rs"), "content").expect("Failed to write tracked file");
        fs::write(temp_path.join("ignored.log"), "content").expect("Failed to write ignored file");

        // Create .gitignore
        fs::write(temp_path.join(".gitignore"), "*.log\n").expect("Failed to write .gitignore");

        // Create options
        let options = SearchSessionOptions {
            root_path: temp_path.to_string_lossy().to_string(),
            pattern: String::new(),
            search_type: SearchType::Files,
            file_pattern: None,
            r#type: vec![],
            type_not: vec![],
            case_mode: CaseMode::Sensitive,
            max_results: Some(100),
            include_hidden: false,
            no_ignore: false,
            context: 0,
            before_context: None,
            after_context: None,
            timeout_ms: None,
            early_termination: None,
            literal_search: false,
            boundary_mode: None,
            output_mode: super::super::super::types::SearchOutputMode::Full,
            invert_match: false,
            engine: super::super::super::types::EngineChoice::Auto,
            preprocessor: None,
            preprocessor_globs: vec![],
            search_zip: false,
            binary_mode: super::super::super::types::BinaryMode::Auto,
            multiline: false,
            max_filesize: None,
            max_depth: None,
            only_matching: false,
            list_files_only: true,
            sort_by: None,
            sort_direction: None,
            encoding: None,
        };

        // Create context
        let (_tx, rx) = watch::channel(false);
        let (first_result_tx, _first_result_rx) = watch::channel(false);
        let start_time = Instant::now();
        let mut ctx = SearchContext {
            results: Arc::new(RwLock::new(Vec::new())),
            is_complete: Arc::new(AtomicBool::new(false)),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            last_read_time_atomic: Arc::new(AtomicU64::new(0)),
            cancellation_rx: rx,
            first_result_tx,
            was_incomplete: Arc::new(RwLock::new(false)),
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            is_error: Arc::new(RwLock::new(false)),
            error: Arc::new(RwLock::new(None)),
            output_mode: super::super::super::types::SearchOutputMode::Full,
            seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
            file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            start_time,
        };

        // Execute
        execute(&options, temp_path, &mut ctx);

        // Verify results - should only find tracked.rs, not ignored.log
        let results = ctx.results.blocking_read();
        assert_eq!(
            results.len(),
            1,
            "Should find only 1 file (ignored.log should be excluded)"
        );
        assert!(results[0].file.contains("tracked.rs"));
    }

    #[test]
    fn test_files_mode_type_filter() {
        // Create temp directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Create files of different types
        fs::write(temp_path.join("code.rs"), "fn main() {}").expect("Failed to write rs file");
        fs::write(temp_path.join("doc.md"), "# Title").expect("Failed to write md file");
        fs::write(temp_path.join("data.json"), "{}").expect("Failed to write json file");

        // Create options with type filter for rust files only
        let options = SearchSessionOptions {
            root_path: temp_path.to_string_lossy().to_string(),
            pattern: String::new(),
            search_type: SearchType::Files,
            file_pattern: None,
            r#type: vec!["rust".to_string()],
            type_not: vec![],
            case_mode: CaseMode::Sensitive,
            max_results: Some(100),
            include_hidden: false,
            no_ignore: false,
            context: 0,
            before_context: None,
            after_context: None,
            timeout_ms: None,
            early_termination: None,
            literal_search: false,
            boundary_mode: None,
            output_mode: super::super::super::types::SearchOutputMode::Full,
            invert_match: false,
            engine: super::super::super::types::EngineChoice::Auto,
            preprocessor: None,
            preprocessor_globs: vec![],
            search_zip: false,
            binary_mode: super::super::super::types::BinaryMode::Auto,
            multiline: false,
            max_filesize: None,
            max_depth: None,
            only_matching: false,
            list_files_only: true,
            sort_by: None,
            sort_direction: None,
            encoding: None,
        };

        // Create context
        let (_tx, rx) = watch::channel(false);
        let (first_result_tx, _first_result_rx) = watch::channel(false);
        let start_time = Instant::now();
        let mut ctx = SearchContext {
            results: Arc::new(RwLock::new(Vec::new())),
            is_complete: Arc::new(AtomicBool::new(false)),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            last_read_time_atomic: Arc::new(AtomicU64::new(0)),
            cancellation_rx: rx,
            first_result_tx,
            was_incomplete: Arc::new(RwLock::new(false)),
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            is_error: Arc::new(RwLock::new(false)),
            error: Arc::new(RwLock::new(None)),
            output_mode: super::super::super::types::SearchOutputMode::Full,
            seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
            file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            start_time,
        };

        // Execute
        execute(&options, temp_path, &mut ctx);

        // Verify results - should only find .rs file
        let results = ctx.results.blocking_read();
        assert_eq!(results.len(), 1, "Should find only 1 rust file");
        assert!(results[0].file.contains("code.rs"));
    }

    #[test]
    fn test_files_mode_max_depth() {
        // Create temp directory with nested structure
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Create files at different depths
        fs::write(temp_path.join("root.txt"), "root").expect("Failed to write root file");

        fs::create_dir(temp_path.join("level1")).expect("Failed to create level1");
        fs::write(temp_path.join("level1/file1.txt"), "l1").expect("Failed to write level1 file");

        fs::create_dir(temp_path.join("level1/level2")).expect("Failed to create level2");
        fs::write(temp_path.join("level1/level2/file2.txt"), "l2")
            .expect("Failed to write level2 file");

        fs::create_dir(temp_path.join("level1/level2/level3")).expect("Failed to create level3");
        fs::write(temp_path.join("level1/level2/level3/file3.txt"), "l3")
            .expect("Failed to write level3 file");

        // Create options with max_depth = 2
        let options = SearchSessionOptions {
            root_path: temp_path.to_string_lossy().to_string(),
            pattern: String::new(),
            search_type: SearchType::Files,
            file_pattern: None,
            r#type: vec![],
            type_not: vec![],
            case_mode: CaseMode::Sensitive,
            max_results: Some(100),
            include_hidden: false,
            no_ignore: false,
            context: 0,
            before_context: None,
            after_context: None,
            timeout_ms: None,
            early_termination: None,
            literal_search: false,
            boundary_mode: None,
            output_mode: super::super::super::types::SearchOutputMode::Full,
            invert_match: false,
            engine: super::super::super::types::EngineChoice::Auto,
            preprocessor: None,
            preprocessor_globs: vec![],
            search_zip: false,
            binary_mode: super::super::super::types::BinaryMode::Auto,
            multiline: false,
            max_filesize: None,
            max_depth: Some(2), // Limit to 2 levels deep
            only_matching: false,
            list_files_only: true,
            sort_by: None,
            sort_direction: None,
            encoding: None,
        };

        // Create context
        let (_tx, rx) = watch::channel(false);
        let (first_result_tx, _first_result_rx) = watch::channel(false);
        let start_time = Instant::now();
        let mut ctx = SearchContext {
            results: Arc::new(RwLock::new(Vec::new())),
            is_complete: Arc::new(AtomicBool::new(false)),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            last_read_time_atomic: Arc::new(AtomicU64::new(0)),
            cancellation_rx: rx,
            first_result_tx,
            was_incomplete: Arc::new(RwLock::new(false)),
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            is_error: Arc::new(RwLock::new(false)),
            error: Arc::new(RwLock::new(None)),
            output_mode: super::super::super::types::SearchOutputMode::Full,
            seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
            file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            start_time,
        };

        // Execute
        execute(&options, temp_path, &mut ctx);

        // Verify results - max_depth=2 in ignore crate means:
        // depth 0 (root) + depth 1 (level1) = 2 levels
        // So we get: root.txt + level1/file1.txt = 2 files
        // level1/level2 (depth 2) is excluded
        let results = ctx.results.blocking_read();
        assert!(
            results.len() >= 2,
            "Should find at least 2 files (root and level1)"
        );
        assert!(
            results.len() <= 3,
            "Should not find more than 3 files (level3 should be excluded)"
        );

        // Verify file3.txt at level3 is NOT included
        let has_level3 = results.iter().any(|r| r.file.contains("level3"));
        assert!(!has_level3, "Should not include files beyond max_depth");
    }
}
