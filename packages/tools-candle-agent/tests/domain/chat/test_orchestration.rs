// Tests extracted from src/domain/chat/orchestration.rs

use kodegen_candle_agent::domain::chat::orchestration::{
    format_tools_for_selection, get_selected_tool_schemas,
};
use kodegen_candle_agent::ToolInfo;
use std::borrow::Cow;
use std::sync::Arc;

#[test]
fn test_format_tools_for_selection() {
    let schema = serde_json::json!({"type": "object"});
    let schema_map = if let serde_json::Value::Object(map) = schema {
        Arc::new(map)
    } else {
        Arc::new(serde_json::Map::new())
    };

    let tools = vec![
        ToolInfo {
            name: Cow::Owned("calculator".to_string()),
            title: None,
            description: Some(Cow::Owned("Perform calculations".to_string())),
            input_schema: schema_map.clone(),
            output_schema: None,
            annotations: None,
            icons: None,
        },
        ToolInfo {
            name: Cow::Owned("search".to_string()),
            title: None,
            description: Some(Cow::Owned("Search the web".to_string())),
            input_schema: schema_map,
            output_schema: None,
            annotations: None,
            icons: None,
        },
    ];

    let formatted = format_tools_for_selection(&tools);
    assert!(formatted.contains("calculator: Perform calculations"));
    assert!(formatted.contains("search: Search the web"));
}

#[test]
fn test_get_selected_tool_schemas() {
    let schema = serde_json::json!({});
    let schema_map = if let serde_json::Value::Object(map) = schema {
        Arc::new(map)
    } else {
        Arc::new(serde_json::Map::new())
    };

    let tools = vec![
        ToolInfo {
            name: Cow::Owned("tool1".to_string()),
            title: None,
            description: Some(Cow::Owned("desc1".to_string())),
            input_schema: schema_map.clone(),
            output_schema: None,
            annotations: None,
            icons: None,
        },
        ToolInfo {
            name: Cow::Owned("tool2".to_string()),
            title: None,
            description: Some(Cow::Owned("desc2".to_string())),
            input_schema: schema_map,
            output_schema: None,
            annotations: None,
            icons: None,
        },
    ];

    let selected = get_selected_tool_schemas(&["tool1".to_string()], &tools);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].name.as_ref(), "tool1");
}
