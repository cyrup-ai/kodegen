//! Configuration constants and types for subprocess transport

/// Default maximum buffer size for JSON messages (1MB)
pub const DEFAULT_MAX_BUFFER_SIZE: usize = 1024 * 1024;

/// Dangerous environment variables that should not be passed to subprocess
///
/// These variables can affect how the subprocess loads and executes code,
/// potentially creating security vulnerabilities.
pub const DANGEROUS_ENV_VARS: &[&str] = &[
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "PATH",
    "NODE_OPTIONS",
    "PYTHONPATH",
    "PERL5LIB",
    "RUBYLIB",
];

/// Allowed extra CLI flags (allowlist approach)
///
/// Only these flags can be passed through the `extra_args` option.
pub const ALLOWED_EXTRA_FLAGS: &[&str] = &["timeout", "retries", "log-level", "cache-dir"];

/// Prompt input type
#[derive(Debug)]
pub enum PromptInput {
    /// Single string prompt
    String(String),
    /// Stream of JSON messages
    Stream,
}

impl From<String> for PromptInput {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for PromptInput {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}
