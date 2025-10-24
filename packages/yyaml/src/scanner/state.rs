//! Scanner state management with zero-allocation design
//!
//! This module provides efficient state tracking for the YAML scanner including
//! position tracking, buffer management, and flow context handling.

use crate::error::{Marker, ScanError};
use crate::scanner::token::Token;
use std::collections::VecDeque;

/// Context tracking for BOM filtering - preserves BOMs in quoted strings per YAML 1.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotedContext {
    /// Outside any quoted string - BOMs filtered per YAML 1.2 spec
    None,
    /// Inside single-quoted string - BOMs preserved as literal content
    Single,
    /// Inside double-quoted string - BOMs preserved as literal content
    Double,
}

impl Default for QuotedContext {
    #[inline]
    fn default() -> Self {
        Self::None
    }
}

/// Scanner configuration for customizable behavior
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    /// Maximum recursion depth for nested structures
    pub max_depth: usize,
    /// Initial buffer capacity for performance
    pub initial_buffer_capacity: usize,
    /// Maximum anchor/alias name length
    pub max_anchor_length: usize,
    /// Enable strict YAML 1.2 compliance
    pub strict_yaml12: bool,
    /// Allow duplicate anchors (non-standard)
    pub allow_duplicate_anchors: bool,
}

impl Default for ScannerConfig {
    #[inline]
    fn default() -> Self {
        Self {
            max_depth: 1024,
            initial_buffer_capacity: 64,
            max_anchor_length: 1024,
            strict_yaml12: true,
            allow_duplicate_anchors: false,
        }
    }
}

/// Scanner state with efficient buffer and position management
pub struct ScannerState<T: Iterator<Item = char>> {
    /// Character source iterator
    source: T,
    /// Lookahead buffer for peeking
    buffer: VecDeque<char>,
    /// Current position marker
    mark: Marker,
    /// Source exhausted flag
    done: bool,
    /// Cached token for peeking
    cached_token: Option<Token>,
    /// Stream start produced flag
    stream_start_produced: bool,
    /// Stream end produced flag
    stream_end_produced: bool,
    /// Flow nesting level for context tracking
    flow_level: usize,
    /// Current indentation level
    indent: i32,
    /// Indentation stack for block context
    indent_stack: Vec<i32>,
    /// Simple key allowed flag
    simple_key_allowed: bool,
    /// BOM filtering context for YAML 1.2 compliance
    quoted_context: QuotedContext,
}

impl<T: Iterator<Item = char>> ScannerState<T> {
    /// Create new scanner state with optimized defaults
    #[inline]
    pub fn new(source: T) -> Self {
        Self {
            source,
            buffer: VecDeque::with_capacity(64),
            mark: Marker::new(),
            done: false,
            cached_token: None,
            stream_start_produced: false,
            stream_end_produced: false,
            flow_level: 0,
            indent: -1,
            indent_stack: Vec::with_capacity(16),
            simple_key_allowed: true,
            quoted_context: QuotedContext::None,
        }
    }

    /// Get current position marker
    #[inline]
    pub const fn mark(&self) -> Marker {
        self.mark
    }

    /// Check if source is exhausted
    #[inline]
    pub fn is_done(&self) -> bool {
        self.done && self.buffer.is_empty()
    }

    /// Check if stream start was produced
    #[inline]
    pub const fn stream_started(&self) -> bool {
        self.stream_start_produced
    }

    /// Check if stream end was produced
    #[inline]
    pub const fn stream_ended(&self) -> bool {
        self.stream_end_produced
    }

    /// Mark stream as started
    #[inline]
    pub const fn mark_stream_started(&mut self) {
        self.stream_start_produced = true;
    }

    /// Mark stream as ended
    #[inline]
    pub const fn mark_stream_ended(&mut self) {
        self.stream_end_produced = true;
    }

    /// Check if we're in flow context
    #[inline]
    pub const fn in_flow_context(&self) -> bool {
        self.flow_level > 0
    }

