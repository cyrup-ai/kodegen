//! Configuration options for message formatting

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::chat::formatting::error::{FormatError, FormatResult};
use regex::Regex;

bitflags::bitflags! {
    /// Formatting feature flags for zero-allocation feature checks
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct FormatFlags: u8 {
        const MARKDOWN = 1 << 0;
        const SYNTAX_HIGHLIGHTING = 1 << 1;
        const INLINE_FORMATTING = 1 << 2;
        const LINK_DETECTION = 1 << 3;
        const EMOJI = 1 << 4;
        const METADATA = 1 << 5;
        const OPTIMIZATIONS = 1 << 6;
    }
}

impl Default for FormatFlags {
    fn default() -> Self {
        Self::MARKDOWN | Self::INLINE_FORMATTING | Self::LINK_DETECTION | Self::OPTIMIZATIONS
    }
}

/// Immutable formatting options with owned strings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmutableFormatOptions {
    /// Formatting feature flags
    pub flags: FormatFlags,
    /// Maximum line length for text wrapping (0 = no wrapping)
    pub max_line_length: usize,
    /// Indentation size for nested content
    pub indent_size: usize,
    /// Theme for syntax highlighting
    pub syntax_theme: SyntaxTheme,
    /// Color scheme for formatting
    pub color_scheme: ImmutableColorScheme,
    /// Output format target
    pub output_format: OutputFormat,
    /// Custom CSS classes for HTML output
    pub custom_css_classes: HashMap<String, String>,
    /// Custom formatting rules
    pub custom_rules: Vec<ImmutableCustomFormatRule>,
}

impl ImmutableFormatOptions {
    /// Create new format options with default values
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create format options optimized for terminal output
    #[inline]
    #[must_use]
    pub fn terminal() -> Self {
        Self {
            flags: FormatFlags::default() | FormatFlags::OPTIMIZATIONS,
            output_format: OutputFormat::AnsiTerminal,
            max_line_length: 120,
            ..Default::default()
        }
    }

    /// Create format options optimized for HTML output
    #[inline]
    #[must_use]
    pub fn html() -> Self {
        Self {
            flags: FormatFlags::MARKDOWN
                | FormatFlags::SYNTAX_HIGHLIGHTING
                | FormatFlags::METADATA
                | FormatFlags::INLINE_FORMATTING
                | FormatFlags::LINK_DETECTION,
            output_format: OutputFormat::Html,
            ..Default::default()
        }
    }

    /// Create format options optimized for plain text
    #[inline]
    #[must_use]
    pub fn plain_text() -> Self {
        Self {
            flags: FormatFlags::OPTIMIZATIONS,
            output_format: OutputFormat::PlainText,
            ..Default::default()
        }
    }

    /// Validate configuration
    ///
    /// # Errors
    ///
    /// Returns `FormatError::ConfigurationError` if validation fails
    #[inline]
    pub fn validate(&self) -> FormatResult<()> {
        if self.max_line_length > 0 && self.max_line_length < 10 {
            return Err(FormatError::ConfigurationError {
                detail: "Max line length must be at least 10 or 0 for no wrapping".to_string(),
            });
        }
        if self.indent_size > 20 {
            return Err(FormatError::ConfigurationError {
                detail: "Indent size cannot exceed 20".to_string(),
            });
        }
        for rule in &self.custom_rules {
            rule.validate()?;
        }
        Ok(())
    }
}

impl Default for ImmutableFormatOptions {
    #[inline]
    fn default() -> Self {
        Self {
            flags: FormatFlags::MARKDOWN
                | FormatFlags::SYNTAX_HIGHLIGHTING
                | FormatFlags::INLINE_FORMATTING
                | FormatFlags::LINK_DETECTION
                | FormatFlags::EMOJI
                | FormatFlags::OPTIMIZATIONS,
            max_line_length: 80,
            indent_size: 2,
            syntax_theme: SyntaxTheme::GitHub,
            color_scheme: ImmutableColorScheme::default(),
            output_format: OutputFormat::Html,
            custom_css_classes: HashMap::new(),
            custom_rules: Vec::new(),
        }
    }
}

/// Syntax highlighting themes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyntaxTheme {
    /// Light theme with dark text
    Light,
    /// Dark theme with light text
    Dark,
    /// High contrast theme
    HighContrast,
    /// Solarized light theme
    SolarizedLight,
    /// Solarized dark theme
    SolarizedDark,
    /// GitHub theme
    GitHub,
    /// VS Code theme
    VSCode,
    /// Custom theme
    Custom,
}
impl SyntaxTheme {
    /// Get theme name as static string (zero allocation)
    #[inline]
    #[must_use]
    pub fn theme_name(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::HighContrast => "high-contrast",
            Self::SolarizedLight => "solarized-light",
            Self::SolarizedDark => "solarized-dark",
            Self::GitHub => "github",
            Self::VSCode => "vscode",
            Self::Custom => "custom",
        }
    }

    /// Check if theme is dark
    #[inline]
    #[must_use]
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark | Self::SolarizedDark)
    }
}

