//! Tag scanning with comprehensive URI handling and validation
//!
//! This module provides efficient scanning of YAML tags with proper handle resolution,
//! URI validation, and escape sequence processing according to YAML 1.2.

use crate::error::{Marker, ScanError};
use crate::scanner::ScannerConfig;
use crate::scanner::state::ScannerState;

/// Scan tag with handle and suffix resolution
#[inline]
pub fn scan_tag<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    _config: &ScannerConfig,
) -> Result<(String, String), ScanError> {
    let start_mark = state.mark();

    // Check for verbatim tag <...>
    if matches!(state.peek_char(), Ok('<')) {
        return scan_verbatim_tag(state);
    }

    // Scan tag handle
    let handle = scan_tag_handle(state)?;

    // Scan tag suffix
    let suffix = scan_tag_suffix(state)?;

    // Validate tag components
    validate_tag_handle(&handle, start_mark)?;
    validate_tag_suffix(&suffix, start_mark)?;

    Ok((handle, suffix))
}

/// Scan verbatim tag <uri>
#[inline]
fn scan_verbatim_tag<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<(String, String), ScanError> {
    let start_mark = state.mark();

    // Consume '<'
    state.consume_char()?;

    let mut uri = String::with_capacity(64);

    loop {
        match state.peek_char()? {
            '>' => {
                state.consume_char()?;
                break;
            }
            '%' => {
                // URI escape sequence
                uri.push_str(&decode_uri_escape(state)?);
            }
            ch if is_uri_char(ch) => {
                uri.push(state.consume_char()?);
            }
            ch => {
                return Err(ScanError::new(
                    state.mark(),
                    &format!("invalid character '{ch}' in verbatim tag"),
                ));
            }
        }

        // Prevent runaway tags
        if uri.len() > 4096 {
            return Err(ScanError::new(
                start_mark,
                "verbatim tag URI too long (max 4096 characters)",
            ));
        }
    }

    if uri.is_empty() {
        return Err(ScanError::new(start_mark, "empty verbatim tag"));
    }

    // Validate URI
    validate_tag_uri(&uri, start_mark)?;

    Ok(("!".to_string(), uri))
}

/// Scan tag handle (empty, !, !!, or !word!)
#[inline]
fn scan_tag_handle<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    let mut handle = String::with_capacity(16);

    // Check for secondary tag indicator
    if matches!(state.peek_char(), Ok('!')) {
        handle.push(state.consume_char()?);

        // Scan handle name
        while let Ok(ch) = state.peek_char() {
            if is_tag_handle_char(ch) {
                handle.push(state.consume_char()?);
            } else if ch == '!' {
                // Closing !
                handle.push(state.consume_char()?);
                break;
            } else {
                // End of handle
                break;
            }
        }
    }

    Ok(handle)
}

/// Scan tag suffix
#[inline]
fn scan_tag_suffix<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    let mut suffix = String::with_capacity(32);

    loop {
        match state.peek_char() {
            Ok('%') => {
                // URI escape sequence
                suffix.push_str(&decode_uri_escape(state)?);
            }
            Ok(ch) if is_tag_char(ch) => {
                suffix.push(state.consume_char()?);
            }
            _ => break,
        }

        // Prevent runaway suffixes
        if suffix.len() > 1024 {
            return Err(ScanError::new(
                state.mark(),
                "tag suffix too long (max 1024 characters)",
            ));
        }
    }

    Ok(suffix)
}

/// Decode URI escape sequence (%XX)
#[inline]
fn decode_uri_escape<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    // Consume '%'
    state.consume_char()?;

    let mut bytes = Vec::new();

    // Read sequence of %XX encodings
    loop {
        // Read two hex digits
        let hex_str = format!("{}{}", read_hex_digit(state)?, read_hex_digit(state)?);

        let byte = u8::from_str_radix(&hex_str, 16)
            .map_err(|_| ScanError::new(state.mark(), "invalid URI escape sequence"))?;

        bytes.push(byte);

        // Check if next is another escape
        if !matches!(state.peek_char(), Ok('%')) {
            break;
        }
        state.consume_char()?; // consume next '%'
    }

    // Convert bytes to UTF-8 string
    String::from_utf8(bytes)
        .map_err(|_| ScanError::new(state.mark(), "invalid UTF-8 in URI escape sequence"))
}

/// Read a single hex digit
#[inline]
fn read_hex_digit<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<char, ScanError> {
    match state.consume_char()? {
        ch @ ('0'..='9' | 'a'..='f' | 'A'..='F') => Ok(ch),
        ch => Err(ScanError::new(
            state.mark(),
            &format!("invalid hex digit '{ch}' in URI escape"),
        )),
    }
}

/// Check if character is valid in tag handles
#[inline]
const fn is_tag_handle_char(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-')
}

/// Check if character is valid in tag suffixes
#[inline]
const fn is_tag_char(ch: char) -> bool {
    // YAML 1.2 tag characters
    match ch {
        // URI characters
        'a'..='z' | 'A'..='Z' | '0'..='9' => true,
        '-' | '_' | '.' | '~' => true,
        ':' | '/' | '?' | '#' | '[' | ']' | '@' => true,
        '!' | '$' | '&' | '\'' | '(' | ')' => true,
        '*' | '+' | ',' | ';' | '=' => true,
        // Additional allowed characters
        '%' => true, // For escape sequences
        _ => false,
    }
}

