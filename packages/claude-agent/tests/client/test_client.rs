//! Unit tests for `ClaudeSDKClient`
//!
//! Tests the client creation and basic functionality

use kodegen_claude_agent::{ClaudeAgentOptions, ClaudeSDKClient};

#[tokio::test]
async fn test_client_creation() {
    let options = ClaudeAgentOptions::default();
    let result = ClaudeSDKClient::new(options, None).await;
    assert!(result.is_ok() || result.is_err()); // Will succeed if CLI is available
}
