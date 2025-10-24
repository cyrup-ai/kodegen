pub mod discovery;
pub mod executor;
pub mod json_parser;
pub mod xml_parser;

pub use discovery::{AlfredScriptFilter, AlfredWorkflow, ScriptType, discover_workflows};
pub use executor::execute_script_filter;
pub use json_parser::{
    AlfredIcon, AlfredItem, AlfredModifier, AlfredScriptFilterOutput, AlfredText, parse_json,
};
pub use xml_parser::{AlfredXmlItem, AlfredXmlModifier, AlfredXmlOutput, parse_xml, xml_to_json};
