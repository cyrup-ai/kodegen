use crate::extensions::common::types::{ExtensionError, Result};
use plist::Value;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlfredWorkflow {
    pub uid: String,
    pub name: String,
    pub description: Option<String>,
    pub bundle_id: String,
    pub script_filters: Vec<AlfredScriptFilter>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlfredScriptFilter {
    pub uid: String,
    pub title: String,
    pub script: String,
    pub script_type: ScriptType,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptType {
    Bash,
    Python,
    Ruby,
    PHP,
    JavaScript,
    AppleScript,
    Other(String),
}

pub async fn discover_workflows() -> Result<Vec<AlfredWorkflow>> {
    let home_dir = dirs::home_dir().ok_or(ExtensionError::HomeDirectoryNotFound)?;

    let workflows_path = home_dir
        .join("Library")
        .join("Application Support")
        .join("Alfred")
        .join("Alfred.alfredpreferences")
        .join("workflows");

    // If Alfred not installed, return empty vec
    if !workflows_path.exists() {
        return Ok(Vec::new());
    }

    let mut workflows = Vec::new();
    let mut entries = fs::read_dir(&workflows_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let info_plist = path.join("info.plist");
            if info_plist.exists()
                && let Ok(workflow) = parse_workflow_metadata(&path, &info_plist).await
            {
                workflows.push(workflow);
            }
        }
    }

    Ok(workflows)
}

async fn parse_workflow_metadata(
    workflow_path: &Path,
    info_plist: &PathBuf,
) -> Result<AlfredWorkflow> {
    let plist = plist::from_file(info_plist)?;

    let dict = match plist {
        Value::Dictionary(d) => d,
        _ => {
            return Err(ExtensionError::MetadataParseError(
                "Invalid plist structure".to_string(),
            ));
        }
    };

    let name = dict
        .get("name")
        .and_then(|v| v.as_string())
        .unwrap_or("Unknown")
        .to_string();

    let description = dict
        .get("description")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let bundle_id = dict
        .get("bundleid")
        .and_then(|v| v.as_string())
        .unwrap_or("unknown")
        .to_string();

    let uid = workflow_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut script_filters = Vec::new();

    if let Some(objects) = dict.get("objects").and_then(|v| v.as_array()) {
        for obj in objects {
            if let Value::Dictionary(obj_dict) = obj {
                let obj_type = obj_dict
                    .get("type")
                    .and_then(|v| v.as_string())
                    .unwrap_or("");

                if obj_type == "alfred.workflow.input.scriptfilter"
                    && let Ok(filter) = parse_script_filter(obj_dict)
                {
                    script_filters.push(filter);
                }
            }
        }
    }

    Ok(AlfredWorkflow {
        uid,
        name,
        description,
        bundle_id,
        script_filters,
        path: workflow_path.to_path_buf(),
    })
}

fn parse_script_filter(dict: &plist::Dictionary) -> Result<AlfredScriptFilter> {
    let uid = dict
        .get("uid")
        .and_then(|v| v.as_string())
        .unwrap_or("unknown")
        .to_string();

    let title = dict
        .get("title")
        .and_then(|v| v.as_string())
        .or_else(|| dict.get("keyword").and_then(|v| v.as_string()))
        .unwrap_or("Unknown")
        .to_string();

    let script = dict
        .get("script")
        .and_then(|v| v.as_string())
        .unwrap_or("")
        .to_string();

    let script_lang = dict
        .get("scriptargtype")
        .and_then(|v| v.as_unsigned_integer())
        .unwrap_or(0);

    let script_type = match script_lang {
        0 => ScriptType::Bash,
        1 => ScriptType::AppleScript,
        2 => ScriptType::Other("zsh".to_string()),
        3 => ScriptType::Other("fish".to_string()),
        4 => ScriptType::Python,
        5 => ScriptType::Ruby,
        6 => ScriptType::PHP,
        7 => ScriptType::JavaScript,
        _ => ScriptType::Bash,
    };

    let keyword = dict
        .get("keyword")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    Ok(AlfredScriptFilter {
        uid,
        title,
        script,
        script_type,
        keyword,
    })
}