/// Check if character is valid in URIs
#[inline]
const fn is_uri_char(ch: char) -> bool {
    // RFC 3986 URI characters
    match ch {
        // Unreserved characters
        'a'..='z' | 'A'..='Z' | '0'..='9' => true,
        '-' | '_' | '.' | '~' => true,
        // Reserved characters (when not percent-encoded)
        ':' | '/' | '?' | '#' | '[' | ']' | '@' => true,
        '!' | '$' | '&' | '\'' | '(' | ')' => true,
        '*' | '+' | ',' | ';' | '=' => true,
        // Percent encoding
        '%' => true,
        _ => false,
    }
}

/// Validate tag handle
#[inline]
fn validate_tag_handle(handle: &str, position: Marker) -> Result<(), ScanError> {
    if handle.is_empty() {
        return Ok(()); // Empty handle is valid (primary tag handle)
    }

    if handle == "!" {
        return Ok(()); // Secondary tag handle
    }

    if handle == "!!" {
        return Ok(()); // Global tag handle
    }

    // Named tag handle: !word!
    if !handle.starts_with('!') || !handle.ends_with('!') {
        return Err(ScanError::new(
            position,
            "invalid tag handle format (must be !, !!, or !word!)",
        ));
    }

    if handle.len() < 3 {
        return Err(ScanError::new(
            position,
            "named tag handle must have at least one character between !",
        ));
    }

    // Validate characters in handle name
    let handle_name = &handle[1..handle.len() - 1];
    for ch in handle_name.chars() {
        if !is_tag_handle_char(ch) {
            return Err(ScanError::new(
                position,
                &format!("invalid character '{ch}' in tag handle"),
            ));
        }
    }

    if handle.len() > 64 {
        return Err(ScanError::new(
            position,
            "tag handle too long (max 64 characters)",
        ));
    }

    Ok(())
}

/// Validate tag suffix
#[inline]
fn validate_tag_suffix(suffix: &str, position: Marker) -> Result<(), ScanError> {
    if suffix.is_empty() {
        return Err(ScanError::new(position, "empty tag suffix"));
    }

    // Check for invalid characters
    for ch in suffix.chars() {
        if !is_tag_char(ch) && ch != '%' {
            return Err(ScanError::new(
                position,
                &format!("invalid character '{ch}' in tag suffix"),
            ));
        }
    }

    // Validate percent encodings
    let mut chars = suffix.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Must be followed by two hex digits
            let hex1 = chars.next().ok_or_else(|| {
                ScanError::new(position, "incomplete percent encoding in tag suffix")
            })?;
            let hex2 = chars.next().ok_or_else(|| {
                ScanError::new(position, "incomplete percent encoding in tag suffix")
            })?;

            if !hex1.is_ascii_hexdigit() || !hex2.is_ascii_hexdigit() {
                return Err(ScanError::new(
                    position,
                    "invalid percent encoding in tag suffix",
                ));
            }
        }
    }

    Ok(())
}

/// Validate complete tag URI
#[inline]
fn validate_tag_uri(uri: &str, position: Marker) -> Result<(), ScanError> {
    if uri.is_empty() {
        return Err(ScanError::new(position, "empty tag URI"));
    }

    // Basic URI validation
    if uri.len() > 4096 {
        return Err(ScanError::new(
            position,
            "tag URI too long (max 4096 characters)",
        ));
    }

    // Check for valid URI characters
    for ch in uri.chars() {
        if !is_uri_char(ch) {
            return Err(ScanError::new(
                position,
                &format!("invalid character '{ch}' in tag URI"),
            ));
        }
    }

    Ok(())
}

/// Resolve tag handle to full URI
pub fn resolve_tag_handle(handle: &str, suffix: &str) -> Result<String, String> {
    let prefix = match handle {
        "" | "!" => "!",              // Local tag
        "!!" => "tag:yaml.org,2002:", // Global tag
        _ => {
            // Named handle - would need tag directive mapping
            return Err(format!("unresolved tag handle '{handle}'"));
        }
    };

    if prefix == "!" {
        // Local tag - just the suffix
        Ok(suffix.to_string())
    } else {
        // Global or named tag - concatenate
        Ok(format!("{prefix}{suffix}"))
    }
}

/// Check if tag is a standard YAML 1.2 tag
#[must_use] 
pub fn is_standard_tag(uri: &str) -> bool {
    uri.starts_with("tag:yaml.org,2002:")
}

/// Get YAML 1.2 standard tag name from URI
#[must_use] 
pub fn get_standard_tag_name(uri: &str) -> Option<&str> {
    uri.strip_prefix("tag:yaml.org,2002:")
}

/// Create local tag URI
#[must_use] 
pub fn create_local_tag(suffix: &str) -> String {
    format!("!{suffix}")
}

/// Create global tag URI
#[must_use] 
pub fn create_global_tag(type_name: &str) -> String {
    format!("tag:yaml.org,2002:{type_name}")
}
