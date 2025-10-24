//! Immutable message formatting with streaming operations
//!
//! Provides zero-allocation, lock-free message formatting with streaming operations.
//! All Arc usage eliminated in favor of owned strings and borrowed data patterns
//! for blazing-fast performance with immutable formatting structures.

// Declare submodules
pub mod compat;
pub mod content;
pub mod error;
pub mod events;
pub mod formatter;
pub mod options;

// Re-export all public types to maintain API compatibility
pub use content::{FormatStyle, ImmutableMessageContent, StyleType};

pub use error::{FormatError, FormatResult};

pub use events::FormattingEvent;

pub use formatter::{FormatterStats, FormatterWithStream, StreamingMessageFormatter};

pub use options::{
    FormatFlags, ImmutableColorScheme, ImmutableCustomFormatRule, ImmutableFormatOptions,
    OutputFormat, SyntaxTheme,
};
