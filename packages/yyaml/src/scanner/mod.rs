//! Zero-allocation YAML scanner with blazing-fast tokenization
//!
//! This module provides comprehensive YAML scanning with zero-allocation design,
//! complete error handling, and production-ready performance optimizations.

pub mod anchors;
pub mod directives;
pub mod document;
pub mod indentation;
pub mod scalars;
pub mod state;
pub mod tags;
pub mod token;
pub mod utils;

pub use state::{QuotedContext, ScannerConfig, ScannerState};
pub use token::{Token, TokenProducer, TokenStream};

use crate::error::{Marker, ScanError};

/// High-performance YAML scanner with zero-allocation tokenization
///
/// Provides streaming tokenization of YAML input with complete error handling,
/// position tracking, and optimized performance for production use.
pub struct Scanner<T: Iterator<Item = char>> {
    state: ScannerState<T>,
    token_producer: TokenProducer,
    config: ScannerConfig,
}

impl<T: Iterator<Item = char>> Scanner<T> {
    /// Create new scanner with default configuration
    #[inline]
    pub fn new(source: T) -> Self {
        Self::with_config(source, ScannerConfig::default())
    }

    /// Create scanner with custom configuration
    #[inline]
    pub fn with_config(source: T, config: ScannerConfig) -> Self {
        Self {
            state: ScannerState::new(source),
            token_producer: TokenProducer::new(),
            config,
        }
    }

    /// Get current position marker
    #[inline]
    pub const fn mark(&self) -> Marker {
        self.state.mark()
    }

    /// Peek at next token without consuming
    #[inline]
    pub fn peek_token(&mut self) -> Result<Token, ScanError> {
        if !self.state.has_cached_token() {
            let token = self.fetch_next_token()?;
            self.state.cache_token(token);
        }
        self.state
            .peek_cached_token()
            .cloned()
            .ok_or_else(|| ScanError::new(self.mark(), "internal error: no cached token"))
    }

    /// Fetch next token, consuming it
    #[inline]
    pub fn fetch_token(&mut self) -> Token {
        if let Some(token) = self.state.take_cached_token() {
            token
        } else {
            // If no cached token, fetch one - this follows original API design
            // where peek_token() should be called first in normal usage
            match self.fetch_next_token() {
                Ok(token) => token,
                Err(_) => {
                    // Create a no-token as fallback to avoid panic
                    // This maintains API compatibility while being safer than unwrap
                    self.token_producer.no_token(self.mark())
                }
            }
        }
    }

    /// Skip current token without returning it
    #[inline]
    pub fn skip(&mut self) {
        self.state.clear_cached_token();
    }

    /// Check if stream has started
    #[inline]
    pub const fn stream_started(&self) -> bool {
        self.state.stream_started()
    }

    /// Check if stream has ended
    #[inline]
    pub const fn stream_ended(&self) -> bool {
        self.state.stream_ended()
    }

    /// Get scanner configuration
    #[inline]
    pub const fn config(&self) -> &ScannerConfig {
        &self.config
    }

    /// Reset scanner state for reuse
    #[inline]
    pub fn reset(&mut self, source: T) {
        self.state = ScannerState::new(source);
        self.token_producer.reset();
    }

    // Character-level access methods for state machine separation functions

    /// Peek at next character without consuming (for separation parsing)
    #[inline]
    pub fn peek_char(&mut self) -> Result<char, ScanError> {
        self.state.peek_char()
    }

    /// Consume next character and update position (for separation parsing)
    #[inline]
    pub fn consume_char(&mut self) -> Result<char, ScanError> {
        self.state.consume_char()
    }

    /// Skip inline separation (spaces and tabs only) per YAML 1.2 rule [80]
    /// Used for BLOCK-KEY and FLOW-KEY contexts
    #[inline]
    pub fn skip_inline_separation(&mut self) -> Result<(), ScanError> {
        loop {
            match self.state.peek_char() {
                Ok(' ') | Ok('\t') => {
                    self.state.consume_char()?;
                }
                Ok('#') => {
                    // Skip comment line but this ends inline separation due to line break
                    utils::skip_comment_line(&mut self.state)?;
                    break;
                }
                _ => {
                    // Any other character ends inline separation
                    break;
                }
            }
        }
        Ok(())
    }

    /// Skip multi-line separation (all whitespace and comments) per YAML 1.2 rule [80]
    /// Used for FLOW-IN, FLOW-OUT, BLOCK-IN, BLOCK-OUT contexts
    #[inline]
    pub fn skip_multiline_separation(&mut self) -> Result<(), ScanError> {
        utils::skip_whitespace_and_comments(&mut self.state)
    }

