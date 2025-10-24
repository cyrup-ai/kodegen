use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("Home directory not found")]
    HomeDirectoryNotFound,

    #[error("Extension directory not found: {0}")]
    ExtensionDirectoryNotFound(PathBuf),

    #[error("Failed to parse metadata: {0}")]
    MetadataParseError(String),

    #[error("Process execution failed: {0}")]
    ProcessError(#[from] std::io::Error),

    #[error("Process timed out after {0} seconds")]
    Timeout(u64),

    #[error("Failed to parse JSON output: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Failed to parse XML output: {0}")]
    XmlParseError(String),

    #[error("Failed to parse plist: {0}")]
    PlistParseError(String),

    #[error("Missing required parameter: {0}")]
    MissingParameter(&'static str),

    #[error("Extension not found: {0}")]
    ExtensionNotFound(String),

    #[error("Invalid UTF-8 in output")]
    InvalidUtf8,

    #[error("Async join error: {0}")]
    JoinError(String),
}

impl From<plist::Error> for ExtensionError {
    fn from(err: plist::Error) -> Self {
        ExtensionError::PlistParseError(err.to_string())
    }
}

impl From<quick_xml::DeError> for ExtensionError {
    fn from(err: quick_xml::DeError) -> Self {
        ExtensionError::XmlParseError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ExtensionError>;
