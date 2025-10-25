//! Basic search functionality examples
//!
//! This example demonstrates core search operations:
//! - Basic content search
//! - Basic file search  
//! - Single file matching
//! - Filename search
//! - Nested file discovery
//!
//! Run with: cargo run --example `search_basics`
//! Debug: `RUST_LOG=debug` cargo run --example `search_basics`

use anyhow::{Context, Result};
use kodegen_mcp_schema::EngineChoice;
use kodegen_tools_filesystem::search::manager::SearchManager;
use kodegen_tools_filesystem::search::types::{
    BinaryMode, CaseMode, SearchSessionOptions, SearchType,
};
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    println!("=== Search Basics Example ===\n");

    let test_dir = create_test_environment().context("Failed to create test environment")?;

    let config = kodegen_tools_config::ConfigManager::new();
    let manager = SearchManager::new(config);

    test_content_search(&manager, &test_dir).await?;
    test_file_search(&manager, &test_dir).await?;
    test_single_match(&manager, &test_dir).await?;
    test_filename_search(&manager, &test_dir).await?;
    test_nested_files(&manager, &test_dir).await?;

    println!("\n✅ All basic tests passed!");
    Ok(())
}

/// Create comprehensive test file structure
fn create_test_environment() -> Result<TempDir> {
    let dir = TempDir::new().context("Failed to create temp directory")?;
    let base = dir.path();

    // Create test files
    fs::write(
        base.join("file1.txt"),
        "hello world\ntest content\nmore text",
    )
    .context("Failed to create file1.txt")?;

    fs::write(base.join("file2.txt"), "another test file\nno match here")
        .context("Failed to create file2.txt")?;

    fs::write(
        base.join("file3.rs"),
        "fn test() {\n    println!(\"test\");\n}",
    )
    .context("Failed to create file3.rs")?;

    fs::write(base.join("README.md"), "# Test Project\nThis is a test")
        .context("Failed to create README.md")?;

    // Create nested directory and file
    fs::create_dir(base.join("subdir")).context("Failed to create subdir")?;

    fs::write(base.join("subdir/nested.txt"), "nested test content")
        .context("Failed to create nested.txt")?;

    Ok(dir)
}

/// Test 1: Basic content search for "test" pattern
async fn test_content_search(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 1: Basic content search for 'test'");

    let options = SearchSessionOptions {
        root_path: test_dir.path().to_string_lossy().to_string(),
        pattern: "test".to_string(),
        search_type: SearchType::Content,
        file_pattern: None,
        r#type: Vec::new(),
        type_not: Vec::new(),
        case_mode: CaseMode::Sensitive,
        max_results: None,
        include_hidden: false,
        no_ignore: true,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        literal_search: false,
        boundary_mode: None,
        output_mode: kodegen_tools_filesystem::search::types::SearchOutputMode::Full,
        invert_match: false,
        engine: EngineChoice::Auto,
        preprocessor: None,
        preprocessor_globs: Vec::new(),
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
    };

    let response = manager
        .start_search(options)
        .await
        .context("Failed to start content search")?;

    let results = manager
        .get_more_results(&response.session_id, 0, 1000)
        .await
        .context("Failed to get search results")?;

    if results.total_results == 0 {
        anyhow::bail!("Expected to find results for 'test', got 0");
    }

    println!(
        "✓ Found {} results in {} files",
        results.total_results,
        results.results.len()
    );
    println!();
    Ok(())
}

/// Test 2: Basic file search for *.txt pattern
async fn test_file_search(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 2: Basic file search for '*.txt'");

    let options = SearchSessionOptions {
        root_path: test_dir.path().to_string_lossy().to_string(),
        pattern: "*.txt".to_string(),
        search_type: SearchType::Files,
        file_pattern: None,
        r#type: Vec::new(),
        type_not: Vec::new(),
        case_mode: CaseMode::Sensitive,
        max_results: None,
        include_hidden: false,
        no_ignore: true,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        literal_search: false,
        boundary_mode: None,
        output_mode: kodegen_tools_filesystem::search::types::SearchOutputMode::Full,
        invert_match: false,
        engine: EngineChoice::Auto,
        preprocessor: None,
        preprocessor_globs: Vec::new(),
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
    };

    let response = manager
        .start_search(options)
        .await
        .context("Failed to start file search")?;

    let results = manager
        .get_more_results(&response.session_id, 0, 1000)
        .await
        .context("Failed to get file search results")?;

    if results.total_results < 3 {
        anyhow::bail!(
            "Expected at least 3 .txt files, got {}",
            results.total_results
        );
    }

    println!("✓ Found {} .txt files", results.total_results);
    println!();
    Ok(())
}

