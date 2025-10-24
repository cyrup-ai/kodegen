//! Directive scanning with comprehensive validation
//!
//! This module handles YAML directives including %YAML version and %TAG prefix
//! declarations with proper syntax validation and semantic checking.

use crate::error::{Marker, ScanError};
use crate::scanner::state::ScannerState;

/// Directive types for type-safe handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    /// YAML version directive: %YAML major.minor
    Version { major: u32, minor: u32 },
    /// TAG directive: %TAG handle prefix
    Tag { handle: String, prefix: String },
    /// Reserved directive: %NAME args...
    Reserved { name: String, args: Vec<String> },
}

/// Scan directive starting after '%' character
#[inline]
pub fn scan_directive<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<Directive, ScanError> {
    let start_mark = state.mark();

    // Skip whitespace after %
    skip_whitespace(state);

    // Read directive name
    let name = scan_directive_name(state)?;

    match name.as_str() {
        "YAML" => scan_yaml_directive(state, start_mark),
        "TAG" => scan_tag_directive(state, start_mark),
        _ => scan_reserved_directive(state, name, start_mark),
    }
}

/// Scan directive name
#[inline]
fn scan_directive_name<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    let mut name = String::with_capacity(16);
    let start_mark = state.mark();

    // First character must be letter
    match state.peek_char()? {
        ch if ch.is_ascii_alphabetic() => {
            name.push(state.consume_char()?);
        }
        ch => {
            return Err(ScanError::new(
                start_mark,
                &format!("invalid directive name start character '{ch}'"),
            ));
        }
    }

    // Subsequent characters can be letters, digits, hyphen, underscore
    while let Ok(ch) = state.peek_char() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            name.push(state.consume_char()?);

            if name.len() > 32 {
                return Err(ScanError::new(
                    start_mark,
                    "directive name too long (max 32 characters)",
                ));
            }
        } else {
            break;
        }
    }

    if name.is_empty() {
        return Err(ScanError::new(start_mark, "empty directive name"));
    }

    Ok(name)
}

/// Scan YAML version directive: %YAML major.minor
#[inline]
fn scan_yaml_directive<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    start_mark: Marker,
) -> Result<Directive, ScanError> {
    // Require at least one space
    require_whitespace(state, "YAML directive")?;
    skip_whitespace(state);

    // Scan major version number
    let major = scan_version_number(state, "major version")?;

    // Require dot separator
    if state.consume_char()? != '.' {
        return Err(ScanError::new(
            state.mark(),
            "expected '.' in YAML version directive",
        ));
    }

    // Scan minor version number
    let minor = scan_version_number(state, "minor version")?;

    // Validate version
    validate_yaml_version(major, minor, start_mark)?;

    // Skip trailing whitespace
    skip_whitespace(state);

    // Ensure line ends properly
    ensure_line_end(state, "YAML directive")?;

    Ok(Directive::Version { major, minor })
}

/// Scan TAG directive: %TAG handle prefix
#[inline]
fn scan_tag_directive<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    start_mark: Marker,
) -> Result<Directive, ScanError> {
    // Require at least one space
    require_whitespace(state, "TAG directive")?;
    skip_whitespace(state);

    // Scan tag handle
    let handle = scan_tag_handle_directive(state)?;

    // Require whitespace between handle and prefix
    require_whitespace(state, "TAG directive handle")?;
    skip_whitespace(state);

    // Scan tag prefix
    let prefix = scan_tag_prefix_directive(state)?;

    // Validate handle and prefix
    validate_tag_directive(&handle, &prefix, start_mark)?;

    // Skip trailing whitespace
    skip_whitespace(state);

    // Ensure line ends properly
    ensure_line_end(state, "TAG directive")?;

    Ok(Directive::Tag { handle, prefix })
}

/// Scan reserved directive: %NAME args...
#[inline]
fn scan_reserved_directive<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    name: String,
    _start_mark: Marker,
) -> Result<Directive, ScanError> {
    let mut args = Vec::new();

    // Skip initial whitespace
    skip_whitespace(state);

    // Scan arguments until end of line
    while !is_line_end(state)? {
        // Skip whitespace before argument
        skip_whitespace(state);

        if is_line_end(state)? {
            break;
        }

        // Scan one argument
        let arg = scan_directive_argument(state)?;
        args.push(arg);

        // Skip whitespace after argument
        skip_whitespace(state);
    }

    Ok(Directive::Reserved { name, args })
}

