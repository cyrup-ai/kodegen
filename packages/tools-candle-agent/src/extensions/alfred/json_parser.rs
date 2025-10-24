use crate::extensions::common::types::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlfredScriptFilterOutput {
    pub skipknowledge: Option<bool>,
    pub rerun: Option<f64>,
    pub variables: Option<HashMap<String, String>>,
    pub items: Vec<AlfredItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlfredItem {
    pub uid: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub arg: Option<String>,
    pub autocomplete: Option<String>,
    pub icon: Option<AlfredIcon>,
    pub valid: Option<bool>,
    #[serde(default)]
    pub mods: HashMap<String, AlfredModifier>,
    pub text: Option<AlfredText>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlfredIcon {
    pub path: Option<String>,
    #[serde(rename = "type")]
    pub icon_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlfredModifier {
    pub subtitle: Option<String>,
    pub arg: Option<String>,
    pub valid: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlfredText {
    pub copy: Option<String>,
    pub largetype: Option<String>,
}

pub fn parse_json(output: &str) -> Result<AlfredScriptFilterOutput> {
    Ok(serde_json::from_str(output)?)
}