/// Test 3: Content search for single file match
async fn test_single_match(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 3: Content search for 'hello' (single file)");

    let options = SearchSessionOptions {
        root_path: test_dir.path().to_string_lossy().to_string(),
        pattern: "hello".to_string(),
        search_type: SearchType::Content,
        file_pattern: None,
        r#type: Vec::new(),
        type_not: Vec::new(),
        case_mode: CaseMode::Sensitive,
        max_results: None,
        include_hidden: false,
        no_ignore: true,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        literal_search: false,
        boundary_mode: None,
        output_mode: kodegen_tools_filesystem::search::types::SearchOutputMode::Full,
        invert_match: false,
        engine: EngineChoice::Auto,
        preprocessor: None,
        preprocessor_globs: Vec::new(),
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
    };

    let response = manager
        .start_search(options)
        .await
        .context("Failed to start search for 'hello'")?;

    let results = manager
        .get_more_results(&response.session_id, 0, 1000)
        .await
        .context("Failed to get results")?;

    if results.total_results == 0 {
        anyhow::bail!("Expected to find 'hello', got 0 results");
    }

    // Verify it's from file1.txt
    let has_file1 = results.results.iter().any(|r| r.file.contains("file1.txt"));
    if !has_file1 {
        anyhow::bail!("Expected result from file1.txt");
    }

    println!("✓ Found 'hello' in file1.txt");
    println!();
    Ok(())
}

/// Test 4: Filename search for README*
async fn test_filename_search(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 4: File search for 'README*'");

    let options = SearchSessionOptions {
        root_path: test_dir.path().to_string_lossy().to_string(),
        pattern: "README*".to_string(),
        search_type: SearchType::Files,
        file_pattern: None,
        r#type: Vec::new(),
        type_not: Vec::new(),
        case_mode: CaseMode::Sensitive,
        max_results: None,
        include_hidden: false,
        no_ignore: true,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        literal_search: false,
        boundary_mode: None,
        output_mode: kodegen_tools_filesystem::search::types::SearchOutputMode::Full,
        invert_match: false,
        engine: EngineChoice::Auto,
        preprocessor: None,
        preprocessor_globs: Vec::new(),
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
    };

    let response = manager
        .start_search(options)
        .await
        .context("Failed to start README search")?;

    let results = manager
        .get_more_results(&response.session_id, 0, 1000)
        .await
        .context("Failed to get README search results")?;

    if results.total_results == 0 {
        anyhow::bail!("Expected to find README.md");
    }

    let has_readme = results.results.iter().any(|r| r.file.contains("README.md"));
    if !has_readme {
        anyhow::bail!("Expected README.md in results");
    }

    println!("✓ Found README.md");
    println!();
    Ok(())
}

/// Test 5: Verify nested files are discovered
async fn test_nested_files(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 5: Content search verifying nested file discovery");

    let options = SearchSessionOptions {
        root_path: test_dir.path().to_string_lossy().to_string(),
        pattern: "nested".to_string(),
        search_type: SearchType::Content,
        file_pattern: None,
        r#type: Vec::new(),
        type_not: Vec::new(),
        case_mode: CaseMode::Sensitive,
        max_results: None,
        include_hidden: false,
        no_ignore: true,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        literal_search: false,
        boundary_mode: None,
        output_mode: kodegen_tools_filesystem::search::types::SearchOutputMode::Full,
        invert_match: false,
        engine: EngineChoice::Auto,
        preprocessor: None,
        preprocessor_globs: Vec::new(),
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
    };

    let response = manager
        .start_search(options)
        .await
        .context("Failed to start nested file search")?;

    let results = manager
        .get_more_results(&response.session_id, 0, 1000)
        .await
        .context("Failed to get nested file results")?;

    if results.total_results == 0 {
        anyhow::bail!("Expected to find 'nested' in subdir/nested.txt");
    }

    let has_nested = results
        .results
        .iter()
        .any(|r| r.file.contains("subdir") && r.file.contains("nested.txt"));
    if !has_nested {
        anyhow::bail!("Expected result from subdir/nested.txt");
    }

    println!("✓ Found nested file in subdir/nested.txt");
    println!();
    Ok(())
}
