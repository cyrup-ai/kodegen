//! Comprehensive character-level analysis for diagnosing string differences
//!
//! This module provides detailed character code analysis to help AI agents
//! identify and fix invisible character differences (tabs, spaces, line endings,
//! zero-width Unicode, encoding issues, etc.)

use lru::LruCache;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use unicode_normalization::UnicodeNormalization;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Zero-width Unicode characters that are invisible but affect string matching
const ZERO_WIDTH_CHARS: &[u32] = &[
    0x200B, // Zero-width space
    0x200C, // Zero-width non-joiner
    0x200D, // Zero-width joiner
    0xFEFF, // Zero-width no-break space (BOM)
];

/// Global LRU cache for analysis results (100 most recent)
static ANALYSIS_CACHE: std::sync::LazyLock<Mutex<LruCache<String, CharCodeData>>> =
    std::sync::LazyLock::new(|| {
        // SAFETY: 100 is a non-zero compile-time constant
        Mutex::new(LruCache::new(unsafe { NonZeroUsize::new_unchecked(100) }))
    });

// ============================================================================
// CORE DATA STRUCTURES
// ============================================================================

/// Comprehensive character code analysis result
#[derive(Debug, Clone)]
pub struct CharCodeData {
    /// Basic report: "code:count[display],..." format
    pub report: String,

    /// Number of unique character codes
    pub unique_count: usize,

    /// Total length of differing portions
    pub diff_length: usize,

    /// Semantic classification of characters
    pub classification: CharCodeClassification,

    /// Detected whitespace issues
    pub whitespace_issues: Vec<WhitespaceIssue>,

    /// Detected encoding issues
    pub encoding_issues: Vec<EncodingIssue>,

    /// Character distribution comparison
    pub distribution: CharDistribution,

    /// Unicode normalization analysis
    pub unicode_analysis: UnicodeAnalysis,

    /// Zero-width character detection
    pub has_zero_width: bool,

    /// Smart fix suggestion
    pub suggestion: Option<String>,

    /// Visual diff with inline codes
    pub visual_diff_with_codes: String,
}

/// Semantic grouping of character types
#[derive(Debug, Clone, Default)]
pub struct CharCodeClassification {
    pub whitespace: Vec<(u32, usize)>,   // spaces, tabs, nbsp
    pub line_endings: Vec<(u32, usize)>, // CR, LF
    pub printable: Vec<(u32, usize)>,    // regular printable chars
    pub control: Vec<(u32, usize)>,      // control chars
    pub unicode: Vec<(u32, usize)>,      // non-ASCII
}

/// Common whitespace/formatting problems
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhitespaceIssue {
    TabsVsSpaces,       // Mixed tabs and spaces
    MixedLineEndings,   // Both CR and LF present
    ExtraSpaces,        // More than 3 consecutive spaces
    TrailingWhitespace, // Whitespace at end of lines
}

/// Encoding-related problems
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodingIssue {
    Utf16Surrogate,  // UTF-16 surrogate in UTF-8 context
    ReplacementChar, // U+FFFD � character
    ByteOrderMark,   // U+FEFF BOM character
}

/// Comparison of character distributions
#[derive(Debug, Clone, Default)]
pub struct CharDistribution {
    pub only_in_expected: Vec<(u32, usize)>, // Chars only in search string
    pub only_in_actual: Vec<(u32, usize)>,   // Chars only in found string
    pub in_both: Vec<(u32, usize, usize)>,   // (code, exp_count, act_count)
}

/// Unicode normalization status
#[derive(Debug, Clone)]
pub struct UnicodeAnalysis {
    pub has_composed: bool,           // Contains composed chars (é)
    pub has_decomposed: bool,         // Contains decomposed chars (e + ´)
    pub normalization_mismatch: bool, // NFC normalized would match
}

// ============================================================================
// MAIN ANALYSIS IMPLEMENTATION
// ============================================================================

