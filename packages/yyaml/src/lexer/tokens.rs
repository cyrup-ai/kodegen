//! Comprehensive token definitions for all YAML 1.2 constructs
//!
//! This module defines all possible tokens that can appear in YAML documents,
//! with zero-allocation string handling using Cow<str> for optimal performance.

use super::position::Position;
use std::borrow::Cow;

/// A token with its kind, value, and source position
#[derive(Debug, Clone, PartialEq)]
pub struct Token<'input> {
    pub kind: TokenKind<'input>,
    pub position: Position,
    pub length: usize,
}

impl<'input> Token<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(kind: TokenKind<'input>, position: Position, length: usize) -> Self {
        Self {
            kind,
            position,
            length,
        }
    }

    /// Get the end position of this token
    #[inline]
    #[must_use] 
    pub const fn end_position(&self) -> Position {
        Position::new(
            self.position.line,
            self.position.column + self.length,
            self.position.byte_offset + self.length,
        )
    }
}

/// All possible YAML token types with their associated data
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind<'input> {
    // Stream structure tokens
    StreamStart,
    StreamEnd,

    // Document structure tokens
    DocumentStart, // ---
    DocumentEnd,   // ...

    // Block structure tokens
    BlockEntry, // -
    BlockEnd,

    // Flow structure tokens
    FlowEntry,         // ,
    FlowSequenceStart, // [
    FlowSequenceEnd,   // ]
    FlowMappingStart,  // {
    FlowMappingEnd,    // }

    // Key-value tokens
    Key,   // ?
    Value, // :

    // Scalar tokens
    Scalar {
        value: Cow<'input, str>,
        style: ScalarStyle,
        tag: Option<Cow<'input, str>>,
    },

    // Anchor and alias tokens
    Anchor(Cow<'input, str>), // &anchor
    Alias(Cow<'input, str>),  // *alias

    // Tag tokens
    Tag {
        handle: Option<Cow<'input, str>>,
        suffix: Cow<'input, str>,
    },

    // Directive tokens
    YamlDirective {
        major: u32,
        minor: u32,
    },
    TagDirective {
        handle: Cow<'input, str>,
        prefix: Cow<'input, str>,
    },
    ReservedDirective {
        name: Cow<'input, str>,
        value: Cow<'input, str>,
    },

    // Whitespace and formatting
    Whitespace(Cow<'input, str>),
    LineBreak,
    Comment(Cow<'input, str>),

    // Indentation tracking
    Indent(usize),
    Dedent(usize),

    // Error recovery
    Error(Cow<'input, str>),
}

/// Scalar representation styles in YAML
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarStyle {
    /// Plain scalar (no quotes)
    Plain,
    /// Single-quoted scalar
    SingleQuoted,
    /// Double-quoted scalar  
    DoubleQuoted,
    /// Literal block scalar (|)
    Literal,
    /// Folded block scalar (>)
    Folded,
}

impl<'input> TokenKind<'input> {
    /// Check if this token can start a value
    #[inline]
    #[must_use] 
    pub const fn can_start_value(&self) -> bool {
        matches!(
            self,
            TokenKind::Scalar { .. }
                | TokenKind::FlowSequenceStart
                | TokenKind::FlowMappingStart
                | TokenKind::Anchor(_)
                | TokenKind::Tag { .. }
                | TokenKind::BlockEntry
        )
    }

    /// Check if this token ends a flow context
    #[inline]
    #[must_use] 
    pub const fn ends_flow(&self) -> bool {
        matches!(self, TokenKind::FlowSequenceEnd | TokenKind::FlowMappingEnd)
    }

    /// Check if this token is structural (affects parsing state)
    #[inline]
    #[must_use] 
    pub const fn is_structural(&self) -> bool {
        matches!(
            self,
            TokenKind::StreamStart
                | TokenKind::StreamEnd
                | TokenKind::DocumentStart
                | TokenKind::DocumentEnd
                | TokenKind::BlockEntry
                | TokenKind::BlockEnd
                | TokenKind::FlowEntry
                | TokenKind::FlowSequenceStart
                | TokenKind::FlowSequenceEnd
                | TokenKind::FlowMappingStart
                | TokenKind::FlowMappingEnd
                | TokenKind::Key
                | TokenKind::Value
                | TokenKind::Indent(_)
                | TokenKind::Dedent(_)
        )
    }

    /// Check if this token is content (carries actual data)
    #[inline]
    #[must_use] 
    pub const fn is_content(&self) -> bool {
        matches!(
            self,
            TokenKind::Scalar { .. }
                | TokenKind::Anchor(_)
                | TokenKind::Alias(_)
                | TokenKind::Tag { .. }
        )
    }

    /// Check if this token is formatting/whitespace
    #[inline]
    #[must_use] 
    pub const fn is_formatting(&self) -> bool {
        matches!(
            self,
            TokenKind::Whitespace(_) | TokenKind::LineBreak | TokenKind::Comment(_)
        )
    }

    /// Get the display name for this token type
    #[must_use] 
    pub const fn type_name(&self) -> &'static str {
        match self {
            TokenKind::StreamStart => "stream-start",
            TokenKind::StreamEnd => "stream-end",
            TokenKind::DocumentStart => "document-start",
            TokenKind::DocumentEnd => "document-end",
            TokenKind::BlockEntry => "block-entry",
            TokenKind::BlockEnd => "block-end",
            TokenKind::FlowEntry => "flow-entry",
            TokenKind::FlowSequenceStart => "flow-sequence-start",
            TokenKind::FlowSequenceEnd => "flow-sequence-end",
            TokenKind::FlowMappingStart => "flow-mapping-start",
            TokenKind::FlowMappingEnd => "flow-mapping-end",
            TokenKind::Key => "key",
            TokenKind::Value => "value",
            TokenKind::Scalar { .. } => "scalar",
            TokenKind::Anchor(_) => "anchor",
            TokenKind::Alias(_) => "alias",
            TokenKind::Tag { .. } => "tag",
            TokenKind::YamlDirective { .. } => "yaml-directive",
            TokenKind::TagDirective { .. } => "tag-directive",
            TokenKind::ReservedDirective { .. } => "reserved-directive",
            TokenKind::Whitespace(_) => "whitespace",
            TokenKind::LineBreak => "line-break",
            TokenKind::Comment(_) => "comment",
            TokenKind::Indent(_) => "indent",
            TokenKind::Dedent(_) => "dedent",
            TokenKind::Error(_) => "error",
        }
    }
}

impl std::fmt::Display for ScalarStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plain => write!(f, "plain"),
            Self::SingleQuoted => write!(f, "single-quoted"),
            Self::DoubleQuoted => write!(f, "double-quoted"),
            Self::Literal => write!(f, "literal"),
            Self::Folded => write!(f, "folded"),
        }
    }
}

