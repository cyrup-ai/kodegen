//! Token representation and production with zero-allocation optimizations
//!
//! This module provides efficient token creation and management for the YAML scanner
//! with specialized builders for different token types.

use crate::error::Marker;
use crate::events::{TEncoding, TScalarStyle, TokenType};

/// A token with position information
#[derive(Clone, Debug)]
pub struct Token(pub Marker, pub TokenType);

impl Token {
    /// Create new token
    #[inline]
    #[must_use] 
    pub const fn new(marker: Marker, token_type: TokenType) -> Self {
        Self(marker, token_type)
    }

    /// Get token position
    #[inline]
    #[must_use] 
    pub const fn position(&self) -> Marker {
        self.0
    }

    /// Get token type
    #[inline]
    #[must_use] 
    pub const fn token_type(&self) -> &TokenType {
        &self.1
    }

    /// Check if token is of specific type
    #[inline]
    #[must_use] 
    pub fn is_type(&self, expected: &TokenType) -> bool {
        std::mem::discriminant(&self.1) == std::mem::discriminant(expected)
    }
}

/// Efficient token producer with optimized constructors
pub struct TokenProducer {
    // Cache for common tokens to avoid allocations
    encoding: TEncoding,
}

impl TokenProducer {
    /// Create new token producer
    #[inline]
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            encoding: TEncoding::Utf8,
        }
    }

    /// Reset producer state
    #[inline]
    pub const fn reset(&mut self) {
        self.encoding = TEncoding::Utf8;
    }

    /// Set encoding for stream
    #[inline]
    pub const fn set_encoding(&mut self, encoding: TEncoding) {
        self.encoding = encoding;
    }

    // Stream tokens

    #[inline]
    #[must_use] 
    pub const fn stream_start_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::StreamStart(self.encoding))
    }

    #[inline]
    #[must_use] 
    pub const fn stream_end_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::StreamEnd)
    }

    #[inline]
    #[must_use] 
    pub const fn no_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::NoToken)
    }

    // Document tokens

    #[inline]
    #[must_use] 
    pub const fn document_start_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::DocumentStart)
    }

    #[inline]
    #[must_use] 
    pub const fn document_end_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::DocumentEnd)
    }

    // Block tokens

    #[inline]
    #[must_use] 
    pub const fn block_sequence_start_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::BlockSequenceStart)
    }

    #[inline]
    #[must_use] 
    pub const fn block_mapping_start_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::BlockMappingStart)
    }

    #[inline]
    #[must_use] 
    pub const fn block_end_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::BlockEnd)
    }

    #[inline]
    #[must_use] 
    pub const fn block_entry_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::BlockEntry)
    }

    // Flow tokens

    #[inline]
    #[must_use] 
    pub const fn flow_sequence_start_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::FlowSequenceStart)
    }

    #[inline]
    #[must_use] 
    pub const fn flow_sequence_end_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::FlowSequenceEnd)
    }

    #[inline]
    #[must_use] 
    pub const fn flow_mapping_start_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::FlowMappingStart)
    }

    #[inline]
    #[must_use] 
    pub const fn flow_mapping_end_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::FlowMappingEnd)
    }

    #[inline]
    #[must_use] 
    pub const fn flow_entry_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::FlowEntry)
    }

    // Key/Value tokens

    #[inline]
    #[must_use] 
    pub const fn key_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::Key)
    }

    #[inline]
    #[must_use] 
    pub const fn value_token(&self, marker: Marker) -> Token {
        Token::new(marker, TokenType::Value)
    }

    // Scalar tokens with zero-copy optimization

    #[inline]
    #[must_use] 
    pub const fn plain_scalar_token(&self, marker: Marker, value: String) -> Token {
        Token::new(marker, TokenType::Scalar(TScalarStyle::Plain, value))
    }

    #[inline]
    #[must_use] 
    pub const fn single_quoted_scalar_token(&self, marker: Marker, value: String) -> Token {
        Token::new(marker, TokenType::Scalar(TScalarStyle::SingleQuoted, value))
    }

    #[inline]
    #[must_use] 
    pub const fn double_quoted_scalar_token(&self, marker: Marker, value: String) -> Token {
        Token::new(marker, TokenType::Scalar(TScalarStyle::DoubleQuoted, value))
    }

    #[inline]
    #[must_use] 
    pub const fn literal_scalar_token(&self, marker: Marker, value: String) -> Token {
        Token::new(marker, TokenType::Scalar(TScalarStyle::Literal, value))
    }

    #[inline]
    #[must_use] 
    pub const fn folded_scalar_token(&self, marker: Marker, value: String) -> Token {
        Token::new(marker, TokenType::Scalar(TScalarStyle::Folded, value))
    }

    // Anchor/Alias tokens

    #[inline]
    #[must_use] 
    pub const fn anchor_token(&self, marker: Marker, name: String) -> Token {
        Token::new(marker, TokenType::Anchor(name))
    }

    #[inline]
    #[must_use] 
    pub const fn alias_token(&self, marker: Marker, name: String) -> Token {
        Token::new(marker, TokenType::Alias(name))
    }

    // Tag tokens

    #[inline]
    #[must_use] 
    pub const fn tag_token(&self, marker: Marker, handle: String, suffix: String) -> Token {
        Token::new(marker, TokenType::Tag(handle, suffix))
    }

    // Directive tokens

    #[inline]
    #[must_use] 
    pub const fn version_directive_token(&self, marker: Marker, major: u32, minor: u32) -> Token {
        Token::new(marker, TokenType::VersionDirective(major, minor))
    }

    #[inline]
    #[must_use] 
    pub const fn tag_directive_token(&self, marker: Marker, handle: String, prefix: String) -> Token {
        Token::new(marker, TokenType::TagDirective(handle, prefix))
    }

    #[inline]
    #[must_use] 
    pub const fn reserved_directive_token(&self, marker: Marker, name: String) -> Token {
        Token::new(marker, TokenType::Reserved(name))
    }

    #[inline]
    #[must_use] 
    pub fn directive_token(
        &self,
        marker: Marker,
        directive: crate::scanner::directives::Directive,
    ) -> Token {
        use crate::scanner::directives::Directive;
        match directive {
            Directive::Version { major, minor } => {
                self.version_directive_token(marker, major, minor)
            }
            Directive::Tag { handle, prefix } => self.tag_directive_token(marker, handle, prefix),
            Directive::Reserved { name, .. } => self.reserved_directive_token(marker, name),
        }
    }
}

impl Default for TokenProducer {
    fn default() -> Self {
        Self::new()
    }
}

/// Token stream for iteration support
pub struct TokenStream<'a> {
    tokens: std::slice::Iter<'a, Token>,
}

impl<'a> TokenStream<'a> {
    /// Create new token stream from slice
    #[inline]
    #[must_use] 
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens: tokens.iter(),
        }
    }

    /// Peek at next token without consuming
    #[inline]
    pub fn peek(&mut self) -> Option<&'a Token> {
        self.tokens.as_slice().first()
    }

    /// Check if stream is empty
    #[inline]
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.tokens.as_slice().is_empty()
    }

    /// Get remaining token count
    #[inline]
    #[must_use] 
    pub fn len(&self) -> usize {
        self.tokens.as_slice().len()
    }
}

impl<'a> Iterator for TokenStream<'a> {
    type Item = &'a Token;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.tokens.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.tokens.size_hint()
    }
}

impl<'a> ExactSizeIterator for TokenStream<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.tokens.len()
    }
}