/// Scan version number (sequence of digits)
#[inline]
fn scan_version_number<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &str,
) -> Result<u32, ScanError> {
    let mut number_str = String::with_capacity(8);
    let start_mark = state.mark();

    // Read digits
    while let Ok(ch) = state.peek_char() {
        if ch.is_ascii_digit() {
            number_str.push(state.consume_char()?);

            if number_str.len() > 6 {
                return Err(ScanError::new(
                    start_mark,
                    &format!("{context} number too long"),
                ));
            }
        } else {
            break;
        }
    }

    if number_str.is_empty() {
        return Err(ScanError::new(
            start_mark,
            &format!("expected {context} number"),
        ));
    }

    number_str.parse::<u32>().map_err(|_| {
        ScanError::new(
            start_mark,
            &format!("invalid {context} number '{number_str}'"),
        )
    })
}

/// Scan tag handle in TAG directive
#[inline]
fn scan_tag_handle_directive<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    let mut handle = String::with_capacity(32);
    let start_mark = state.mark();

    // Must start with !
    if state.consume_char()? != '!' {
        return Err(ScanError::new(start_mark, "tag handle must start with '!'"));
    }
    handle.push('!');

    // Scan handle characters
    while let Ok(ch) = state.peek_char() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            handle.push(state.consume_char()?);
        } else if ch == '!' {
            handle.push(state.consume_char()?);
            break;
        } else if ch.is_whitespace() {
            break;
        } else {
            return Err(ScanError::new(
                state.mark(),
                &format!("invalid character '{ch}' in tag handle"),
            ));
        }

        if handle.len() > 64 {
            return Err(ScanError::new(
                start_mark,
                "tag handle too long (max 64 characters)",
            ));
        }
    }

    // Handle must end with ! (except for primary handle "!")
    if handle != "!" && !handle.ends_with('!') {
        return Err(ScanError::new(
            start_mark,
            "tag handle must end with '!' (except primary handle)",
        ));
    }

    Ok(handle)
}

/// Scan tag prefix in TAG directive
#[inline]
fn scan_tag_prefix_directive<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    let mut prefix = String::with_capacity(128);
    let start_mark = state.mark();

    // Scan prefix characters (URI characters plus percent encoding)
    while let Ok(ch) = state.peek_char() {
        if is_tag_prefix_char(ch) {
            prefix.push(state.consume_char()?);
        } else if ch == '%' {
            // Percent encoding
            prefix.push_str(&scan_percent_encoding(state)?);
        } else if ch.is_whitespace() || is_line_end_char(ch) {
            break;
        } else {
            return Err(ScanError::new(
                state.mark(),
                &format!("invalid character '{ch}' in tag prefix"),
            ));
        }

        if prefix.len() > 1024 {
            return Err(ScanError::new(
                start_mark,
                "tag prefix too long (max 1024 characters)",
            ));
        }
    }

    if prefix.is_empty() {
        return Err(ScanError::new(start_mark, "empty tag prefix"));
    }

    Ok(prefix)
}

/// Scan directive argument (non-whitespace sequence)
#[inline]
fn scan_directive_argument<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    let mut arg = String::with_capacity(64);

    while let Ok(ch) = state.peek_char() {
        if ch.is_whitespace() || is_line_end_char(ch) {
            break;
        }
        arg.push(state.consume_char()?);

        if arg.len() > 256 {
            return Err(ScanError::new(
                state.mark(),
                "directive argument too long (max 256 characters)",
            ));
        }
    }

    Ok(arg)
}

/// Scan percent encoding sequence (%XX)
#[inline]
fn scan_percent_encoding<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    // Consume '%'
    let percent = state.consume_char()?;

    // Read two hex digits
    let hex1 = match state.consume_char()? {
        ch if ch.is_ascii_hexdigit() => ch,
        ch => {
            return Err(ScanError::new(
                state.mark(),
                &format!("invalid hex digit '{ch}' in percent encoding"),
            ));
        }
    };

    let hex2 = match state.consume_char()? {
        ch if ch.is_ascii_hexdigit() => ch,
        ch => {
            return Err(ScanError::new(
                state.mark(),
                &format!("invalid hex digit '{ch}' in percent encoding"),
            ));
        }
    };

    Ok(format!("{percent}{hex1}{hex2}"))
}