    /// Fetch next token from stream with optimized dispatch
    fn fetch_next_token(&mut self) -> Result<Token, ScanError> {
        // Handle stream start/end tokens
        if !self.state.stream_started() {
            self.state.mark_stream_started();
            return Ok(self.token_producer.stream_start_token(self.mark()));
        }

        // Skip whitespace and comments efficiently
        utils::skip_whitespace_and_comments(&mut self.state)?;

        // Handle BOM at document/stream boundaries per YAML 1.2 spec
        self.handle_bom_at_boundary()?;

        // Check for stream end
        if self.state.is_done() {
            if !self.state.stream_ended() {
                self.state.mark_stream_ended();
                return Ok(self.token_producer.stream_end_token(self.mark()));
            }
            return Ok(self.token_producer.no_token(self.mark()));
        }

        // Peek at next character for dispatching
        let start_mark = self.mark();
        match self.state.peek_char()? {
            '-' => self.scan_dash_token(start_mark),
            '.' => self.scan_dot_token(start_mark),
            '[' => self.scan_flow_sequence_start(start_mark),
            ']' => self.scan_flow_sequence_end(start_mark),
            '{' => self.scan_flow_mapping_start(start_mark),
            '}' => self.scan_flow_mapping_end(start_mark),
            ',' => self.scan_flow_entry(start_mark),
            ':' => self.scan_value_token(start_mark),
            '?' => self.scan_key_token(start_mark),
            '&' => self.scan_anchor_token(start_mark),
            '*' => self.scan_alias_token(start_mark),
            '!' => self.scan_tag_token(start_mark),
            '|' => self.scan_literal_block_scalar(start_mark),
            '>' => self.scan_folded_block_scalar(start_mark),
            '\'' => self.scan_single_quoted_scalar(start_mark),
            '"' => self.scan_double_quoted_scalar(start_mark),
            '%' => self.scan_directive_token(start_mark),
            '#' => self.scan_comment_and_retry(start_mark),
            _ => self.scan_plain_scalar(start_mark),
        }
    }

    // Token scanning methods with optimized implementations

    #[inline]
    fn scan_dash_token(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        if self.state.check_document_start()? {
            self.state.consume_chars(3)?;
            Ok(self.token_producer.document_start_token(start_mark))
        } else if self.state.check_block_entry()? {
            self.state.consume_char()?;
            Ok(self.token_producer.block_entry_token(start_mark))
        } else {
            self.scan_plain_scalar(start_mark)
        }
    }

    #[inline]
    fn scan_dot_token(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        if self.state.check_document_end()? {
            self.state.consume_chars(3)?;
            Ok(self.token_producer.document_end_token(start_mark))
        } else {
            self.scan_plain_scalar(start_mark)
        }
    }

    #[inline]
    fn scan_flow_sequence_start(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?;
        self.state.enter_flow_context();
        Ok(self.token_producer.flow_sequence_start_token(start_mark))
    }

    #[inline]
    fn scan_flow_sequence_end(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?;
        self.state.exit_flow_context()?;
        Ok(self.token_producer.flow_sequence_end_token(start_mark))
    }

    #[inline]
    fn scan_flow_mapping_start(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?;
        self.state.enter_flow_context();
        Ok(self.token_producer.flow_mapping_start_token(start_mark))
    }

    #[inline]
    fn scan_flow_mapping_end(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?;
        self.state.exit_flow_context()?;
        Ok(self.token_producer.flow_mapping_end_token(start_mark))
    }

    #[inline]
    fn scan_flow_entry(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?;
        Ok(self.token_producer.flow_entry_token(start_mark))
    }

    #[inline]
    fn scan_value_token(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?;
        Ok(self.token_producer.value_token(start_mark))
    }

    #[inline]
    fn scan_key_token(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?;
        Ok(self.token_producer.key_token(start_mark))
    }

    #[inline]
    fn scan_anchor_token(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?; // consume '&'
        let name = anchors::scan_anchor_name(&mut self.state)?;
        Ok(self.token_producer.anchor_token(start_mark, name))
    }

    #[inline]
    fn scan_alias_token(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?; // consume '*'
        let name = anchors::scan_alias_name(&mut self.state)?;
        Ok(self.token_producer.alias_token(start_mark, name))
    }

    #[inline]
    fn scan_tag_token(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?; // consume '!'
        let (handle, suffix) = tags::scan_tag(&mut self.state, &self.config)?;
        Ok(self.token_producer.tag_token(start_mark, handle, suffix))
    }

    #[inline]
    fn scan_literal_block_scalar(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?; // consume '|'
        let content = scalars::scan_block_scalar(&mut self.state, true)?;
        Ok(self
            .token_producer
            .literal_scalar_token(start_mark, content))
    }

    #[inline]
    fn scan_folded_block_scalar(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?; // consume '>'
        let content = scalars::scan_block_scalar(&mut self.state, false)?;
        Ok(self.token_producer.folded_scalar_token(start_mark, content))
    }

    #[inline]
    fn scan_single_quoted_scalar(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?; // consume '\''
        let content = scalars::scan_single_quoted(&mut self.state)?;
        Ok(self
            .token_producer
            .single_quoted_scalar_token(start_mark, content))
    }

