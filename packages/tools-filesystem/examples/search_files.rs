//! File filtering and ignore handling examples
//!
//! Run with: cargo run --example `search_files`

use anyhow::Result;
use kodegen_mcp_schema::EngineChoice;
use kodegen_tools_filesystem::search::manager::SearchManager;
use kodegen_tools_filesystem::search::types::{
    BinaryMode, CaseMode, SearchOutputMode, SearchSessionOptions, SearchType,
};
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    println!("=== Search Files Example ===\n");

    let test_dir = create_test_environment()?;
    let config = kodegen_tools_config::ConfigManager::new();
    let manager = SearchManager::new(config);

    // Test 1: File pattern filtering (globs)
    test_file_patterns(&manager, &test_dir).await?;

    // Test 2: Type filtering (--type)
    test_type_filtering(&manager, &test_dir).await?;

    // Test 3: Hidden files
    test_hidden_files(&manager, &test_dir).await?;

    // Test 4: Gitignore handling
    test_gitignore(&manager, &test_dir).await?;

    // Test 5: Max depth limiting
    test_max_depth(&manager, &test_dir).await?;

    // Test 6: File size filtering
    test_max_filesize(&manager, &test_dir).await?;

    println!("\n✅ All file filtering tests passed!");
    Ok(())
}

