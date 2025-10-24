//! Core content types for message formatting

use serde::{Deserialize, Serialize};

use crate::domain::chat::formatting::error::{FormatError, FormatResult};

/// Immutable message content with owned strings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImmutableMessageContent {
    /// Plain text content
    Plain {
        /// The plain text content
        text: String,
    },
    /// Markdown formatted content
    Markdown {
        /// The raw markdown content
        content: String,
        /// Optional pre-rendered HTML version
        rendered_html: Option<String>,
    },
    /// Code block with syntax highlighting
    Code {
        /// The source code content
        content: String,
        /// Programming language for syntax highlighting
        language: String,
        /// Optional pre-highlighted HTML version
        highlighted: Option<String>,
    },
    /// Formatted content with inline styling
    Formatted {
        /// The base content text
        content: String,
        /// Applied formatting styles
        styles: Vec<FormatStyle>,
    },
    /// Composite content with multiple parts
    Composite {
        /// Individual content parts making up the composite
        parts: Vec<ImmutableMessageContent>,
    },
}

impl ImmutableMessageContent {
    /// Get content as borrowed string (zero allocation)
    #[inline]
    #[must_use]
    pub fn as_text(&self) -> &str {
        match self {
            Self::Plain { text } => text,
            Self::Markdown { content, .. }
            | Self::Code { content, .. }
            | Self::Formatted { content, .. } => content,
            Self::Composite { .. } => "", // Composite content needs rendering
        }
    }

    /// Get content type as static string (zero allocation)
    #[inline]
    #[must_use]
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Plain { .. } => "plain",
            Self::Markdown { .. } => "markdown",
            Self::Code { .. } => "code",
            Self::Formatted { .. } => "formatted",
            Self::Composite { .. } => "composite",
        }
    }

    /// Check if content is empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Plain { text } => text.is_empty(),
            Self::Markdown { content, .. }
            | Self::Code { content, .. }
            | Self::Formatted { content, .. } => content.is_empty(),
            Self::Composite { parts } => parts.is_empty(),
        }
    }

    /// Get estimated character count
    #[inline]
    pub fn char_count(&self) -> usize {
        match self {
            Self::Plain { text } => text.chars().count(),
            Self::Markdown { content, .. }
            | Self::Code { content, .. }
            | Self::Formatted { content, .. } => content.chars().count(),
            Self::Composite { parts } => parts.iter().map(Self::char_count).sum(),
        }
    }

    /// Validate content structure
    ///
    /// # Errors
    ///
    /// Returns `FormatError::InvalidContent` if validation fails
    #[inline]
    pub fn validate(&self) -> FormatResult<()> {
        match self {
            Self::Code { language, .. } => {
                if language.is_empty() {
                    return Err(FormatError::InvalidContent {
                        detail: "Code language cannot be empty".to_string(),
                    });
                }
            }
            Self::Formatted { content, styles } => {
                for style in styles {
                    if style.end > content.len() {
                        return Err(FormatError::InvalidContent {
                            detail: "Style range exceeds content length".to_string(),
                        });
                    }
                    if style.start >= style.end {
                        return Err(FormatError::InvalidContent {
                            detail: "Invalid style range".to_string(),
                        });
                    }
                }
            }
            Self::Composite { parts } => {
                for part in parts {
                    part.validate()?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Formatting styles for inline text
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormatStyle {
    /// Start position in the text
    pub start: usize,
    /// End position in the text
    pub end: usize,
    /// Style type
    pub style: StyleType,
}

impl FormatStyle {
    /// Create new format style with validation
    ///
    /// # Errors
    ///
    /// Returns `FormatError::InvalidContent` if start >= end
    #[inline]
    pub fn new(start: usize, end: usize, style: StyleType) -> FormatResult<Self> {
        if start >= end {
            return Err(FormatError::InvalidContent {
                detail: "Style start must be less than end".to_string(),
            });
        }
        Ok(Self { start, end, style })
    }

    /// Get style length
    #[inline]
    #[must_use]
    pub fn length(&self) -> usize {
        self.end - self.start
    }

    /// Check if style overlaps with another
    #[inline]
    #[must_use]
    pub fn overlaps_with(&self, other: &FormatStyle) -> bool {
        !(self.end <= other.start || other.end <= self.start)
    }
}

/// Available style types with owned strings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StyleType {
    /// Bold text formatting
    Bold,
    /// Italic text formatting
    Italic,
    /// Underlined text formatting
    Underline,
    /// Strikethrough text formatting
    Strikethrough,
    /// Inline code formatting
    Code,
    /// Hyperlink with URL
    Link {
        /// URL target for the link
        url: String,
    },
    /// Text color formatting
    Color {
        /// RGB color value (24-bit)
        rgb: u32,
    },
    /// Background color formatting
    Background {
        /// RGB background color value (24-bit)
        rgb: u32,
    },
}

impl StyleType {
    /// Get style name as static string (zero allocation)
    #[inline]
    #[must_use]
    pub fn style_name(&self) -> &'static str {
        match self {
            Self::Bold => "bold",
            Self::Italic => "italic",
            Self::Underline => "underline",
            Self::Strikethrough => "strikethrough",
            Self::Code => "code",
            Self::Link { .. } => "link",
            Self::Color { .. } => "color",
            Self::Background { .. } => "background",
        }
    }

    /// Check if style requires additional data
    #[inline]
    #[must_use]
    pub fn requires_data(&self) -> bool {
        matches!(
            self,
            Self::Link { .. } | Self::Color { .. } | Self::Background { .. }
        )
    }
}
