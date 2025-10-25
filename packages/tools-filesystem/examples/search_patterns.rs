//! Pattern matching examples - demonstrates all pattern modes
//!
//! Run with: cargo run --example `search_patterns`

use anyhow::Result;
use kodegen_mcp_schema::EngineChoice;
use kodegen_tools_filesystem::search::manager::SearchManager;
use kodegen_tools_filesystem::search::types::{
    BinaryMode, BoundaryMode, CaseMode, SearchOutputMode, SearchSessionOptions,
    SearchType,
};
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    println!("=== Search Patterns Example ===\n");

    let test_dir = create_test_environment()?;
    let config = kodegen_tools_config::ConfigManager::new();
    let manager = SearchManager::new(config);

    // Test 1: Literal vs Regex Search
    test_literal_vs_regex(&manager, &test_dir).await?;

    // Test 2: Case Sensitivity Modes
    test_case_modes(&manager, &test_dir).await?;

    // Test 3: Word Boundary Mode
    test_word_boundary(&manager, &test_dir).await?;

    // Test 4: Line Boundary Mode
    test_line_boundary(&manager, &test_dir).await?;

    // Test 5: Multiline Pattern Matching
    test_multiline(&manager, &test_dir).await?;

    // Test 6: Inverted Match
    test_inverted_match(&manager, &test_dir).await?;

    // Test 7: Regex Special Characters
    test_regex_special_chars(&manager, &test_dir).await?;

    println!("\n✅ All pattern matching tests passed!");
    Ok(())
}

