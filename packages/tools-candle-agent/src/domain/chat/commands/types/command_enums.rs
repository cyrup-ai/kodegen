//! Command utility enumerations with zero allocation patterns
//!
//! Provides blazing-fast enumeration dispatch with owned strings allocated once
//! for maximum performance. No Arc usage, no locking, pure enum-based dispatch.

use serde::{Deserialize, Serialize};

/// Settings category enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SettingsCategory {
    /// General application settings
    General,
    /// User interface settings
    Interface,
    /// Performance and optimization settings
    Performance,
    /// Security and privacy settings
    Security,
    /// Network and connectivity settings
    Network,
    /// Storage and persistence settings
    Storage,
    /// Logging and debugging settings
    Logging,

    /// Appearance and theming settings
    Appearance,
    /// Accessibility settings
    Accessibility,
}

impl SettingsCategory {
    /// Get category name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Interface => "interface",
            Self::Performance => "performance",
            Self::Security => "security",
            Self::Network => "network",
            Self::Storage => "storage",
            Self::Logging => "logging",

            Self::Appearance => "appearance",
            Self::Accessibility => "accessibility",
        }
    }
}

/// Output type enumeration for zero allocation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputType {
    /// Plain text output
    Text,
    /// JSON structured data
    Json,
    /// Markdown formatted text
    Markdown,
    /// HTML formatted output
    Html,
    /// CSV tabular data
    Csv,
    /// XML structured data
    Xml,
    /// YAML data format
    Yaml,
    /// Binary data
    Binary,
    /// Image data
    Image,
    /// Audio data
    Audio,
    /// Video data
    Video,
    /// Log entries
    Log,
    /// Error messages
    Error,
}

impl OutputType {
    /// Get type name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Csv => "csv",
            Self::Xml => "xml",
            Self::Yaml => "yaml",
            Self::Binary => "binary",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Log => "log",
            Self::Error => "error",
        }
    }

    /// Get MIME type for HTTP responses
    #[inline]
    #[must_use]
    pub const fn mime_type(&self) -> &'static str {
        match self {
            Self::Text | Self::Log | Self::Error => "text/plain",
            Self::Json => "application/json",
            Self::Markdown => "text/markdown",
            Self::Html => "text/html",
            Self::Csv => "text/csv",
            Self::Xml => "application/xml",
            Self::Yaml => "application/yaml",
            Self::Binary => "application/octet-stream",
            Self::Image => "image/png",
            Self::Audio => "audio/mpeg",
            Self::Video => "video/mp4",
        }
    }
}
