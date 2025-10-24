//! Legacy compatibility type aliases (deprecated)

use crate::domain::chat::formatting::{
    content::ImmutableMessageContent,
    formatter::StreamingMessageFormatter,
    options::{ImmutableColorScheme, ImmutableCustomFormatRule, ImmutableFormatOptions},
};

/// Deprecated alias for `ImmutableMessageContent` - use `ImmutableMessageContent` instead for zero-allocation streaming
#[deprecated(note = "Use ImmutableMessageContent instead for zero-allocation streaming")]
pub type MessageContent = ImmutableMessageContent;

/// Deprecated alias for `ImmutableFormatOptions` - use `ImmutableFormatOptions` instead for zero-allocation streaming
#[deprecated(note = "Use ImmutableFormatOptions instead for zero-allocation streaming")]
pub type FormatOptions = ImmutableFormatOptions;

/// Deprecated alias for `ImmutableColorScheme` - use `ImmutableColorScheme` instead for zero-allocation streaming
#[deprecated(note = "Use ImmutableColorScheme instead for zero-allocation streaming")]
pub type ColorScheme = ImmutableColorScheme;

/// Deprecated alias for `ImmutableCustomFormatRule` - use `ImmutableCustomFormatRule` instead for zero-allocation streaming
#[deprecated(note = "Use ImmutableCustomFormatRule instead for zero-allocation streaming")]
pub type CustomFormatRule = ImmutableCustomFormatRule;

/// Legacy compatibility alias for `StreamingMessageFormatter`
#[deprecated(note = "Use StreamingMessageFormatter instead for zero-allocation streaming")]
pub type MessageFormatter = StreamingMessageFormatter;
