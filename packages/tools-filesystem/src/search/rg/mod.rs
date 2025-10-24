// Ripgrep integration for MCP - configuration types and matcher building
// Exposes ripgrep's powerful regex/PCRE2 matching through MCP JSON interface
pub mod flags;
pub mod haystack;
pub(crate) mod json_output;
pub mod search;

use anyhow::Result;
use grep::regex::{RegexMatcher, RegexMatcherBuilder};
use grep_pcre2::RegexMatcherBuilder as PCRE2Builder;

// Re-export PatternMatcher for use in other modules
pub use search::PatternMatcher;

/// Build pattern matcher with ripgrep's exact configuration
/// Returns `PatternMatcher` enum supporting both Rust regex and PCRE2
pub fn build_pattern_matcher(
    pattern: &str,
    engine: super::types::Engine,
    case_mode: super::types::CaseMode,
    literal_search: bool,
    word_boundary: bool,
) -> Result<PatternMatcher> {
    use super::types::Engine;

    match engine {
        Engine::Rust => {
            let matcher = build_rust_matcher(pattern, case_mode, literal_search, word_boundary)?;
            Ok(PatternMatcher::RustRegex(matcher))
        }
        Engine::PCRE2 => {
            let matcher = build_pcre2_matcher(pattern, case_mode, literal_search, word_boundary)?;
            Ok(PatternMatcher::PCRE2(matcher))
        }
        Engine::Auto => {
            // Try Rust first, fall back to PCRE2 on error
            match build_rust_matcher(pattern, case_mode, literal_search, word_boundary) {
                Ok(m) => Ok(PatternMatcher::RustRegex(m)),
                Err(rust_err) => {
                    match build_pcre2_matcher(pattern, case_mode, literal_search, word_boundary) {
                        Ok(m) => Ok(PatternMatcher::PCRE2(m)),
                        Err(_pcre_err) => {
                            // Return original Rust error for clarity
                            Err(rust_err)
                        }
                    }
                }
            }
        }
    }
}

/// Build Rust regex matcher with ripgrep's exact configuration
pub(crate) fn build_rust_matcher(
    pattern: &str,
    case_mode: super::types::CaseMode,
    literal_search: bool,
    word_boundary: bool,
) -> Result<RegexMatcher> {
    use super::types::CaseMode;

    let mut builder = RegexMatcherBuilder::new();

    // Core configuration (from ripgrep)
    builder
        .multi_line(true)
        .unicode(true)
        .octal(false)
        .fixed_strings(literal_search);

    // Case sensitivity
    match case_mode {
        CaseMode::Sensitive => builder.case_insensitive(false),
        CaseMode::Insensitive => builder.case_insensitive(true),
        CaseMode::Smart => builder.case_smart(true),
    };

    // Word boundary
    if word_boundary {
        builder.word(true);
    }

    // Line terminator configuration
    builder
        .line_terminator(Some(b'\n'))
        .dot_matches_new_line(false);

    let matcher = builder.build(pattern)?;
    Ok(matcher)
}

/// Build PCRE2 matcher with ripgrep's exact configuration
pub(crate) fn build_pcre2_matcher(
    pattern: &str,
    case_mode: super::types::CaseMode,
    literal_search: bool,
    word_boundary: bool,
) -> Result<grep_pcre2::RegexMatcher> {
    use super::types::CaseMode;

    let mut builder = PCRE2Builder::new();

    // Core configuration
    builder.multi_line(true).fixed_strings(literal_search);

    // Case sensitivity
    match case_mode {
        CaseMode::Sensitive => builder.caseless(false),
        CaseMode::Insensitive => builder.caseless(true),
        CaseMode::Smart => builder.case_smart(true),
    };

    // Word boundary
    if word_boundary {
        builder.word(true);
    }

    // Unicode support
    builder.utf(true).ucp(true);

    // JIT compilation (64-bit only)
    if cfg!(target_pointer_width = "64") {
        builder
            .jit_if_available(true)
            .max_jit_stack_size(Some(10 * (1 << 20))); // 10MB
    }

    let matcher = builder.build(pattern)?;
    Ok(matcher)
}