fn create_test_environment() -> Result<TempDir> {
    let dir = TempDir::new()?;
    let base = dir.path();

    // File with various case combinations
    fs::write(
        base.join("patterns.txt"),
        "Test TEST test\ntesting tested tester\ncontest protest\nword.test.word\ntest\nTESTING METHODS\nContest Winner\nRETEST PASSED\n",
    )?;

    // File for multiline testing
    fs::write(
        base.join("multiline.txt"),
        "line1\nline2\nline3\nmulti\nline\npattern\nhere\n",
    )?;

    // File with special regex characters
    fs::write(
        base.join("special.txt"),
        "test.log\ntest*log\ntest[log]\n(test)\ntest+log\n",
    )?;

    // Code file for word boundary testing
    fs::write(
        base.join("code.rs"),
        "fn test_func() {}\nfn testing() {}\nlet test = 42;\ncontest_winner\n",
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

async fn test_literal_vs_regex(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 1: Literal vs Regex Search");

    // Regex search for "test.log" (dot matches any character)
    let options_regex = SearchSessionOptions {
        literal_search: false,
        ..default_options(test_dir, "test.log")
    };
    let count_regex = run_search(manager, options_regex).await?;

    // Literal search for "test.log" (dot is literal dot)
    let count_literal = run_search(manager, default_options(test_dir, "test.log")).await?;

    println!("  Regex mode found {count_regex} results (matches test.log, test*log, etc.)");
    println!("  Literal mode found {count_literal} results (only test.log)");

    if count_literal == 0 {
        anyhow::bail!("Expected literal search to find 'test.log', got 0 results");
    }

    println!("  ✓ Literal vs regex working correctly\n");
    Ok(())
}

async fn test_case_modes(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 2: Case Sensitivity Modes");

    // Case-sensitive search (only lowercase "test")
    let count_sensitive = run_search(manager, default_options(test_dir, "test")).await?;

    // Case-insensitive search (Test, TEST, test)
    let options_insensitive = SearchSessionOptions {
        case_mode: CaseMode::Insensitive,
        ..default_options(test_dir, "test")
    };
    let count_insensitive = run_search(manager, options_insensitive).await?;

    // Smart case with lowercase pattern (acts as insensitive)
    let options_smart_lower = SearchSessionOptions {
        case_mode: CaseMode::Smart,
        ..default_options(test_dir, "test")
    };
    let count_smart_lower = run_search(manager, options_smart_lower).await?;

    // Smart case with uppercase pattern (acts as sensitive)
    let options_smart_upper = SearchSessionOptions {
        case_mode: CaseMode::Smart,
        ..default_options(test_dir, "TEST")
    };
    let count_smart_upper = run_search(manager, options_smart_upper).await?;

    println!("  Sensitive: {count_sensitive} results (only 'test')");
    println!("  Insensitive: {count_insensitive} results (Test, TEST, test)");
    println!("  Smart (lowercase): {count_smart_lower} results (like insensitive)");
    println!("  Smart (uppercase): {count_smart_upper} results (like sensitive, only TEST)");

    if count_insensitive <= count_sensitive {
        anyhow::bail!("Expected insensitive to find more results than sensitive");
    }

    println!("  ✓ Case modes working correctly\n");
    Ok(())
}

async fn test_word_boundary(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 3: Word Boundary Mode");

    // Without word boundary (matches testing, contest, etc.)
    let count_no_boundary = run_search(manager, default_options(test_dir, "test")).await?;

    // With word boundary (only matches "test" as a whole word)
    let options_boundary = SearchSessionOptions {
        boundary_mode: Some(BoundaryMode::Word),
        ..default_options(test_dir, "test")
    };
    let count_boundary = run_search(manager, options_boundary).await?;

    println!("  Without boundary: {count_no_boundary} results (test, testing, contest, etc.)");
    println!("  With word boundary: {count_boundary} results (only whole word 'test')");

    if count_boundary >= count_no_boundary {
        anyhow::bail!("Expected word boundary to reduce matches");
    }

    println!("  ✓ Word boundary working correctly\n");
    Ok(())
}

async fn test_line_boundary(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 4: Line Boundary Mode");

    // Without line boundary (matches partial lines)
    let count_no_line = run_search(manager, default_options(test_dir, "test")).await?;

    // With line boundary (only matches lines that are exactly "test")
    let options_line = SearchSessionOptions {
        boundary_mode: Some(BoundaryMode::Line),
        ..default_options(test_dir, "test")
    };
    let count_line = run_search(manager, options_line).await?;

    println!("  Without line boundary: {count_no_line} results");
    println!("  With line boundary: {count_line} results (only lines with just 'test')");

    if count_line >= count_no_line {
        anyhow::bail!("Expected line boundary to reduce matches");
    }

    println!("  ✓ Line boundary working correctly\n");
    Ok(())
}

async fn test_multiline(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 5: Multiline Pattern Matching");

    // Multiline pattern that spans multiple lines
    let options_multiline = SearchSessionOptions {
        pattern: "multi.*line.*pattern".to_string(),
        literal_search: false,
        multiline: true,
        ..default_options(test_dir, "")
    };
    let count_multiline = run_search(manager, options_multiline).await?;

    println!("  Multiline pattern found {count_multiline} results");

    if count_multiline == 0 {
        anyhow::bail!("Expected multiline pattern to find results");
    }

    println!("  ✓ Multiline matching working correctly\n");
    Ok(())
}

async fn test_inverted_match(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 6: Inverted Match");

    // Normal search for "test"
    let count_normal = run_search(manager, default_options(test_dir, "test")).await?;

    // Inverted search (lines that DON'T contain "test")
    let options_inverted = SearchSessionOptions {
        invert_match: true,
        ..default_options(test_dir, "test")
    };
    let count_inverted = run_search(manager, options_inverted).await?;

    println!("  Normal match: {count_normal} results (lines with 'test')");
    println!("  Inverted match: {count_inverted} results (lines without 'test')");

    if count_inverted == 0 {
        anyhow::bail!("Expected inverted match to find lines without 'test'");
    }

    println!("  ✓ Inverted match working correctly\n");
    Ok(())
}

async fn test_regex_special_chars(manager: &SearchManager, test_dir: &TempDir) -> Result<()> {
    println!("Test 7: Regex Special Characters");

    // Regex pattern with special chars
    let options_regex = SearchSessionOptions {
        pattern: r"test\[log\]".to_string(),
        literal_search: false,
        ..default_options(test_dir, "")
    };
    let count_regex = run_search(manager, options_regex).await?;

    // Literal pattern with special chars (brackets are literal)
    let options_literal = SearchSessionOptions {
        pattern: "test[log]".to_string(),
        literal_search: true,
        ..default_options(test_dir, "")
    };
    let count_literal = run_search(manager, options_literal).await?;

    println!("  Regex mode: {count_regex} results (escaped brackets)");
    println!("  Literal mode: {count_literal} results (literal brackets)");

    if count_literal == 0 {
        anyhow::bail!("Expected to find literal 'test[log]'");
    }

    println!("  ✓ Regex special characters handled correctly\n");
    Ok(())
}
