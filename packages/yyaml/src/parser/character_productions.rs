//! YAML 1.2 Character Productions [1-62] - Consolidated Implementation
//!
//! This module consolidates all character handling functionality from across
//! the codebase into a single, spec-compliant interface that eliminates code
//! duplication and provides clear traceability to YAML 1.2 specification.
//!
//! All functionality delegates to the primary implementation in `lexer/unicode.rs`
//! to ensure a single source of truth for character handling operations.

use std::borrow::Cow;

// Re-export existing implementations for direct access
pub use crate::lexer::unicode::chars::*;
pub use crate::lexer::unicode::{EscapeError, EscapeStyle, UnicodeProcessor};

/// YAML 1.2 Character Productions [1-62] - Consolidated Interface
///
/// This struct provides a unified API for all YAML character productions,
/// delegating to the primary implementation in `lexer/unicode.rs` to eliminate
/// code duplication across the codebase.
pub struct CharacterProductions;

impl CharacterProductions {
    /// [1] c-printable - Check if character is printable for YAML
    ///
    /// Delegates to: `lexer/unicode.rs::chars::is_printable()`
    #[inline]
    #[must_use] 
    pub fn is_printable(ch: char) -> bool {
        crate::lexer::unicode::chars::is_printable(ch)
    }

    /// [2] nb-json - Check if character is JSON-compatible (printable except C0, C1, surrogates)
    ///
    /// JSON compatible characters are tab plus the printable subset of Unicode.
    #[inline]
    #[must_use] 
    pub fn is_nb_json(ch: char) -> bool {
        ch == '\t' || (ch as u32 >= 0x20 && ch as u32 <= 0x10FFFF && !Self::is_surrogate(ch))
    }

    /// [3] c-byte-order-mark - Unicode BOM detection and removal
    ///
    /// Delegates to: `lexer/unicode.rs::normalization::remove_bom()`
    #[inline]
    #[must_use] 
    pub fn remove_bom(input: &str) -> &str {
        crate::lexer::unicode::normalization::remove_bom(input)
    }

    /// [24-26] Line break characters
    ///
    /// Delegates to: `lexer/unicode.rs::chars::is_break()`
    #[inline]
    #[must_use] 
    pub const fn is_break(ch: char) -> bool {
        crate::lexer::unicode::chars::is_break(ch)
    }

    /// [31-33] White space characters (space and tab only in YAML)
    ///
    /// Delegates to: `lexer/unicode.rs::chars::is_white()`
    #[inline]
    #[must_use] 
    pub const fn is_white(ch: char) -> bool {
        crate::lexer::unicode::chars::is_white(ch)
    }

    /// [34] ns-char - Non-space characters
    ///
    /// Characters that are neither line breaks nor white space.
    #[inline]
    #[must_use] 
    pub fn is_ns_char(ch: char) -> bool {
        Self::is_printable(ch) && !Self::is_white(ch) && !Self::is_break(ch)
    }

