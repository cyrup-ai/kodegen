//! Document marker scanning and validation
//!
//! This module handles YAML document start (---) and end (...) markers
//! with proper boundary checking and context validation.

use crate::error::ScanError;
use crate::scanner::state::ScannerState;

/// Document marker types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentMarker {
    /// Document start marker (---)
    Start,
    /// Document end marker (...)
    End,
}

/// Scan document start marker (---)
#[inline]
pub fn scan_document_start<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<(), ScanError> {
    let start_mark = state.mark();

    // Verify we're at start of line or after whitespace
    if !is_valid_marker_context(state)? {
        return Err(ScanError::new(
            start_mark,
            "document start marker must be at start of line or after whitespace",
        ));
    }

    // Consume the three dashes
    for i in 0..3 {
        match state.consume_char()? {
            '-' => continue,
            ch => {
                return Err(ScanError::new(
                    state.mark(),
                    &format!("expected '-' at position {i} in document start marker, found '{ch}'"),
                ));
            }
        }
    }

    // Verify marker is followed by proper boundary
    validate_marker_boundary(state, DocumentMarker::Start)?;

    Ok(())
}

/// Scan document end marker (...)
#[inline]
pub fn scan_document_end<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<(), ScanError> {
    let start_mark = state.mark();

    // Verify we're at start of line or after whitespace
    if !is_valid_marker_context(state)? {
        return Err(ScanError::new(
            start_mark,
            "document end marker must be at start of line or after whitespace",
        ));
    }

    // Consume the three dots
    for i in 0..3 {
        match state.consume_char()? {
            '.' => continue,
            ch => {
                return Err(ScanError::new(
                    state.mark(),
                    &format!("expected '.' at position {i} in document end marker, found '{ch}'"),
                ));
            }
        }
    }

    // Verify marker is followed by proper boundary
    validate_marker_boundary(state, DocumentMarker::End)?;

    Ok(())
}

/// Check if current context is valid for document markers
#[inline]
const fn is_valid_marker_context<T: Iterator<Item = char>>(
    state: &ScannerState<T>,
) -> Result<bool, ScanError> {
    // Document markers are valid:
    // 1. At start of line (column 0)
    // 2. After whitespace-only content on the line
    // 3. At start of stream
    Ok(state.at_line_start() || state.mark().index == 0)
}

/// Validate that document marker is followed by proper boundary
#[inline]
fn validate_marker_boundary<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    marker_type: DocumentMarker,
) -> Result<(), ScanError> {
    let marker_name = match marker_type {
        DocumentMarker::Start => "document start",
        DocumentMarker::End => "document end",
    };

    match state.peek_char() {
        // EOF is always valid boundary
        Err(_) => Ok(()),

        // Whitespace is valid boundary
        Ok(' ') | Ok('\t') | Ok('\n') | Ok('\r') => Ok(()),

        // Comment is valid boundary
        Ok('#') => Ok(()),

        // Flow indicators are valid in flow context
        Ok(',') | Ok('[') | Ok(']') | Ok('{') | Ok('}') if state.in_flow_context() => Ok(()),

        // Invalid boundary
        Ok(ch) => Err(ScanError::new(
            state.mark(),
            &format!(
                "{marker_name} marker must be followed by whitespace, comment, or end of input, found '{ch}'"
            ),
        )),
    }
}

/// Check if sequence looks like document start
#[inline]
pub fn is_document_start_sequence<T: Iterator<Item = char>>(state: &mut ScannerState<T>) -> bool {
    state.check_chars(&['-', '-', '-']) && check_document_boundary(state, 3)
}

/// Check if sequence looks like document end
#[inline]
pub fn is_document_end_sequence<T: Iterator<Item = char>>(state: &mut ScannerState<T>) -> bool {
    state.check_chars(&['.', '.', '.']) && check_document_boundary(state, 3)
}

/// Check if characters after position form valid document marker boundary
#[inline]
fn check_document_boundary<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    offset: usize,
) -> bool {
    match state.peek_char_at(offset) {
        // EOF is valid boundary
        None => true,

        // Whitespace and comment are valid boundaries
        Some(' ') | Some('\t') | Some('\n') | Some('\r') | Some('#') => true,

        // Flow indicators in flow context
        Some(',') | Some('[') | Some(']') | Some('{') | Some('}') if state.in_flow_context() => {
            true
        }

        // Everything else is invalid
        _ => false,
    }
}

/// Peek ahead to check for document markers without consuming
#[inline]
pub fn peek_document_marker<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Option<DocumentMarker> {
    if is_document_start_sequence(state) {
        Some(DocumentMarker::Start)
    } else if is_document_end_sequence(state) {
        Some(DocumentMarker::End)
    } else {
        None
    }
}

/// Skip past document marker and any following whitespace
#[inline]
pub fn consume_document_marker<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    marker_type: DocumentMarker,
) -> Result<(), ScanError> {
    match marker_type {
        DocumentMarker::Start => scan_document_start(state),
        DocumentMarker::End => scan_document_end(state),
    }
}

/// Check if we're at a potential document boundary
#[inline]
pub fn at_document_boundary<T: Iterator<Item = char>>(state: &mut ScannerState<T>) -> bool {
    peek_document_marker(state).is_some()
}

/// Validate document marker in current context
pub fn validate_document_marker_context<T: Iterator<Item = char>>(
    state: &ScannerState<T>,
    marker_type: DocumentMarker,
) -> Result<(), ScanError> {
    let _marker_name = match marker_type {
        DocumentMarker::Start => "document start",
        DocumentMarker::End => "document end",
    };

    // Document start is always valid
    if matches!(marker_type, DocumentMarker::Start) {
        return Ok(());
    }

    // Document end validation
    if matches!(marker_type, DocumentMarker::End) {
        // Document end in flow context might be problematic
        if state.in_flow_context() {
            return Err(ScanError::new(
                state.mark(),
                "document end marker not allowed inside flow collections",
            ));
        }

        // Document end should not be nested
        if state.flow_level() > 0 {
            return Err(ScanError::new(
                state.mark(),
                "document end marker not allowed at this nesting level",
            ));
        }
    }

    Ok(())
}

/// Get marker character for document marker type
#[inline]
#[must_use] 
pub const fn marker_character(marker_type: DocumentMarker) -> char {
    match marker_type {
        DocumentMarker::Start => '-',
        DocumentMarker::End => '.',
    }
}

/// Get marker string for document marker type
#[inline]
#[must_use] 
pub const fn marker_string(marker_type: DocumentMarker) -> &'static str {
    match marker_type {
        DocumentMarker::Start => "---",
        DocumentMarker::End => "...",
    }
}

/// Create formatted error message for document marker
#[must_use] 
pub fn format_marker_error(marker_type: DocumentMarker, context: &str) -> String {
    let marker_str = marker_string(marker_type);
    let marker_name = match marker_type {
        DocumentMarker::Start => "document start",
        DocumentMarker::End => "document end",
    };

    format!("{marker_name} marker '{marker_str}' {context}")
}