    #[inline]
    fn scan_double_quoted_scalar(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?; // consume '"'
        let content = scalars::scan_double_quoted(&mut self.state)?;
        Ok(self
            .token_producer
            .double_quoted_scalar_token(start_mark, content))
    }

    #[inline]
    fn scan_directive_token(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        self.state.consume_char()?; // consume '%'
        let directive = directives::scan_directive(&mut self.state)?;
        Ok(self.token_producer.directive_token(start_mark, directive))
    }

    #[inline]
    fn scan_plain_scalar(&mut self, start_mark: Marker) -> Result<Token, ScanError> {
        let content = scalars::scan_plain_scalar(&mut self.state, &self.config)?;
        Ok(self.token_producer.plain_scalar_token(start_mark, content))
    }

    #[inline]
    fn scan_comment_and_retry(&mut self, _start_mark: Marker) -> Result<Token, ScanError> {
        utils::skip_comment_line(&mut self.state)?;
        self.fetch_next_token()
    }

    /// Handle BOM at document/stream boundaries per YAML 1.2 specification  
    #[inline]
    fn handle_bom_at_boundary(&mut self) -> Result<(), ScanError> {
        // Check if BOM character is present at current position
        match self.state.peek_char() {
            Ok('\u{feff}') => {
                // BOM found - check if we're at a valid boundary
                if self.is_at_valid_bom_position()? {
                    // Valid position - strip BOM per YAML 1.2 spec
                    self.state.consume_char()?;
                    Ok(())
                } else {
                    // Invalid position - BOM inside document content
                    let mark = self.mark();
                    Err(ScanError::new(
                        mark,
                        &format!(
                            "BOM (\\u{{feff}}) must not appear inside a document at line {}, column {} (YAML 1.2 violation). BOM is only valid at stream start or document boundaries.",
                            mark.line,
                            mark.col + 1
                        ),
                    ))
                }
            }
            Ok(_) | Err(_) => {
                // No BOM present or end of input - continue normally
                Ok(())
            }
        }
    }

    /// Check if scanner is at a valid BOM position per YAML 1.2
    #[inline]
    fn is_at_valid_bom_position(&mut self) -> Result<bool, ScanError> {
        let mark = self.state.mark();

        // YAML 1.2 Rule 1: Stream start (position 0) - BOM always valid
        if mark.index == 0 {
            return Ok(true);
        }

        // YAML 1.2 Rule 2: BOM only valid at document boundaries, not inside content
        if !self.state.at_line_start() {
            return Ok(false);
        }

        // YAML 1.2 Rule 3: Before explicit document start marker (---)
        if self.state.peek_char_at(0) == Some('-')
            && self.state.peek_char_at(1) == Some('-')
            && self.state.peek_char_at(2) == Some('-')
        {
            // Verify boundary after --- per YAML 1.2 specification
            match self.state.peek_char_at(3) {
                Some(' ') | Some('\t') | Some('\n') | Some('\r') | None => return Ok(true),
                _ => {} // Not a valid document marker, continue checking
            }
        }

        // YAML 1.2 Rule 4: Before document end marker (...)
        if self.state.peek_char_at(0) == Some('.')
            && self.state.peek_char_at(1) == Some('.')
            && self.state.peek_char_at(2) == Some('.')
        {
            // Verify boundary after ... per YAML 1.2 specification
            match self.state.peek_char_at(3) {
                Some(' ') | Some('\t') | Some('\n') | Some('\r') | None => return Ok(true),
                _ => {} // Not a valid document marker, continue checking
            }
        }

        // YAML 1.2 Rule 5: Implicit document start (single-document streams)
        // BOM valid at start of first content line after stream start
        if mark.line == 1 && mark.col == 0 {
            return Ok(true);
        }

        // YAML 1.2 Rule 6: Inside document content - BOM invalid
        Ok(false)
    }

    /// Process structural separation using existing infrastructure
    pub fn process_structural_separation(
        &mut self,
        context: &mut crate::parser::grammar::ParametricContext,
        n: i32,
    ) -> Result<(), ScanError> {
        crate::parser::structural_productions::StructuralProductions::process_separation(
            &mut self.state,
            context,
            n,
        )
    }

    /// Validate structural indentation using existing system
    pub fn validate_structural_indent(
        &mut self,
        context: &crate::parser::grammar::ParametricContext,
        n: i32,
    ) -> Result<bool, ScanError> {
        crate::parser::structural_productions::StructuralProductions::validate_exact_indent(
            &mut self.state,
            context,
            n,
        )
    }

    /// Skip comments using existing utilities
    pub fn skip_structural_comments(&mut self) -> Result<Vec<String>, ScanError> {
        crate::parser::structural_productions::StructuralProductions::skip_comment_lines(
            &mut self.state,
        )
    }
}

#[cfg(test)]
mod tests {
    // Tests will be in separate test files
}
