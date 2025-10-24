use crate::extensions::common::types::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaycastResult {
    pub items: Vec<RaycastItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaycastItem {
    pub title: String,
    pub subtitle: Option<String>,
    pub arg: Option<String>,
    pub icon: Option<RaycastIcon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaycastIcon {
    pub path: String,
}

/// Attempt to parse Raycast Script Command JSON output
/// Returns Ok(Some(result)) if valid JSON, Ok(None) if not JSON, Err on parse error
pub fn parse_json_output(output: &str) -> Result<Option<RaycastResult>> {
    if output.trim().is_empty() {
        return Ok(None);
    }

    // Try to parse as JSON
    match serde_json::from_str::<RaycastResult>(output) {
        Ok(result) => Ok(Some(result)),
        Err(_) => {
            // Not JSON format, return None (plain text output)
            Ok(None)
        }
    }
}

/// Parse plain text output as a simple result
pub fn parse_text_output(output: &str) -> RaycastResult {
    let lines: Vec<&str> = output.lines().collect();
    let items = lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| RaycastItem {
            title: line.to_string(),
            subtitle: None,
            arg: Some(line.to_string()),
            icon: None,
        })
        .collect();

    RaycastResult { items }
}
