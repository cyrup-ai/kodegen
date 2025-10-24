//! Comprehensive lexical analysis module for YAML 1.2 tokenization
//!
//! This module provides a complete tokenization system with zero-allocation design
//! and precise source location tracking for production-quality YAML parsing.

pub mod position;
pub mod scanner;
pub mod tokens;
pub mod unicode;

pub use position::*;
pub use scanner::*;
pub use tokens::*;

/// High-performance lexer interface for YAML tokenization
#[derive(Debug, Clone)]
pub struct YamlLexer<'input> {
    scanner: Scanner<'input>,
    position: PositionTracker,
}

impl<'input> YamlLexer<'input> {
    /// Create a new lexer for the given input
    #[inline]
    #[must_use] 
    pub fn new(input: &'input str) -> Self {
        Self {
            scanner: Scanner::new(input),
            position: PositionTracker::new(),
        }
    }

    /// Get the next token from the input
    #[inline]
    pub fn next_token(&mut self) -> Result<Token<'input>, LexError> {
        self.scanner.scan_token(&mut self.position)
    }

    /// Peek at the next token without consuming it
    #[inline]
    pub fn peek_token(&mut self) -> Result<Token<'input>, LexError> {
        self.scanner.peek_token(&mut self.position)
    }

    /// Get the current position in the input
    #[inline]
    #[must_use] 
    pub const fn position(&self) -> Position {
        self.position.current()
    }

    /// Check if we've reached the end of input
    #[inline]
    pub fn is_at_end(&mut self) -> bool {
        self.scanner.is_at_end()
    }

    /// Create an iterator over all tokens
    #[must_use] 
    pub const fn tokens(self) -> TokenIterator<'input> {
        TokenIterator::new(self)
    }
}

/// Iterator over tokens produced by the lexer
pub struct TokenIterator<'input> {
    lexer: YamlLexer<'input>,
    finished: bool,
}

impl<'input> TokenIterator<'input> {
    #[inline]
    const fn new(lexer: YamlLexer<'input>) -> Self {
        Self {
            lexer,
            finished: false,
        }
    }
}

impl<'input> Iterator for TokenIterator<'input> {
    type Item = Result<Token<'input>, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.lexer.next_token() {
            Ok(token) => {
                if matches!(token.kind, TokenKind::StreamEnd) {
                    self.finished = true;
                }
                Some(Ok(token))
            }
            Err(e) => {
                self.finished = true;
                Some(Err(e))
            }
        }
    }
}

/// Lexical error type with precise source location
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub position: Position,
}

impl LexError {
    #[inline]
    #[must_use] 
    pub const fn new(kind: LexErrorKind, position: Position) -> Self {
        Self { kind, position }
    }
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.kind, self.position.line, self.position.column
        )
    }
}

impl std::error::Error for LexError {}

/// Types of lexical errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    UnexpectedCharacter(String),
    InvalidEscape(String),
    UnterminatedString,
    InvalidUnicode,
    InvalidUnicodeEscape,
    InvalidNumber,
    InvalidTag(String),
    InvalidAnchor(String),
    InvalidAlias(String),
    InvalidDirective(String),
    InvalidIndentation(String),
    UnexpectedEndOfInput,
    EmptyScalar,
}

impl std::fmt::Display for LexErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedCharacter(msg) => write!(f, "unexpected character: {msg}"),
            Self::InvalidEscape(msg) => write!(f, "invalid escape sequence: {msg}"),
            Self::UnterminatedString => write!(f, "unterminated string"),
            Self::InvalidUnicode => write!(f, "invalid unicode sequence"),
            Self::InvalidUnicodeEscape => write!(f, "invalid unicode escape"),
            Self::InvalidNumber => write!(f, "invalid number format"),
            Self::InvalidTag(msg) => write!(f, "invalid tag: {msg}"),
            Self::InvalidAnchor(msg) => write!(f, "invalid anchor: {msg}"),
            Self::InvalidAlias(msg) => write!(f, "invalid alias: {msg}"),
            Self::InvalidDirective(msg) => write!(f, "invalid directive: {msg}"),
            Self::InvalidIndentation(msg) => write!(f, "invalid indentation: {msg}"),
            Self::UnexpectedEndOfInput => write!(f, "unexpected end of input"),
            Self::EmptyScalar => write!(f, "empty scalar"),
        }
    }
}
