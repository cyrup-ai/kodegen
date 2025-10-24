//! Anchor and alias scanning with efficient validation
//!
//! This module provides scanning of YAML anchors (&) and aliases (*) with proper
//! name validation and length checking according to YAML 1.2 specification.

use crate::error::ScanError;
use crate::scanner::state::ScannerState;

/// Scan anchor name (&anchor)
#[inline]
pub fn scan_anchor_name<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    scan_name(state, "anchor")
}

/// Scan alias name (*alias)
#[inline]
pub fn scan_alias_name<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    scan_name(state, "alias")
}

/// Scan anchor or alias name with validation
#[inline]
fn scan_name<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    name_type: &str,
) -> Result<String, ScanError> {
    let mut result = String::with_capacity(32);
    let start_mark = state.mark();

    // Read name characters
    loop {
        match state.peek_char() {
            Ok(ch) if is_anchor_char(ch) => {
                result.push(state.consume_char()?);

                // Check length limit
                if result.len() > 1024 {
                    return Err(ScanError::new(
                        state.mark(),
                        &format!("{name_type} name too long (max 1024 characters)"),
                    ));
                }
            }
            _ => break,
        }
    }

    // Validate name
    if result.is_empty() {
        return Err(ScanError::new(
            start_mark,
            &format!("empty {name_type} name"),
        ));
    }

    // Check for invalid first character
    if let Some(first_char) = result.chars().next()
        && !is_valid_first_anchor_char(first_char)
    {
        return Err(ScanError::new(
            start_mark,
            &format!("invalid first character '{first_char}' in {name_type} name"),
        ));
    }

    // Validate all characters are proper anchor chars
    for (i, ch) in result.chars().enumerate() {
        if !is_anchor_char(ch) {
            return Err(ScanError::new(
                start_mark,
                &format!("invalid character '{ch}' at position {i} in {name_type} name"),
            ));
        }
    }

    Ok(result)
}

/// Check if character can be used in anchor/alias names
#[inline]
fn is_anchor_char(ch: char) -> bool {
    // YAML 1.2 allows most printable characters except flow indicators
    // and some special characters
    match ch {
        // Flow indicators
        ',' | '[' | ']' | '{' | '}' => false,
        // Whitespace
        ' ' | '\t' | '\n' | '\r' => false,
        // Comment indicator
        '#' => false,
        // Tag indicators
        '!' => false,
        // Anchor/alias indicators (only valid as prefix)
        '&' | '*' => false,
        // Key/value indicators
        '?' | ':' => false,
        // Document markers
        '-' if is_document_marker_context(ch) => false,
        '.' if is_document_marker_context(ch) => false,
        // Quoted string indicators
        '\'' | '"' => false,
        // Block scalar indicators
        '|' | '>' => false,
        // Directive indicator
        '%' => false,
        // Reserved characters
        '@' | '`' => false,
        // Control characters
        ch if ch.is_control() && !matches!(ch, '\t' | '\n' | '\r') => false,
        // Unicode categories that should be excluded
        ch if is_unicode_format_char(ch) => false,
        // Everything else is allowed
        _ => true,
    }
}

/// Check if character is valid as first character of anchor/alias name
#[inline]
fn is_valid_first_anchor_char(ch: char) -> bool {
    // First character has additional restrictions
    match ch {
        // Numbers are allowed but might cause confusion
        '0'..='9' => true,
        // Letters are always good
        'a'..='z' | 'A'..='Z' => true,
        // Underscore is commonly used
        '_' => true,
        // Hyphen could be confused with document markers
        '-' => true,
        // Other characters that are anchor chars
        ch if is_anchor_char(ch) => {
            // Additional check for confusing first characters
            !matches!(ch, '.' | '+' | '~')
        }
        _ => false,
    }
}

/// Check if character is in document marker context (---, ...)
#[inline]
const fn is_document_marker_context(_ch: char) -> bool {
    // This is a simplified check - in real implementation,
    // we'd need to check if we're at start of line with
    // proper sequence
    false
}

/// Check if character is a Unicode format character that should be excluded
#[inline]
const fn is_unicode_format_char(ch: char) -> bool {
    // Unicode format characters that might cause issues
    matches!(
        ch,
        '\u{200B}' |  // Zero Width Space
        '\u{200C}' |  // Zero Width Non-Joiner
        '\u{200D}' |  // Zero Width Joiner
        '\u{2060}' |  // Word Joiner
        '\u{FEFF}' // Zero Width No-Break Space (BOM)
    )
}

/// Validate anchor name for YAML 1.2 compliance
pub fn validate_anchor_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("anchor name cannot be empty".to_string());
    }

    if name.len() > 1024 {
        return Err("anchor name too long (max 1024 characters)".to_string());
    }

    // Check first character
    if let Some(first) = name.chars().next()
        && !is_valid_first_anchor_char(first)
    {
        return Err(format!("invalid first character '{first}' in anchor name"));
    }

    // Check all characters
    for (i, ch) in name.chars().enumerate() {
        if !is_anchor_char(ch) {
            return Err(format!(
                "invalid character '{ch}' at position {i} in anchor name"
            ));
        }
    }

    // Check for reserved names that might cause confusion
    if is_reserved_anchor_name(name) {
        return Err(format!("anchor name '{name}' is reserved"));
    }

    Ok(())
}

/// Check if anchor name is reserved and might cause confusion
#[inline]
fn is_reserved_anchor_name(name: &str) -> bool {
    // Reserved names that might cause confusion with YAML constructs
    matches!(
        name,
        // Boolean values
        "true" | "false" | "True" | "False" | "TRUE" | "FALSE" |
        // Null values
        "null" | "Null" | "NULL" | "~" |
        // Numbers that might be confusing
        "inf" | "Inf" | "INF" | "+inf" | "+Inf" | "+INF" |
        "-inf" | "-Inf" | "-INF" |
        "nan" | "NaN" | "NAN" |
        // Document markers
        "---" | "..." |
        // Empty string representations
        "" | " " | "\t"
    )
}

/// Generate unique anchor name with prefix
#[must_use] 
pub fn generate_unique_anchor_name(prefix: &str, counter: usize) -> String {
    if prefix.is_empty() {
        format!("anchor_{counter}")
    } else {
        format!("{prefix}_{counter}")
    }
}

/// Check if two anchor names are equivalent (case-sensitive comparison)
#[inline]
#[must_use] 
pub fn anchor_names_equal(name1: &str, name2: &str) -> bool {
    // YAML anchor names are case-sensitive
    name1 == name2
}

/// Normalize anchor name for consistent storage
#[must_use] 
pub fn normalize_anchor_name(name: &str) -> String {
    // YAML anchor names should not be normalized - they are case-sensitive
    // and should be stored exactly as written
    name.to_string()
}
