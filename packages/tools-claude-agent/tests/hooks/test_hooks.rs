//! Unit tests for `HookManager`
//!
//! Tests the hook system for intercepting agent events

use kodegen_tools_claude_agent::hooks::{HookManager, HookMatcherBuilder};
use kodegen_tools_claude_agent::{HookContext, HookOutput};

#[tokio::test]
async fn test_hook_manager() {
    let mut manager = HookManager::new();

    // Register a hook
    let hook = HookManager::callback(|_event_data, _tool_name, _context| async {
        Ok(HookOutput::default())
    });

    let matcher = HookMatcherBuilder::new(Some("*")).add_hook(hook).build();
    manager.register(matcher);

    // Invoke hook
    let context = HookContext {};
    let result = manager
        .invoke(serde_json::json!({}), Some("test".to_string()), context)
        .await;
    assert!(result.is_ok());
}

#[test]
fn test_matcher_wildcard() {
    assert!(HookManager::matches(
        Some("*".to_string()).as_ref(),
        Some("any_tool".to_string()).as_ref()
    ));
    assert!(HookManager::matches(
        None,
        Some("any_tool".to_string()).as_ref()
    ));
}

#[test]
fn test_matcher_specific() {
    assert!(HookManager::matches(
        Some("Bash".to_string()).as_ref(),
        Some("Bash".to_string()).as_ref()
    ));
    assert!(!HookManager::matches(
        Some("Bash".to_string()).as_ref(),
        Some("Write".to_string()).as_ref()
    ));
}

#[test]
fn test_matcher_pattern() {
    assert!(HookManager::matches(
        Some("Write|Edit".to_string()).as_ref(),
        Some("Write".to_string()).as_ref()
    ));
    assert!(HookManager::matches(
        Some("Write|Edit".to_string()).as_ref(),
        Some("Edit".to_string()).as_ref()
    ));
    assert!(!HookManager::matches(
        Some("Write|Edit".to_string()).as_ref(),
        Some("Bash".to_string()).as_ref()
    ));
}