    /// [42-61] Escape sequence processing - UNIFIED IMPLEMENTATION
    ///
    /// Delegates to: `lexer/unicode.rs::UnicodeProcessor::process_escapes()`
    ///
    /// This eliminates the duplicate implementation in `scanner/scalars.rs`
    /// and provides a single source of truth for all escape processing.
    #[inline]
    pub fn process_escape_sequences(input: &str) -> Result<Cow<'_, str>, EscapeError> {
        crate::lexer::unicode::UnicodeProcessor::process_escapes(input)
    }

    /// [62] Escaped line break handling
    ///
    /// Line breaks in escape sequences should be folded to single space.
    /// This is handled within the main escape processing.
    #[inline]
    #[must_use] 
    pub const fn fold_escaped_line_break() -> char {
        ' '
    }

    // Helper functions for character classification

    /// Check if character is a Unicode surrogate
    #[inline]
    fn is_surrogate(ch: char) -> bool {
        let code = ch as u32;
        (0xD800..=0xDFFF).contains(&code)
    }

    /// Check if character is blank (white space or line break)
    ///
    /// Delegates to: `lexer/unicode.rs::chars::is_blank()`
    #[inline]
    #[must_use] 
    pub const fn is_blank(ch: char) -> bool {
        crate::lexer::unicode::chars::is_blank(ch)
    }

    /// Check if character can start a plain scalar
    ///
    /// Delegates to: `lexer/unicode.rs::chars::can_start_plain_scalar()`
    #[inline]
    #[must_use] 
    pub const fn can_start_plain_scalar(ch: char) -> bool {
        crate::lexer::unicode::chars::can_start_plain_scalar(ch)
    }

    /// Check if character can continue a plain scalar
    ///
    /// Delegates to: `lexer/unicode.rs::chars::can_continue_plain_scalar()`
    #[inline]
    #[must_use] 
    pub const fn can_continue_plain_scalar(ch: char) -> bool {
        crate::lexer::unicode::chars::can_continue_plain_scalar(ch)
    }

    /// Escape string for YAML output
    ///
    /// Delegates to: `lexer/unicode.rs::UnicodeProcessor::escape_string()`
    #[inline]
    #[must_use] 
    pub fn escape_string(input: &str, style: EscapeStyle) -> String {
        crate::lexer::unicode::UnicodeProcessor::escape_string(input, style)
    }

    /// Normalize line endings to LF
    ///
    /// Delegates to: `lexer/unicode.rs::normalization::normalize_line_endings()`
    #[inline]
    #[must_use] 
    pub fn normalize_line_endings(input: &str) -> Cow<'_, str> {
        crate::lexer::unicode::normalization::normalize_line_endings(input)
    }

    /// Check if text contains only valid YAML characters
    ///
    /// Delegates to: `lexer/unicode.rs::normalization::is_valid_yaml_text()`
    #[inline]
    #[must_use] 
    pub fn is_valid_yaml_text(text: &str) -> bool {
        crate::lexer::unicode::normalization::is_valid_yaml_text(text)
    }

    /// Read exactly 2 hex digits for \xXX escapes
    #[inline]
    pub fn read_2_hex_digits<I: Iterator<Item = char>>(chars: &mut I) -> Result<u32, EscapeError> {
        crate::lexer::unicode::UnicodeProcessor::read_hex_digits(chars, 2)
    }

    /// Read exactly 4 hex digits for \uXXXX escapes  
    #[inline]
    pub fn read_4_hex_digits<I: Iterator<Item = char>>(chars: &mut I) -> Result<u32, EscapeError> {
        crate::lexer::unicode::UnicodeProcessor::read_hex_digits(chars, 4)
    }

    /// Read exactly 8 hex digits for \UXXXXXXXX escapes
    #[inline]
    pub fn read_8_hex_digits<I: Iterator<Item = char>>(chars: &mut I) -> Result<u32, EscapeError> {
        crate::lexer::unicode::UnicodeProcessor::read_hex_digits(chars, 8)
    }

    /// Parse hex characters from array - direct implementation for error handling
    #[inline]
    pub fn parse_hex_chars(hex_chars: &[char]) -> Result<u32, EscapeError> {
        let mut result = 0u32;
        for &ch in hex_chars {
            if ch.is_ascii_hexdigit() {
                if let Some(digit) = ch.to_digit(16) {
                    result = result * 16 + digit;
                } else {
                    return Err(EscapeError::InvalidHexDigit(ch));
                }
            } else {
                return Err(EscapeError::InvalidHexDigit(ch));
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_productions_delegation() {
        // Test that our delegated functions work correctly
        assert!(CharacterProductions::is_printable('A'));
        assert!(CharacterProductions::is_white(' '));
        assert!(CharacterProductions::is_break('\n'));
        assert!(!CharacterProductions::is_ns_char(' '));
        assert!(CharacterProductions::is_ns_char('A'));
    }

    #[test]
    fn test_escape_sequence_consolidation() {
        // Test that escape processing works through our unified interface
        let result = CharacterProductions::process_escape_sequences("hello\\nworld");
        match result {
            Ok(processed) => assert_eq!(processed, "hello\nworld"),
            Err(_) => panic!("Expected successful escape processing"),
        }
    }

    #[test]
    fn test_nb_json_compatibility() {
        assert!(CharacterProductions::is_nb_json('\t'));
        assert!(CharacterProductions::is_nb_json('A'));
        assert!(CharacterProductions::is_nb_json('€'));
        assert!(!CharacterProductions::is_nb_json('\x00'));
        assert!(!CharacterProductions::is_nb_json('\x1F'));
    }
}
