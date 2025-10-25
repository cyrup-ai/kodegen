//! Output modes and formatting examples
//!
//! Run with: cargo run --example `search_output`

use anyhow::Result;
use kodegen_mcp_schema::EngineChoice;
use kodegen_tools_filesystem::search::manager::SearchManager;
use kodegen_tools_filesystem::search::types::{
    BinaryMode, CaseMode, SearchOutputMode, SearchSessionOptions, SearchType, SortBy,
    SortDirection,
};
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    println!("=== Search Output Example ===\n");

    let test_dir = create_test_environment()?;
    let config = kodegen_tools_config::ConfigManager::new();
    let manager = SearchManager::new(config);

    // Test 1: Output modes (Full, FilesOnly, CountPerFile)
    test_output_modes(&manager, &test_dir).await?;

    // Test 2: Context lines (before/after)
    test_context_lines(&manager, &test_dir).await?;

    // Test 3: Sorting results
    test_sorting(&manager, &test_dir).await?;

    // Test 4: Only matching portions
    test_only_matching(&manager, &test_dir).await?;

    println!("\n✅ All output formatting tests passed!");
    Ok(())
}

fn create_test_environment() -> Result<TempDir> {
    let dir = TempDir::new()?;
    let base = dir.path();

    // File for context testing
    fs::write(
        base.join("context.txt"),
        "line1\nline2 match here\nline3\nline4 match here\nline5\n",
    )?;

    // File with multiple matches
    fs::write(
        base.join("multi_match.txt"),
        "first match\nsome text\nsecond match\nmore text\nthird match\n",
    )?;

    // Files for sorting tests (with different timestamps if possible)
    fs::write(base.join("file_a.txt"), "content with match")?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(base.join("file_b.txt"), "content with match")?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(base.join("file_z.txt"), "content with match")?;

    // File for only_matching test
    fs::write(
        base.join("partial.txt"),
        "prefix MATCH suffix\nanother MATCH line\n",
    )?;

    Ok(dir)
}

/// Helper function to create default search options
fn default_options(test_dir: &TempDir, pattern: &str) -> SearchSessionOptions {
    SearchSessionOptions {
        root_path: test_dir.path().to_string_lossy().to_string(),
        pattern: pattern.to_string(),
        search_type: SearchType::Content,
        literal_search: true,
        no_ignore: true,
        case_mode: CaseMode::Sensitive,
        boundary_mode: None,
        output_mode: SearchOutputMode::Full,
        invert_match: false,
        engine: EngineChoice::Auto,
        file_pattern: None,
        r#type: vec![],
        type_not: vec![],
        max_results: None,
        include_hidden: false,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        preprocessor: None,
        preprocessor_globs: vec![],
        search_zip: false,
        binary_mode: BinaryMode::Auto,
        multiline: false,
        max_filesize: None,
        max_depth: None,
        only_matching: false,
        list_files_only: false,
        sort_by: None,
        sort_direction: None,
        encoding: None,
    }
}

/// Helper to run a search and return total results
async fn run_search(manager: &SearchManager, options: SearchSessionOptions) -> Result<usize> {
    let response = manager.start_search(options).await?;

    // Wait for search to complete before reading results
    loop {
        let results = manager
            .get_more_results(&response.session_id, 0, 1000)
            .await?;
        if results.is_complete {
            return Ok(results.total_results);
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

async fn test_output_modes(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 1: Output Modes");

    // Full mode (default) - shows all match details
    let count_full = run_search(manager, default_options(test_dir, "match")).await?;

    // FilesOnly mode - only unique file paths
    let options_files_only = SearchSessionOptions {
        output_mode: SearchOutputMode::FilesOnly,
        ..default_options(test_dir, "match")
    };
    let count_files_only = run_search(manager, options_files_only).await?;

    // CountPerFile mode - file paths with match counts
    let options_count_per_file = SearchSessionOptions {
        output_mode: SearchOutputMode::CountPerFile,
        ..default_options(test_dir, "match")
    };
    let count_count_per_file = run_search(manager, options_count_per_file).await?;

    println!("  Full mode: {count_full} results (all match details)");
    println!("  FilesOnly mode: {count_files_only} results (unique file paths)");
    println!("  CountPerFile mode: {count_count_per_file} results (files with counts)");

    if count_files_only >= count_full {
        anyhow::bail!("Expected FilesOnly to return fewer results than Full mode");
    }

    println!("  ✓ Output modes working correctly\n");
    Ok(())
}

async fn test_context_lines(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 2: Context Lines");

    // No context
    let count_no_context = run_search(manager, default_options(test_dir, "match")).await?;

    // With 1 line of context (before and after)
    let options_context = SearchSessionOptions {
        context: 1,
        ..default_options(test_dir, "match")
    };
    let count_context = run_search(manager, options_context).await?;

    // Separate before/after context
    let options_separate = SearchSessionOptions {
        before_context: Some(2),
        after_context: Some(1),
        ..default_options(test_dir, "match")
    };
    let count_separate = run_search(manager, options_separate).await?;

    println!("  No context: {count_no_context} results");
    println!("  Context 1: {count_context} results (includes context lines)");
    println!("  Before 2, After 1: {count_separate} results");

    if count_context <= count_no_context {
        anyhow::bail!("Expected context mode to include more lines");
    }

    println!("  ✓ Context lines working correctly\n");
    Ok(())
}

async fn test_sorting(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 3: Sorting Results");

    // No sorting (filesystem order)
    let count_no_sort = run_search(manager, default_options(test_dir, "match")).await?;

    // Sort by path ascending
    let options_path_asc = SearchSessionOptions {
        sort_by: Some(SortBy::Path),
        sort_direction: Some(SortDirection::Ascending),
        ..default_options(test_dir, "match")
    };
    let count_path_asc = run_search(manager, options_path_asc).await?;

    // Sort by modified time descending (newest first)
    let options_modified = SearchSessionOptions {
        sort_by: Some(SortBy::Modified),
        sort_direction: Some(SortDirection::Descending),
        ..default_options(test_dir, "match")
    };
    let count_modified = run_search(manager, options_modified).await?;

    println!("  No sorting: {count_no_sort} results (filesystem order)");
    println!("  Sorted by path (A-Z): {count_path_asc} results");
    println!("  Sorted by modified (newest first): {count_modified} results");

    // All should have same count, just different order
    if count_no_sort != count_path_asc || count_no_sort != count_modified {
        anyhow::bail!("Sorting should not change result count");
    }

    println!("  ✓ Sorting working correctly\n");
    Ok(())
}

async fn test_only_matching(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 4: Only Matching Portions");

    // Full line mode (default)
    let count_full_line = run_search(manager, default_options(test_dir, "MATCH")).await?;

    // Only matching portion
    let options_only_match = SearchSessionOptions {
        only_matching: true,
        ..default_options(test_dir, "MATCH")
    };
    let count_only_match = run_search(manager, options_only_match).await?;

    println!("  Full line mode: {count_full_line} results (includes prefix/suffix)");
    println!("  Only matching: {count_only_match} results (just 'MATCH' text)");

    // Should have same count, different content format
    if count_full_line != count_only_match {
        anyhow::bail!("Only matching should not change result count");
    }

    println!("  ✓ Only matching working correctly\n");
    Ok(())
}