/// Token precedence for parsing disambiguation
impl<'input> TokenKind<'input> {
    /// Get the precedence of this token for parsing decisions
    #[inline]
    #[must_use] 
    pub const fn precedence(&self) -> u8 {
        match self {
            TokenKind::StreamStart | TokenKind::StreamEnd => 0,
            TokenKind::DocumentStart | TokenKind::DocumentEnd => 1,
            TokenKind::YamlDirective { .. }
            | TokenKind::TagDirective { .. }
            | TokenKind::ReservedDirective { .. } => 2,
            TokenKind::BlockEntry | TokenKind::BlockEnd => 3,
            TokenKind::FlowSequenceStart | TokenKind::FlowSequenceEnd => 4,
            TokenKind::FlowMappingStart | TokenKind::FlowMappingEnd => 4,
            TokenKind::Key => 5,
            TokenKind::Value => 6,
            TokenKind::FlowEntry => 7,
            TokenKind::Tag { .. } => 8,
            TokenKind::Anchor(_) => 9,
            TokenKind::Alias(_) => 10,
            TokenKind::Scalar { .. } => 11,
            TokenKind::Whitespace(_) | TokenKind::LineBreak | TokenKind::Comment(_) => 20,
            TokenKind::Indent(_) | TokenKind::Dedent(_) => 21,
            TokenKind::Error(_) => 255,
        }
    }
}
