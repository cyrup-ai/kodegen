//! Unit tests for `PermissionManager`
//!
//! Tests the permission system for tool access control

use kodegen_tools_claude_agent::permissions::PermissionManager;
use kodegen_tools_claude_agent::{PermissionResult, ToolName, ToolPermissionContext};

#[tokio::test]
async fn test_permission_manager_default_allow() {
    let manager = PermissionManager::new();

    let result = manager
        .can_use_tool(
            ToolName::new("test_tool"),
            serde_json::json!({}),
            ToolPermissionContext {
                suggestions: vec![],
            },
        )
        .await
        .unwrap();

    match result {
        PermissionResult::Allow(_) => {}
        PermissionResult::Deny(_) => panic!("Expected allow"),
    }
}

#[tokio::test]
async fn test_permission_manager_disallowed() {
    let mut manager = PermissionManager::new();
    manager.set_disallowed_tools(vec![ToolName::new("bad_tool")]);

    let result = manager
        .can_use_tool(
            ToolName::new("bad_tool"),
            serde_json::json!({}),
            ToolPermissionContext {
                suggestions: vec![],
            },
        )
        .await
        .unwrap();

    match result {
        PermissionResult::Allow(_) => panic!("Expected deny"),
        PermissionResult::Deny(_) => {}
    }
}

#[tokio::test]
async fn test_permission_manager_allowed_list() {
    let mut manager = PermissionManager::new();
    manager.set_allowed_tools(Some(vec![ToolName::new("good_tool")]));

    // Should allow good_tool
    let result = manager
        .can_use_tool(
            ToolName::new("good_tool"),
            serde_json::json!({}),
            ToolPermissionContext {
                suggestions: vec![],
            },
        )
        .await
        .unwrap();

    match result {
        PermissionResult::Allow(_) => {}
        PermissionResult::Deny(_) => panic!("Expected allow"),
    }

    // Should deny other_tool
    let result = manager
        .can_use_tool(
            ToolName::new("other_tool"),
            serde_json::json!({}),
            ToolPermissionContext {
                suggestions: vec![],
            },
        )
        .await
        .unwrap();

    match result {
        PermissionResult::Allow(_) => panic!("Expected deny"),
        PermissionResult::Deny(_) => {}
    }
}
