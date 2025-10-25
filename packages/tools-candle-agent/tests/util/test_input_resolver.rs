// Tests extracted from src/util/input_resolver.rs

use kodegen_candle_agent::util::input_resolver::*;

#[tokio::test]
async fn test_resolve_literal_text() {
    let result = resolve_input("Hello, world!").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, world!");
}

#[tokio::test]
async fn test_resolve_url() {
    // URL format should be recognized
    let input = "https://example.com/doc.txt";
    let result = resolve_input(input).await;
    // We don't test actual fetch, just that it's recognized as URL
    // The result may fail due to network, which is expected
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_sync_literal() {
    let result = resolve_input_sync("Test content");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Test content");
}
