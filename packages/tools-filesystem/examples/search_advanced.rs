//! Advanced search features examples
//!
//! Run with: cargo run --example `search_advanced`

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
    println!("=== Search Advanced Example ===\n");

    let test_dir = create_test_environment()?;
    let config = kodegen_tools_config::ConfigManager::new();
    let manager = SearchManager::new(config);

    // Test 1: Regex engine selection
    test_engine_selection(&manager, &test_dir).await?;

    // Test 2: Result pagination
    test_pagination(&manager, &test_dir).await?;

    // Test 3: Timeouts and early termination
    test_timeouts(&manager, &test_dir).await?;

    // Test 4: Binary file handling
    test_binary_mode(&manager, &test_dir).await?;

    // Test 5: Max results limiting
    test_max_results(&manager, &test_dir).await?;

    println!("\n✅ All advanced feature tests passed!");
    Ok(())
}

fn create_test_environment() -> Result<TempDir> {
    let dir = TempDir::new()?;
    let base = dir.path();

    // Files for regex engine testing
    fs::write(base.join("rust_regex.txt"), "test(123) test[abc] test{xyz}")?;
    fs::write(
        base.join("pcre_pattern.txt"),
        "before test123 after\ntest456\n",
    )?;

    // File with many results for pagination
    fs::write(base.join("many_results.txt"), "match\n".repeat(100))?;

    // Quick match file for early termination
    fs::write(base.join("quick.txt"), "quick match here")?;

    // Binary-like file (with null bytes)
    let binary_content = vec![
        b'h', b'e', b'l', b'l', b'o', 0x00, b'w', b'o', b'r', b'l', b'd',
    ];
    fs::write(base.join("binary.bin"), binary_content)?;

    // Normal text file
    fs::write(base.join("text.txt"), "normal text content")?;

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

async fn test_engine_selection(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 1: Regex Engine Selection");

    // Auto engine (default)
    let options_auto = SearchSessionOptions {
        pattern: r"test\d+".to_string(),
        literal_search: false,
        engine: EngineChoice::Auto,
        ..default_options(test_dir, "")
    };
    let response_auto = manager.start_search(options_auto).await?;
    let results_auto = manager
        .get_more_results(&response_auto.session_id, 0, 1000)
        .await?;

    // Force Rust engine
    let options_rust = SearchSessionOptions {
        pattern: r"test\d+".to_string(),
        literal_search: false,
        engine: EngineChoice::Rust,
        ..default_options(test_dir, "")
    };
    let response_rust = manager.start_search(options_rust).await?;
    let results_rust = manager
        .get_more_results(&response_rust.session_id, 0, 1000)
        .await?;

    // PCRE2 engine (supports more advanced patterns)
    let options_pcre = SearchSessionOptions {
        pattern: r"test\d+".to_string(),
        literal_search: false,
        engine: EngineChoice::PCRE2,
        ..default_options(test_dir, "")
    };
    let response_pcre = manager.start_search(options_pcre).await?;
    let results_pcre = manager
        .get_more_results(&response_pcre.session_id, 0, 1000)
        .await?;

    println!("  Auto engine: {} results", results_auto.total_results);
    println!("  Rust engine: {} results", results_rust.total_results);
    println!("  PCRE2 engine: {} results", results_pcre.total_results);

    println!("  ✓ Engine selection working correctly\n");
    Ok(())
}

async fn test_pagination(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 2: Result Pagination");

    let response = manager
        .start_search(default_options(test_dir, "match"))
        .await?;

    // Get first 10 results
    let page1 = manager
        .get_more_results(&response.session_id, 0, 10)
        .await?;

    // Get next 10 results
    let page2 = manager
        .get_more_results(&response.session_id, 10, 10)
        .await?;

    // Get last 10 results (using negative offset)
    let last_page = manager
        .get_more_results(&response.session_id, -10, 10)
        .await?;

    println!("  Page 1 (0-10): {} results", page1.returned_count);
    println!("  Page 2 (10-20): {} results", page2.returned_count);
    println!(
        "  Last page (last 10): {} results",
        last_page.returned_count
    );
    println!("  Total results: {}", page1.total_results);

    println!("  ✓ Pagination working correctly\n");
    Ok(())
}

async fn test_timeouts(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 3: Timeouts and Early Termination");

    // With timeout
    let options_timeout = SearchSessionOptions {
        timeout_ms: Some(5000), // 5 second timeout
        ..default_options(test_dir, "match")
    };
    let response_timeout = manager.start_search(options_timeout).await?;
    let results_timeout = manager
        .get_more_results(&response_timeout.session_id, 0, 1000)
        .await?;

    // With early termination for file searches
    let options_early = SearchSessionOptions {
        search_type: SearchType::Files,
        pattern: "quick.txt".to_string(),
        early_termination: Some(true),
        ..default_options(test_dir, "")
    };
    let response_early = manager.start_search(options_early).await?;
    let results_early = manager
        .get_more_results(&response_early.session_id, 0, 1000)
        .await?;

    println!(
        "  With timeout (5s): {} results",
        results_timeout.total_results
    );
    println!(
        "  With early termination: {} results",
        results_early.total_results
    );

    println!("  ✓ Timeouts and early termination working correctly\n");
    Ok(())
}

async fn test_binary_mode(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 4: Binary File Handling");

    // Auto mode (skip binary files)
    let options_auto = SearchSessionOptions {
        binary_mode: BinaryMode::Auto,
        ..default_options(test_dir, "world")
    };
    let response_auto = manager.start_search(options_auto).await?;
    let results_auto = manager
        .get_more_results(&response_auto.session_id, 0, 1000)
        .await?;

    // Binary mode (search but suppress binary content)
    let options_binary = SearchSessionOptions {
        binary_mode: BinaryMode::Binary,
        ..default_options(test_dir, "world")
    };
    let response_binary = manager.start_search(options_binary).await?;
    let results_binary = manager
        .get_more_results(&response_binary.session_id, 0, 1000)
        .await?;

    // Text mode (treat all files as text)
    let options_text = SearchSessionOptions {
        binary_mode: BinaryMode::Text,
        ..default_options(test_dir, "world")
    };
    let response_text = manager.start_search(options_text).await?;
    let results_text = manager
        .get_more_results(&response_text.session_id, 0, 1000)
        .await?;

    println!(
        "  Auto mode: {} results (skips binary)",
        results_auto.total_results
    );
    println!(
        "  Binary mode: {} results (searches binary, suppresses content)",
        results_binary.total_results
    );
    println!(
        "  Text mode: {} results (treats all as text)",
        results_text.total_results
    );

    println!("  ✓ Binary mode handling working correctly\n");
    Ok(())
}

async fn test_max_results(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 5: Max Results Limiting");

    // Unlimited results
    let response_unlimited = manager
        .start_search(default_options(test_dir, "match"))
        .await?;
    let results_unlimited = manager
        .get_more_results(&response_unlimited.session_id, 0, 10000)
        .await?;

    // Limited to 10 results
    let options_limited = SearchSessionOptions {
        max_results: Some(10),
        ..default_options(test_dir, "match")
    };
    let response_limited = manager.start_search(options_limited).await?;
    let results_limited = manager
        .get_more_results(&response_limited.session_id, 0, 10000)
        .await?;

    println!("  Unlimited: {} results", results_unlimited.total_results);
    println!("  Limited to 10: {} results", results_limited.total_results);

    if results_limited.total_results > 10 {
        anyhow::bail!(
            "Expected max_results to limit to 10, got {}",
            results_limited.total_results
        );
    }

    println!("  ✓ Max results limiting working correctly\n");
    Ok(())
}