impl CharCodeData {
    /// Perform comprehensive character code analysis with caching
    ///
    /// This is the main entry point for AI agents. It analyzes the differences
    /// between expected and actual strings, providing detailed diagnostic information.
    #[must_use]
    pub fn analyze(expected: &str, actual: &str) -> Self {
        // Try cache first
        let cache_key = format!("{expected}|{actual}");
        {
            let mut cache = ANALYSIS_CACHE.lock();
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }

        // Perform analysis
        let result = Self::analyze_uncached(expected, actual);

        // Cache result
        {
            let mut cache = ANALYSIS_CACHE.lock();
            cache.put(cache_key, result.clone());
        }

        result
    }

    fn analyze_uncached(expected: &str, actual: &str) -> Self {
        // Step 1: Find common boundaries
        let (prefix_len, suffix_len) = find_common_boundaries(expected, actual);

        // Step 2: Extract diffs using character indices
        let exp_chars: Vec<char> = expected.chars().collect();
        let act_chars: Vec<char> = actual.chars().collect();

        let exp_diff_chars = &exp_chars[prefix_len..exp_chars.len().saturating_sub(suffix_len)];
        let act_diff_chars = &act_chars[prefix_len..act_chars.len().saturating_sub(suffix_len)];

        let expected_diff: String = exp_diff_chars.iter().collect();
        let actual_diff: String = act_diff_chars.iter().collect();

        // Step 3: Count character codes in combined diff
        let mut full_diff = String::new();
        full_diff.push_str(&expected_diff);
        full_diff.push_str(&actual_diff);

        let mut codes: HashMap<u32, usize> = HashMap::new();
        for ch in full_diff.chars() {
            *codes.entry(ch as u32).or_insert(0) += 1;
        }

        // Step 4: Generate basic report
        let report = format_char_code_report(&codes);
        let unique_count = codes.len();
        let diff_length = full_diff.chars().count();

        // Step 5: Semantic classification
        let classification = classify_characters(&codes);

        // Step 6: Detect whitespace issues
        let whitespace_issues = detect_whitespace_issues(&codes, &expected_diff, &actual_diff);

        // Step 7: Detect encoding issues
        let encoding_issues = detect_encoding_issues(&codes);

        // Step 8: Character distribution comparison
        let distribution = compare_distribution(&expected_diff, &actual_diff);

        // Step 9: Unicode normalization analysis
        let unicode_analysis = analyze_unicode(expected, actual);

        // Step 10: Zero-width detection
        let has_zero_width = ZERO_WIDTH_CHARS
            .iter()
            .any(|&code| codes.contains_key(&code));

        // Step 11: Generate smart suggestion
        let suggestion = generate_suggestion(
            &whitespace_issues,
            &encoding_issues,
            has_zero_width,
            &unicode_analysis,
        );

        // Step 12: Visual diff with inline codes
        let visual_diff_with_codes = format_visual_diff_with_codes(
            expected,
            actual,
            &expected_diff,
            &actual_diff,
            prefix_len,
            suffix_len,
        );

        Self {
            report,
            unique_count,
            diff_length,
            classification,
            whitespace_issues,
            encoding_issues,
            distribution,
            unicode_analysis,
            has_zero_width,
            suggestion,
            visual_diff_with_codes,
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Find common prefix and suffix lengths (in character counts, not bytes)
fn find_common_boundaries(a: &str, b: &str) -> (usize, usize) {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    // Find prefix length
    let prefix_len = a_chars
        .iter()
        .zip(b_chars.iter())
        .take_while(|(ca, cb)| ca == cb)
        .count();

    // Find suffix length
    let max_suffix = a_chars.len().min(b_chars.len()).saturating_sub(prefix_len);
    let suffix_len = a_chars
        .iter()
        .rev()
        .zip(b_chars.iter().rev())
        .take(max_suffix)
        .take_while(|(ca, cb)| ca == cb)
        .count();

    (prefix_len, suffix_len)
}

/// Format character codes as "code:count[display],code:count[display],..."
/// Sorted by character code in ascending order
fn format_char_code_report(codes: &HashMap<u32, usize>) -> String {
    let mut entries: Vec<_> = codes.iter().collect();
    entries.sort_by_key(|(code, _)| *code); // Sort by code ascending

    entries
        .iter()
        .map(|(code, count)| {
            let display = format_char_display(**code);
            format!("{code}:{count}[{display}]")
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Format a single character code for display
/// - Printable ASCII (32-126): show the character itself
/// - Non-printable (<32, 127+): show as \xHH hex escape
fn format_char_display(code: u32) -> String {
    match code {
        32..=126 => {
            char::from_u32(code).map_or_else(|| format!("\\x{code:02x}"), |c| c.to_string())
        }
        _ => format!("\\x{code:02x}"),
    }
}

/// Get human-readable name for common character codes
fn format_char_name(code: u32) -> &'static str {
    match code {
        9 => "TAB",
        10 => "LF",
        13 => "CR",
        32 => "SPACE",
        160 => "NBSP",
        0x200B => "ZWSP",
        0x200C => "ZWNJ",
        0x200D => "ZWJ",
        0xFEFF => "BOM",
        _ => "?",
    }
}

/// Classify characters by type for semantic grouping
fn classify_characters(codes: &HashMap<u32, usize>) -> CharCodeClassification {
    let mut classification = CharCodeClassification::default();

    for (code, count) in codes {
        match *code {
            9 | 32 | 160 => classification.whitespace.push((*code, *count)),
            10 | 13 => classification.line_endings.push((*code, *count)),
            0..=31 | 127 => classification.control.push((*code, *count)),
            33..=126 => classification.printable.push((*code, *count)),
            _ => classification.unicode.push((*code, *count)),
        }
    }

    classification
}

/// Detect common whitespace/formatting issues
fn detect_whitespace_issues(
    codes: &HashMap<u32, usize>,
    expected_diff: &str,
    actual_diff: &str,
) -> Vec<WhitespaceIssue> {
    let mut issues = Vec::new();

    let has_tab = codes.contains_key(&9);
    let has_space = codes.contains_key(&32);
    let has_cr = codes.contains_key(&13);
    let has_lf = codes.contains_key(&10);

    if has_tab && has_space {
        issues.push(WhitespaceIssue::TabsVsSpaces);
    }

    if has_cr && has_lf {
        issues.push(WhitespaceIssue::MixedLineEndings);
    }

    if codes.get(&32).copied().unwrap_or(0) > 3 {
        issues.push(WhitespaceIssue::ExtraSpaces);
    }

    // Check for trailing whitespace
    if expected_diff
        .lines()
        .any(|line| line.ends_with(' ') || line.ends_with('\t'))
        || actual_diff
            .lines()
            .any(|line| line.ends_with(' ') || line.ends_with('\t'))
    {
        issues.push(WhitespaceIssue::TrailingWhitespace);
    }

    issues
}

/// Detect encoding-related issues
fn detect_encoding_issues(codes: &HashMap<u32, usize>) -> Vec<EncodingIssue> {
    let mut issues = Vec::new();

    if codes.contains_key(&0xFFFD) {
        issues.push(EncodingIssue::ReplacementChar);
    }

    if codes.contains_key(&0xFEFF) {
        issues.push(EncodingIssue::ByteOrderMark);
    }

    for code in codes.keys() {
        if (0xD800..=0xDFFF).contains(code) {
            issues.push(EncodingIssue::Utf16Surrogate);
            break;
        }
    }

    issues
}

/// Compare character distributions between expected and actual
fn compare_distribution(expected: &str, actual: &str) -> CharDistribution {
    let exp_codes = count_chars(expected);
    let act_codes = count_chars(actual);

    let mut dist = CharDistribution::default();

    for (code, exp_count) in &exp_codes {
        if let Some(act_count) = act_codes.get(code) {
            dist.in_both.push((*code, *exp_count, *act_count));
        } else {
            dist.only_in_expected.push((*code, *exp_count));
        }
    }

    for (code, act_count) in &act_codes {
        if !exp_codes.contains_key(code) {
            dist.only_in_actual.push((*code, *act_count));
        }
    }

    dist
}

/// Count character occurrences
fn count_chars(s: &str) -> HashMap<u32, usize> {
    let mut codes = HashMap::new();
    for ch in s.chars() {
        *codes.entry(ch as u32).or_insert(0) += 1;
    }
    codes
}

/// Analyze Unicode normalization
fn analyze_unicode(expected: &str, actual: &str) -> UnicodeAnalysis {
    let exp_nfc: String = expected.nfc().collect();
    let exp_nfd: String = expected.nfd().collect();
    let act_nfc: String = actual.nfc().collect();
    let act_nfd: String = actual.nfd().collect();

    UnicodeAnalysis {
        has_composed: exp_nfc == expected || act_nfc == actual,
        has_decomposed: exp_nfd != expected || act_nfd != actual,
        normalization_mismatch: exp_nfc == act_nfc && expected != actual,
    }
}

/// Generate smart fix suggestion based on detected issues
fn generate_suggestion(
    whitespace_issues: &[WhitespaceIssue],
    encoding_issues: &[EncodingIssue],
    has_zero_width: bool,
    unicode_analysis: &UnicodeAnalysis,
) -> Option<String> {
    if has_zero_width {
        return Some("Remove zero-width characters from your search string".to_string());
    }

    if unicode_analysis.normalization_mismatch {
        return Some("Normalize Unicode to NFC form in your search string".to_string());
    }

    if let Some(issue) = whitespace_issues.first() {
        match issue {
            WhitespaceIssue::TabsVsSpaces => {
                return Some(
                    "Replace tabs with spaces (or vice versa) in your search string".to_string(),
                );
            }
            WhitespaceIssue::MixedLineEndings => {
                return Some("Use consistent line endings (LF or CRLF, not mixed)".to_string());
            }
            WhitespaceIssue::ExtraSpaces => {
                return Some("Check for extra/missing spaces in your search string".to_string());
            }
            WhitespaceIssue::TrailingWhitespace => {
                return Some(
                    "Remove trailing whitespace from lines in your search string".to_string(),
                );
            }
        }
    }

    if let Some(issue) = encoding_issues.first() {
        match issue {
            EncodingIssue::ReplacementChar => {
                return Some("File contains invalid UTF-8 characters (�)".to_string());
            }
            EncodingIssue::ByteOrderMark => {
                return Some("Remove Byte Order Mark (BOM) from file".to_string());
            }
            EncodingIssue::Utf16Surrogate => {
                return Some("File contains invalid UTF-16 surrogate characters".to_string());
            }
        }
    }

    None
}

/// Format visual diff with inline character codes
fn format_visual_diff_with_codes(
    expected: &str,
    _actual: &str,
    expected_diff: &str,
    actual_diff: &str,
    prefix_len: usize,
    suffix_len: usize,
) -> String {
    let exp_chars: Vec<char> = expected.chars().collect();

    let prefix: String = exp_chars.iter().take(prefix_len).collect();
    let suffix: String = exp_chars
        .iter()
        .skip(exp_chars.len().saturating_sub(suffix_len))
        .collect();

    // Format as: prefix{-removed-}{+added+}suffix
    let mut output = String::new();
    output.push_str(&prefix);
    if !expected_diff.is_empty() {
        output.push_str(&format!("{{-{expected_diff}-}}"));
    }
    if !actual_diff.is_empty() {
        output.push_str(&format!("{{+{actual_diff}+}}"));
    }
    output.push_str(&suffix);

    // Add inline codes
    output.push_str("\n\nWith character codes:");
    output.push_str(&format!(
        "\nExpected diff: {:?} [{}]",
        expected_diff,
        inline_codes(expected_diff)
    ));
    output.push_str(&format!(
        "\nActual diff:   {:?} [{}]",
        actual_diff,
        inline_codes(actual_diff)
    ));

    output
}

/// Format string as comma-separated character codes
fn inline_codes(s: &str) -> String {
    if s.is_empty() {
        return String::from("empty");
    }
    s.chars()
        .map(|c| format!("{}", c as u32))
        .collect::<Vec<_>>()
        .join(",")
}

// ============================================================================
// PUBLIC FORMATTING METHODS
// ============================================================================

impl CharCodeData {
    /// Format complete analysis as structured text for AI agents
    ///
    /// Produces machine-parseable, human-readable output showing all analysis results
    #[must_use]
    pub fn format_detailed_report(&self) -> String {
        let mut output = String::new();

        output.push_str("Character Analysis:\n");
        output.push_str(&format!("  Character codes: {}\n", self.report));
        output.push_str(&format!(
            "  Unique codes: {}, Diff length: {}\n",
            self.unique_count, self.diff_length
        ));

        // Classification
        if !self.classification.whitespace.is_empty()
            || !self.classification.line_endings.is_empty()
        {
            output.push_str("\nCharacter Types:\n");

            if !self.classification.whitespace.is_empty() {
                let ws: Vec<String> = self
                    .classification
                    .whitespace
                    .iter()
                    .map(|(code, count)| format!("{}×{}", format_char_name(*code), count))
                    .collect();
                output.push_str(&format!("  Whitespace: {}\n", ws.join(", ")));
            }

            if !self.classification.line_endings.is_empty() {
                let le: Vec<String> = self
                    .classification
                    .line_endings
                    .iter()
                    .map(|(code, count)| format!("{}×{}", format_char_name(*code), count))
                    .collect();
                output.push_str(&format!("  Line endings: {}\n", le.join(", ")));
            }

            if !self.classification.control.is_empty() {
                output.push_str(&format!(
                    "  Control chars: {} types\n",
                    self.classification.control.len()
                ));
            }

            if !self.classification.unicode.is_empty() {
                output.push_str(&format!(
                    "  Unicode chars: {} types\n",
                    self.classification.unicode.len()
                ));
            }
        }

        // Issues detected
        if !self.whitespace_issues.is_empty()
            || !self.encoding_issues.is_empty()
            || self.has_zero_width
        {
            output.push_str("\nIssues Detected:\n");

            for issue in &self.whitespace_issues {
                output.push_str(&format!("  ⚠️  {issue:?}\n"));
            }

            for issue in &self.encoding_issues {
                output.push_str(&format!("  ⚠️  {issue:?}\n"));
            }

            if self.has_zero_width {
                output.push_str("  ⚠️  Zero-width characters detected\n");
            }

            if self.unicode_analysis.normalization_mismatch {
                output.push_str("  ⚠️  Unicode normalization mismatch (NFC vs NFD)\n");
            }
        }

        // Distribution
        if !self.distribution.only_in_expected.is_empty()
            || !self.distribution.only_in_actual.is_empty()
        {
            output.push_str("\nDistribution:\n");

            if !self.distribution.only_in_expected.is_empty() {
                let chars: Vec<String> = self
                    .distribution
                    .only_in_expected
                    .iter()
                    .map(|(code, count)| format!("{}×{}", format_char_name(*code), count))
                    .collect();
                output.push_str(&format!("  Only in search string: {}\n", chars.join(", ")));
            }

            if !self.distribution.only_in_actual.is_empty() {
                let chars: Vec<String> = self
                    .distribution
                    .only_in_actual
                    .iter()
                    .map(|(code, count)| format!("{}×{}", format_char_name(*code), count))
                    .collect();
                output.push_str(&format!("  Only in found string: {}\n", chars.join(", ")));
            }
        }

        // Visual diff with codes
        output.push_str("\nVisual Diff:\n");
        output.push_str(&self.visual_diff_with_codes);
        output.push('\n');

        // Suggestion
        if let Some(ref suggestion) = self.suggestion {
            output.push_str(&format!("\n💡 Suggestion: {suggestion}\n"));
        }

        output
    }
}