    /// Enter flow context (for [ ] or { })
    #[inline]
    pub const fn enter_flow_context(&mut self) {
        self.flow_level += 1;
        self.simple_key_allowed = true;
    }

    /// Exit flow context with validation
    #[inline]
    pub fn exit_flow_context(&mut self) -> Result<(), ScanError> {
        if self.flow_level == 0 {
            return Err(ScanError::new(self.mark, "unexpected flow collection end"));
        }
        self.flow_level -= 1;
        Ok(())
    }

    /// Get current flow level
    #[inline]
    pub const fn flow_level(&self) -> usize {
        self.flow_level
    }

    /// Check if simple key is allowed
    #[inline]
    pub const fn simple_key_allowed(&self) -> bool {
        self.simple_key_allowed
    }

    /// Set simple key allowed flag
    #[inline]
    pub const fn set_simple_key_allowed(&mut self, allowed: bool) {
        self.simple_key_allowed = allowed;
    }

    /// Get current indentation
    #[inline]
    pub const fn indent(&self) -> i32 {
        self.indent
    }

    /// Push indentation level
    #[inline]
    pub fn push_indent(&mut self, indent: i32) {
        self.indent_stack.push(self.indent);
        self.indent = indent;
    }

    /// Pop indentation level
    #[inline]
    pub fn pop_indent(&mut self) -> Option<i32> {
        if let Some(prev) = self.indent_stack.pop() {
            let current = self.indent;
            self.indent = prev;
            Some(current)
        } else {
            None
        }
    }

    /// Check if has cached token
    #[inline]
    pub const fn has_cached_token(&self) -> bool {
        self.cached_token.is_some()
    }

    /// Cache a token for peeking
    #[inline]
    pub fn cache_token(&mut self, token: Token) {
        self.cached_token = Some(token);
    }

    /// Peek at cached token
    #[inline]
    pub const fn peek_cached_token(&self) -> Option<&Token> {
        self.cached_token.as_ref()
    }

    /// Take cached token
    #[inline]
    pub const fn take_cached_token(&mut self) -> Option<Token> {
        self.cached_token.take()
    }

    /// Clear cached token
    #[inline]
    pub fn clear_cached_token(&mut self) {
        self.cached_token = None;
    }

    /// Get current quoted context for BOM filtering
    #[inline]
    pub const fn quoted_context(&self) -> QuotedContext {
        self.quoted_context
    }

    /// Set quoted context for BOM filtering (used by quoted string scanners)
    #[inline]
    pub const fn set_quoted_context(&mut self, context: QuotedContext) {
        self.quoted_context = context;
    }

    /// Check if currently inside quoted string context
    #[inline]
    pub fn in_quoted_context(&self) -> bool {
        self.quoted_context != QuotedContext::None
    }

    /// Fill buffer to ensure at least n characters
    #[inline]
    pub fn ensure_buffer(&mut self, n: usize) {
        while self.buffer.len() < n && !self.done {
            if let Some(ch) = self.source.next() {
                self.buffer.push_back(ch);
            } else {
                self.done = true;
            }
        }
    }

    /// Peek at next character without consuming (raw - no BOM filtering)
    #[inline]
    pub fn peek_char_raw(&mut self) -> Result<char, ScanError> {
        self.ensure_buffer(1);
        self.buffer
            .front()
            .copied()
            .ok_or_else(|| ScanError::new(self.mark, "unexpected end of input"))
    }

    /// Peek at next character (no BOM filtering - handled at document level)
    #[inline(always)]
    pub fn peek_char(&mut self) -> Result<char, ScanError> {
        self.peek_char_raw()
    }

    /// Peek at character at offset without consuming (raw - no BOM filtering)
    #[inline]
    pub fn peek_char_at_raw(&mut self, offset: usize) -> Option<char> {
        self.ensure_buffer(offset + 1);
        self.buffer.get(offset).copied()
    }

