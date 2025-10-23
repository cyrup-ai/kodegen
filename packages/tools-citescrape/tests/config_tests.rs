//! Tests for the type-safe configuration builder pattern

use kodegen_tools_citescrape::config::CrawlConfig;
use std::path::PathBuf;
use tempfile::TempDir;

mod common;

#[tokio::test]
async fn test_builder_requires_storage_dir_and_start_url() {
    // This should not compile if uncommented - testing compile-time guarantees
    // let config = CrawlConfig::builder().build();

    // This should also not compile - missing start_url
    // let config = CrawlConfig::builder()
    //     .storage_dir(PathBuf::from("/tmp"))
    //     .build();

    // This should also not compile - missing storage_dir
    // let config = CrawlConfig::builder()
    //     .start_url("https://example.com")
    //     .build();

    // This SHOULD compile - both required fields provided
    let temp_dir = TempDir::new().unwrap();
    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .build()
        .unwrap();

    assert_eq!(config.storage_dir(), temp_dir.path());
    assert_eq!(config.start_url(), "https://example.com");
}

#[tokio::test]
async fn test_builder_optional_fields_have_defaults() {
    let temp_dir = TempDir::new().unwrap();
    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .build()
        .unwrap();

    // Check defaults
    assert_eq!(config.limit(), None);
    assert_eq!(config.allowed_domains(), None);
    assert_eq!(config.excluded_patterns(), None);
    assert!(config.headless());
    assert!(config.save_screenshots());
    assert!(config.save_markdown());
    assert!(config.extract_main_content());
    assert!(!config.save_raw_html());
    assert_eq!(config.max_depth(), 3);
}

#[tokio::test]
async fn test_builder_with_all_optional_fields() {
    let temp_dir = TempDir::new().unwrap();
    let allowed_domains = vec!["example.com".to_string(), "test.com".to_string()];
    let excluded_patterns = vec![r"\.pdf$".to_string(), r"\.zip$".to_string()];

    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .limit(Some(100))
        .allowed_domains(Some(allowed_domains.clone()))
        .excluded_patterns(Some(excluded_patterns.clone()))
        .headless(false)
        .save_screenshots(true)
        .save_raw_html(true)
        .extract_main_content(false)
        .save_markdown(false)
        .max_depth(5)
        .screenshot_quality(95)
        .stealth_mode(true)
        .build()
        .unwrap();

    assert_eq!(config.limit(), Some(100));
    assert_eq!(config.allowed_domains(), Some(&allowed_domains));
    assert_eq!(config.excluded_patterns(), Some(&excluded_patterns));
    assert!(!config.headless());
    assert!(config.save_screenshots());
    assert!(config.save_raw_html());
    assert!(!config.extract_main_content());
    assert!(!config.save_markdown());
    assert_eq!(config.max_depth(), 5);
    assert_eq!(config.screenshot_quality(), 95);
    assert!(config.stealth_mode());
}

#[tokio::test]
async fn test_builder_field_override() {
    let temp_dir = TempDir::new().unwrap();

    // Test that we can override fields multiple times
    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .limit(Some(50))
        .limit(Some(100)) // Override previous value
        .headless(true)
        .headless(false) // Override previous value
        .build()
        .unwrap();

    assert_eq!(config.limit(), Some(100));
    assert!(!config.headless());
}

#[tokio::test]
async fn test_url_normalization_in_builder() {
    let temp_dir = TempDir::new().unwrap();

    // Test various URL formats
    let test_cases = vec![
        ("example.com", "https://example.com"),
        ("http://example.com", "http://example.com"),
        ("https://example.com", "https://example.com"),
        ("https://example.com/", "https://example.com/"),
        ("https://example.com/path", "https://example.com/path"),
    ];

    for (input, expected) in test_cases {
        let config = CrawlConfig::builder()
            .storage_dir(temp_dir.path().to_path_buf())
            .start_url(input)
            .build()
            .unwrap();

        assert_eq!(config.start_url(), expected);
    }
}

#[tokio::test]
async fn test_storage_dir_path_handling() {
    // Test with absolute path
    let abs_path = PathBuf::from("/tmp/test");
    let config = CrawlConfig::builder()
        .storage_dir(abs_path.clone())
        .start_url("https://example.com")
        .build()
        .unwrap();
    assert_eq!(config.storage_dir(), &abs_path);

    // Test with relative path
    let rel_path = PathBuf::from("./output");
    let config = CrawlConfig::builder()
        .storage_dir(rel_path.clone())
        .start_url("https://example.com")
        .build()
        .unwrap();
    assert_eq!(config.storage_dir(), &rel_path);
}

#[tokio::test]
async fn test_config_validation_logic() {
    let temp_dir = TempDir::new().unwrap();

    // Test empty allowed_domains
    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .allowed_domains(Some(vec![]))
        .build()
        .unwrap();
    assert_eq!(config.allowed_domains(), Some(&vec![]));

    // Test empty excluded_patterns
    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .excluded_patterns(Some(vec![]))
        .build()
        .unwrap();
    assert_eq!(config.excluded_patterns(), Some(&vec![]));
}

#[tokio::test]
async fn test_config_serialization() {
    let temp_dir = TempDir::new().unwrap();
    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .limit(Some(50))
        .build()
        .unwrap();

    // Test that we can serialize to JSON (config has Serialize trait)
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("https://example.com"));

    // Test deserialization
    let _deserialized: CrawlConfig = serde_json::from_str(&json).unwrap();
}

#[tokio::test]
async fn test_config_debug_trait() {
    let temp_dir = TempDir::new().unwrap();
    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .build()
        .unwrap();

    // Test that Debug trait is implemented
    let debug_str = format!("{config:?}");
    assert!(debug_str.contains("CrawlConfig"));
    assert!(debug_str.contains("storage_dir"));
    assert!(debug_str.contains("start_url"));
}

#[tokio::test]
async fn test_builder_state_transitions() {
    // This test verifies the type-state pattern works correctly
    let temp_dir = TempDir::new().unwrap();

    // Create builder in initial state
    let builder = CrawlConfig::builder();

    // After setting storage_dir, we should be in WithStorageDir state
    let builder_with_storage = builder.storage_dir(temp_dir.path().to_path_buf());

    // After setting start_url, we should be in Complete state and can build
    let _config = builder_with_storage
        .start_url("https://example.com")
        .build()
        .unwrap();

    // The above should compile and work correctly
}

// NOTE: These tests are commented out because max_concurrent_requests and request_timeout
// methods don't exist on the CrawlConfigBuilder. These may need to be re-added to the builder
// or these tests should be removed.
/*
#[test]
fn test_concurrent_request_limits() {
    let temp_dir = TempDir::new().unwrap();

    // Test edge cases for concurrent requests
    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .max_concurrent_requests(Some(1))
        .build()
        .unwrap();
    assert_eq!(config.max_concurrent_requests, 1);

    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .max_concurrent_requests(Some(1000))
        .build()
        .unwrap();
    assert_eq!(config.max_concurrent_requests, 1000);
}

#[test]
fn test_timeout_duration_handling() {
    let temp_dir = TempDir::new().unwrap();

    // Test various timeout durations
    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .request_timeout(Some(std::time::Duration::from_millis(1)))
        .build()
        .unwrap();
    assert_eq!(config.request_timeout, std::time::Duration::from_millis(1));

    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .request_timeout(Some(std::time::Duration::from_secs(3600)))
        .build()
        .unwrap();
    assert_eq!(config.request_timeout, std::time::Duration::from_secs(3600));
}
*/