// Utility functions

/// Skip whitespace characters (space and tab)
#[inline]
fn skip_whitespace<T: Iterator<Item = char>>(state: &mut ScannerState<T>) {
    while matches!(state.peek_char(), Ok(' ') | Ok('\t')) {
        let _ = state.consume_char();
    }
}

/// Require at least one whitespace character
#[inline]
fn require_whitespace<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &str,
) -> Result<(), ScanError> {
    if !matches!(state.peek_char(), Ok(' ') | Ok('\t')) {
        return Err(ScanError::new(
            state.mark(),
            &format!("expected whitespace in {context}"),
        ));
    }

    skip_whitespace(state);
    Ok(())
}

/// Check if at end of line
#[inline]
fn is_line_end<T: Iterator<Item = char>>(state: &mut ScannerState<T>) -> Result<bool, ScanError> {
    match state.peek_char() {
        Ok(ch) => Ok(is_line_end_char(ch)),
        Err(_) => Ok(true), // EOF is line end
    }
}

/// Check if character is line ending
#[inline]
const fn is_line_end_char(ch: char) -> bool {
    matches!(ch, '\n' | '\r')
}

/// Ensure line ends properly (newline, EOF, or comment)
#[inline]
fn ensure_line_end<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &str,
) -> Result<(), ScanError> {
    match state.peek_char() {
        Ok('#') => Ok(()),                        // Comment starts
        Ok(ch) if is_line_end_char(ch) => Ok(()), // Newline
        Err(_) => Ok(()),                         // EOF
        Ok(ch) => Err(ScanError::new(
            state.mark(),
            &format!("unexpected character '{ch}' after {context}"),
        )),
    }
}

/// Check if character is valid in tag prefixes
#[inline]
const fn is_tag_prefix_char(ch: char) -> bool {
    // URI characters excluding % (handled separately)
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '-' | '_'
                | '.'
                | '~'
                | ':'
                | '/'
                | '?'
                | '#'
                | '['
                | ']'
                | '@'
                | '!'
                | '$'
                | '&'
                | '\''
                | '('
                | ')'
                | '*'
                | '+'
                | ','
                | ';'
                | '='
        )
}

// Validation functions

/// Validate YAML version
#[inline]
fn validate_yaml_version(major: u32, minor: u32, position: Marker) -> Result<(), ScanError> {
    // YAML versions we support
    match (major, minor) {
        (1, 0) | (1, 1) | (1, 2) => Ok(()),
        (1, m) if m > 2 => {
            // Future minor versions might be compatible
            Ok(())
        }
        (m, _) if m > 1 => Err(ScanError::new(
            position,
            &format!("unsupported YAML version {major}.{minor}"),
        )),
        (0, _) => Err(ScanError::new(
            position,
            "invalid YAML version (major version cannot be 0)",
        )),
        _ => Err(ScanError::new(
            position,
            &format!("invalid YAML version {major}.{minor}"),
        )),
    }
}

/// Validate TAG directive components
#[inline]
fn validate_tag_directive(handle: &str, prefix: &str, position: Marker) -> Result<(), ScanError> {
    // Validate handle format
    if handle != "!" && handle != "!!" && (!handle.starts_with('!') || !handle.ends_with('!')) {
        return Err(ScanError::new(
            position,
            "invalid tag handle format (must be !, !!, or !word!)",
        ));
    }

    // Validate handle length
    if handle.len() > 64 {
        return Err(ScanError::new(
            position,
            "tag handle too long (max 64 characters)",
        ));
    }

    // Validate prefix
    if prefix.is_empty() {
        return Err(ScanError::new(position, "empty tag prefix"));
    }

    if prefix.len() > 1024 {
        return Err(ScanError::new(
            position,
            "tag prefix too long (max 1024 characters)",
        ));
    }

    Ok(())
}

/// Check if directive is valid YAML 1.2 directive
#[must_use] 
pub const fn is_standard_directive(directive: &Directive) -> bool {
    matches!(directive, Directive::Version { .. } | Directive::Tag { .. })
}

/// Get directive name as string
#[must_use] 
pub fn directive_name(directive: &Directive) -> &str {
    match directive {
        Directive::Version { .. } => "YAML",
        Directive::Tag { .. } => "TAG",
        Directive::Reserved { name, .. } => name,
    }
}
