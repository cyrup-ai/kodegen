use crate::extensions::alfred::json_parser::{
    AlfredIcon, AlfredItem, AlfredModifier, AlfredScriptFilterOutput,
};
use crate::extensions::common::types::Result;
use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlfredXmlOutput {
    #[serde(rename = "item", default)]
    pub items: Vec<AlfredXmlItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlfredXmlItem {
    #[serde(rename = "@uid")]
    pub uid: Option<String>,
    #[serde(rename = "@arg")]
    pub arg: Option<String>,
    #[serde(rename = "@valid")]
    pub valid: Option<String>,
    #[serde(rename = "@autocomplete")]
    pub autocomplete: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    #[serde(rename = "mod", default)]
    pub mods: Vec<AlfredXmlModifier>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlfredXmlModifier {
    #[serde(rename = "@key")]
    pub key: String,
    pub subtitle: Option<String>,
    pub arg: Option<String>,
}

pub fn parse_xml(output: &str) -> Result<AlfredXmlOutput> {
    Ok(from_str(output)?)
}

pub fn xml_to_json(xml_output: AlfredXmlOutput) -> AlfredScriptFilterOutput {
    AlfredScriptFilterOutput {
        items: xml_output
            .items
            .into_iter()
            .map(|xml_item| AlfredItem {
                uid: xml_item.uid,
                title: xml_item.title,
                subtitle: xml_item.subtitle,
                arg: xml_item.arg,
                autocomplete: xml_item.autocomplete,
                icon: xml_item.icon.map(|path| AlfredIcon {
                    path: Some(path),
                    icon_type: None,
                }),
                valid: xml_item.valid.map(|v| v == "yes" || v == "true"),
                mods: xml_item
                    .mods
                    .into_iter()
                    .map(|m| {
                        (
                            m.key.clone(),
                            AlfredModifier {
                                subtitle: m.subtitle,
                                arg: m.arg,
                                valid: None,
                            },
                        )
                    })
                    .collect(),
                text: None,
            })
            .collect(),
        skipknowledge: None,
        rerun: None,
        variables: None,
    }
}