    /// Peek at character at offset (no BOM filtering - handled at document level)
    #[inline]
    pub fn peek_char_at(&mut self, offset: usize) -> Option<char> {
        self.peek_char_at_raw(offset)
    }

    /// Check if next characters match a pattern (no BOM filtering)
    #[inline]
    pub fn check_chars(&mut self, pattern: &[char]) -> bool {
        for (i, &expected_char) in pattern.iter().enumerate() {
            match self.peek_char_at(i) {
                Some(actual_char) if actual_char == expected_char => continue,
                _ => return false,
            }
        }
        true
    }

    /// Consume next character and update position (raw - no BOM filtering)
    #[inline]
    pub fn consume_char_raw(&mut self) -> Result<char, ScanError> {
        if let Some(ch) = self.buffer.pop_front() {
            self.mark.index += 1;
            if ch == '\n' {
                self.mark.line += 1;
                self.mark.col = 0;
            } else {
                self.mark.col += 1;
            }
            Ok(ch)
        } else {
            Err(ScanError::new(self.mark, "unexpected end of input"))
        }
    }

    /// Consume next character (no BOM filtering - handled at document level)
    #[inline(always)]
    pub fn consume_char(&mut self) -> Result<char, ScanError> {
        self.consume_char_raw()
    }

    /// Consume multiple characters
    #[inline]
    pub fn consume_chars(&mut self, n: usize) -> Result<(), ScanError> {
        for _ in 0..n {
            self.consume_char()?;
        }
        Ok(())
    }

    /// Check for document start marker (---)
    #[inline]
    pub fn check_document_start(&mut self) -> Result<bool, ScanError> {
        Ok(self.check_chars(&['-', '-', '-']) && self.check_boundary_after(3))
    }

    /// Check for document end marker (...)
    #[inline]
    pub fn check_document_end(&mut self) -> Result<bool, ScanError> {
        Ok(self.check_chars(&['.', '.', '.']) && self.check_boundary_after(3))
    }

    /// Check for block entry (- followed by space/newline)
    #[inline]
    pub fn check_block_entry(&mut self) -> Result<bool, ScanError> {
        match self.peek_char_at(0) {
            Some('-') => {
                match self.peek_char_at(1) {
                    Some(' ') | Some('\t') | Some('\n') | Some('\r') => Ok(true),
                    None => Ok(true), // EOF after -
                    _ => Ok(false),
                }
            }
            _ => Ok(false),
        }
    }

    /// Check if character at offset is a boundary (space, newline, EOF, etc)
    #[inline]
    fn check_boundary_after(&mut self, offset: usize) -> bool {
        match self.peek_char_at(offset) {
            Some(ch) => matches!(ch, ' ' | '\t' | '\n' | '\r' | ',' | ']' | '}'),
            None => true, // EOF is a boundary
        }
    }

    /// Peek ahead multiple characters as slice
    #[inline]
    pub fn peek_slice(&mut self, n: usize) -> Vec<char> {
        self.ensure_buffer(n);
        self.buffer.iter().take(n).copied().collect()
    }

    /// Check if at beginning of line
    #[inline]
    pub const fn at_line_start(&self) -> bool {
        self.mark.col == 0
    }

    /// Get current column position
    #[inline]
    pub const fn column(&self) -> usize {
        self.mark.col
    }

    /// Create scanner state from lexer iterator for block scalar processing
    pub fn from_lexer_input(input: T) -> Self {
        Self::new(input)
    }

    /// Extract remaining input after block scalar processing
    pub fn into_remaining_input(self) -> T {
        self.source
    }
}

/// Marker extensions for efficient position tracking
impl Marker {
    /// Create default marker at start of stream
    #[inline]
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            index: 0,
            line: 1,
            col: 0,
        }
    }

    /// Create marker at specific position
    #[inline]
    #[must_use] 
    pub const fn at(index: usize, line: usize, col: usize) -> Self {
        Self { index, line, col }
    }
}