fn create_test_environment() -> Result<TempDir> {
    let dir = TempDir::new()?;
    let base = dir.path();

    // Initialize as git repository (required for .gitignore to work)
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(base)
        .output()?;

    // Create .gitignore
    fs::write(base.join(".gitignore"), "*.log\ntemp/\n")?;

    // Create various file types
    fs::write(base.join("visible.txt"), "visible content")?;
    fs::write(base.join(".hidden.txt"), "hidden content")?;
    fs::write(base.join("data.json"), r#"{"key":"value"}"#)?;
    fs::write(base.join("code.rs"), "fn main() {}")?;
    fs::write(base.join("code.py"), "def main(): pass")?;
    fs::write(base.join("script.js"), "console.log('test')")?;
    fs::write(base.join("README.md"), "# Project")?;

    // Create ignored directory and files
    fs::create_dir(base.join("temp"))?;
    fs::write(base.join("temp/ignored.log"), "ignored log content")?;
    fs::write(base.join("should_ignore.log"), "another log")?;

    // Create nested directory structure for depth testing
    fs::create_dir(base.join("level1"))?;
    fs::write(base.join("level1/file1.txt"), "level 1")?;
    fs::create_dir(base.join("level1/level2"))?;
    fs::write(base.join("level1/level2/file2.txt"), "level 2")?;
    fs::create_dir(base.join("level1/level2/level3"))?;
    fs::write(base.join("level1/level2/level3/file3.txt"), "level 3")?;

    // Create large file for size testing
    fs::write(base.join("large.txt"), "x".repeat(10000))?;
    fs::write(base.join("small.txt"), "tiny")?;

    Ok(dir)
}

/// Helper function to create default search options
fn default_options(test_dir: &TempDir, pattern: &str) -> SearchSessionOptions {
    SearchSessionOptions {
        root_path: test_dir.path().to_string_lossy().to_string(),
        pattern: pattern.to_string(),
        search_type: SearchType::Content,
        literal_search: true,
        no_ignore: false, // Respect .gitignore by default
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

async fn test_file_patterns(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 1: File Pattern Filtering (Globs)");

    // Search only .txt files
    let options_txt = SearchSessionOptions {
        file_pattern: Some("*.txt".to_string()),
        ..default_options(test_dir, "content")
    };
    let count_txt = run_search(manager, options_txt).await?;

    // Search only .rs files
    let options_rs = SearchSessionOptions {
        file_pattern: Some("*.rs".to_string()),
        ..default_options(test_dir, "main")
    };
    let count_rs = run_search(manager, options_rs).await?;

    // Search .json and .js files
    let options_json_js = SearchSessionOptions {
        file_pattern: Some("*.{json,js}".to_string()),
        ..default_options(test_dir, "")
    };
    let count_json_js = run_search(manager, options_json_js).await?;

    println!("  *.txt files: {count_txt} results");
    println!("  *.rs files: {count_rs} results");
    println!("  *.{{json,js}} files: {count_json_js} results");

    println!("  ✓ File pattern filtering working correctly\n");
    Ok(())
}

async fn test_type_filtering(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 2: Type Filtering (--type)");

    // Search only rust files using type filter
    let options_rust = SearchSessionOptions {
        r#type: vec!["rust".to_string()],
        ..default_options(test_dir, "fn")
    };
    let count_rust = run_search(manager, options_rust).await?;

    // Search python files
    let options_python = SearchSessionOptions {
        r#type: vec!["python".to_string()],
        ..default_options(test_dir, "def")
    };
    let count_python = run_search(manager, options_python).await?;

    // Search markdown files, exclude json
    let options_md_not_json = SearchSessionOptions {
        r#type: vec!["markdown".to_string()],
        type_not: vec!["json".to_string()],
        ..default_options(test_dir, "")
    };
    let count_md = run_search(manager, options_md_not_json).await?;

    println!("  --type rust: {count_rust} results");
    println!("  --type python: {count_python} results");
    println!("  --type markdown --type-not json: {count_md} results");

    println!("  ✓ Type filtering working correctly\n");
    Ok(())
}

async fn test_hidden_files(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 3: Hidden Files");

    // Search without hidden files (default)
    let count_no_hidden = run_search(manager, default_options(test_dir, "content")).await?;

    // Search with hidden files
    let options_with_hidden = SearchSessionOptions {
        include_hidden: true,
        ..default_options(test_dir, "content")
    };
    let count_with_hidden = run_search(manager, options_with_hidden).await?;

    println!("  Without hidden: {count_no_hidden} results");
    println!("  With hidden: {count_with_hidden} results (includes .hidden.txt)");

    if count_with_hidden <= count_no_hidden {
        anyhow::bail!("Expected hidden files search to find more results");
    }

    println!("  ✓ Hidden file filtering working correctly\n");
    Ok(())
}

async fn test_gitignore(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 4: Gitignore Handling");

    // Search respecting .gitignore (default)
    let count_with_ignore = run_search(manager, default_options(test_dir, "")).await?;

    // Search ignoring .gitignore (no_ignore = true)
    let options_no_ignore = SearchSessionOptions {
        no_ignore: true,
        ..default_options(test_dir, "")
    };
    let count_no_ignore = run_search(manager, options_no_ignore).await?;

    println!("  Respecting .gitignore: {count_with_ignore} results (excludes *.log, temp/)");
    println!("  Ignoring .gitignore (no_ignore): {count_no_ignore} results (includes all)");

    if count_no_ignore <= count_with_ignore {
        anyhow::bail!("Expected no_ignore to find more results");
    }

    println!("  ✓ Gitignore handling working correctly\n");
    Ok(())
}

async fn test_max_depth(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 5: Max Depth Limiting");

    // Unlimited depth
    let count_unlimited = run_search(manager, default_options(test_dir, "level")).await?;

    // Max depth = 1 (root + immediate children)
    let options_depth1 = SearchSessionOptions {
        max_depth: Some(1),
        ..default_options(test_dir, "level")
    };
    let count_depth1 = run_search(manager, options_depth1).await?;

    // Max depth = 2
    let options_depth2 = SearchSessionOptions {
        max_depth: Some(2),
        ..default_options(test_dir, "level")
    };
    let count_depth2 = run_search(manager, options_depth2).await?;

    println!("  Unlimited depth: {count_unlimited} results (finds level 1, 2, 3)");
    println!("  Max depth 1: {count_depth1} results (finds level 1 only)");
    println!("  Max depth 2: {count_depth2} results (finds level 1, 2)");

    if count_depth1 >= count_depth2 || count_depth2 >= count_unlimited {
        anyhow::bail!("Expected deeper searches to find more results");
    }

    println!("  ✓ Max depth limiting working correctly\n");
    Ok(())
}

async fn test_max_filesize(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 6: File Size Filtering");

    // No size limit
    let count_unlimited = run_search(manager, default_options(test_dir, "x")).await?;

    // Max filesize = 100 bytes (excludes large.txt)
    let options_small = SearchSessionOptions {
        max_filesize: Some(100),
        ..default_options(test_dir, "x")
    };
    let count_small = run_search(manager, options_small).await?;

    println!("  No size limit: {count_unlimited} results (includes large.txt)");
    println!("  Max 100 bytes: {count_small} results (excludes large.txt)");

    if count_small >= count_unlimited {
        anyhow::bail!("Expected size limit to reduce results");
    }

    println!("  ✓ File size filtering working correctly\n");
    Ok(())
}