/// Immutable color scheme with owned strings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImmutableColorScheme {
    /// Primary text color
    pub primary_text: String,
    /// Secondary text color
    pub secondary_text: String,
    /// Background color
    pub background: String,
    /// Accent color
    pub accent: String,
    /// Error color
    pub error: String,
    /// Warning color
    pub warning: String,
    /// Success color
    pub success: String,
    /// Link color
    pub link: String,
}
impl ImmutableColorScheme {
    /// Create new color scheme with validation
    ///
    /// Users should construct the struct directly and call `validated()` to ensure validity.
    ///
    /// # Example
    /// ```ignore
    /// let scheme = ImmutableColorScheme {
    ///     primary_text: "#000000".to_string(),
    ///     secondary_text: "#666666".to_string(),
    ///     // ... other fields
    /// }.validated()?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `FormatError::ConfigurationError` if any color format is invalid
    #[inline]
    pub fn validated(self) -> FormatResult<Self> {
        self.validate()?;
        Ok(self)
    }

    /// Validate color values
    ///
    /// # Errors
    ///
    /// Returns `FormatError::ConfigurationError` if any color format is invalid
    #[inline]
    pub fn validate(&self) -> FormatResult<()> {
        let colors = [
            &self.primary_text,
            &self.secondary_text,
            &self.background,
            &self.accent,
            &self.error,
            &self.warning,
            &self.success,
            &self.link,
        ];

        for color in &colors {
            if !Self::is_valid_color(color) {
                return Err(FormatError::ConfigurationError {
                    detail: format!("Invalid color format: {color}"),
                });
            }
        }
        Ok(())
    }

    /// Check if color string is valid (hex format)
    #[inline]
    fn is_valid_color(color: &str) -> bool {
        if !color.starts_with('#') || color.len() != 7 {
            return false;
        }
        color[1..].chars().all(|c| c.is_ascii_hexdigit())
    }
}

impl Default for ImmutableColorScheme {
    #[inline]
    fn default() -> Self {
        Self {
            primary_text: "#333333".to_string(),
            secondary_text: "#666666".to_string(),
            background: "#ffffff".to_string(),
            accent: "#0066cc".to_string(),
            error: "#cc0000".to_string(),
            warning: "#ff9900".to_string(),
            success: "#00cc00".to_string(),
            link: "#0066cc".to_string(),
        }
    }
}

/// Output format targets
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutputFormat {
    /// Plain text output
    PlainText,
    /// HTML output
    Html,
    /// Markdown output
    Markdown,
    /// ANSI colored terminal output
    AnsiTerminal,
    /// Rich text format
    RichText,
    /// LaTeX output
    LaTeX,
}

impl OutputFormat {
    /// Get format name as static string (zero allocation)
    #[inline]
    #[must_use]
    pub fn format_name(&self) -> &'static str {
        match self {
            Self::PlainText => "plain-text",
            Self::Html => "html",
            Self::Markdown => "markdown",
            Self::AnsiTerminal => "ansi-terminal",
            Self::RichText => "rich-text",
            Self::LaTeX => "latex",
        }
    }

    /// Check if format supports styling
    #[inline]
    #[must_use]
    pub fn supports_styling(&self) -> bool {
        !matches!(self, Self::PlainText)
    }

    /// Check if format supports colors
    #[inline]
    #[must_use]
    pub fn supports_colors(&self) -> bool {
        matches!(self, Self::Html | Self::AnsiTerminal | Self::RichText)
    }
}

/// Immutable custom formatting rules with owned strings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmutableCustomFormatRule {
    /// Rule name/identifier
    pub name: String,
    /// Pattern to match (regex)
    pub pattern: String,
    /// Replacement template
    pub replacement: String,
    /// Rule priority (higher = applied first)
    pub priority: u32,
    /// Whether rule is enabled
    pub enabled: bool,
}

impl ImmutableCustomFormatRule {
    /// Create new custom format rule with validation
    ///
    /// # Errors
    ///
    /// Returns `FormatError::ConfigurationError` if name or pattern is empty
    #[inline]
    pub fn new(
        name: String,
        pattern: String,
        replacement: String,
        priority: u32,
        enabled: bool,
    ) -> FormatResult<Self> {
        if name.is_empty() {
            return Err(FormatError::ConfigurationError {
                detail: "Rule name cannot be empty".to_string(),
            });
        }
        if pattern.is_empty() {
            return Err(FormatError::ConfigurationError {
                detail: "Rule pattern cannot be empty".to_string(),
            });
        }

        Ok(Self {
            name,
            pattern,
            replacement,
            priority,
            enabled,
        })
    }

    /// Validate rule configuration
    ///
    /// # Errors
    ///
    /// Returns `FormatError::ConfigurationError` if validation fails
    #[inline]
    pub fn validate(&self) -> FormatResult<()> {
        if self.name.is_empty() {
            return Err(FormatError::ConfigurationError {
                detail: "Rule name cannot be empty".to_string(),
            });
        }
        if self.pattern.is_empty() {
            return Err(FormatError::ConfigurationError {
                detail: "Rule pattern cannot be empty".to_string(),
            });
        }
        // Validate regex pattern syntax
        Regex::new(&self.pattern).map_err(|e| FormatError::ConfigurationError {
            detail: format!("Invalid regex pattern '{}': {}", self.pattern, e),
        })?;
        Ok(())
    }
}
