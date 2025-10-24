//! Security validation patterns and utilities
//!
//! Provides pre-compiled regex patterns for detecting security threats
//! including command injection, path traversal, and script injection.

use regex::Regex;
use std::sync::LazyLock;

/// Regex for detecting command injection patterns: `[;&|$()]` including backtick
pub static COMMAND_INJECTION_REGEX: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"[;&|`$()]").ok());

/// Regex for detecting path traversal patterns: ../
pub static PATH_TRAVERSAL_REGEX: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\.\.[\\/]").ok());

/// Regex for detecting script injection patterns: <script>
pub static SCRIPT_INJECTION_REGEX: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"<script[^>]*>").ok());

/// Get all security regex patterns as a vector
#[must_use]
pub fn get_security_patterns() -> Vec<Regex> {
    [
        &*COMMAND_INJECTION_REGEX,
        &*PATH_TRAVERSAL_REGEX,
        &*SCRIPT_INJECTION_REGEX,
    ]
    .iter()
    .filter_map(|opt| opt.as_ref().map(|r| (*r).clone()))
    .collect()
}
