//! User-facing suggestion system for `edit_block` failures
//!
//! Provides actionable guidance when edit operations fail, matching
//! the helpful UX of Desktop Commander's error messages.

use std::path::PathBuf;

// ============================================================================
// FAILURE REASONS
// ============================================================================

/// Reasons why an `edit_block` operation might fail
#[derive(Debug, Clone)]
pub enum EditFailureReason {
    /// No match found at all
    NoMatchFound,

    /// Fuzzy match found but below similarity threshold
    FuzzyMatchBelowThreshold {
        similarity: f64,
        threshold: f64,
        found_text: String,
    },

    /// Fuzzy match found above threshold (not actually a failure, but user needs guidance)
    FuzzyMatchAboveThreshold {
        similarity: f64,
        is_whitespace_only: bool,
    },

    /// Unexpected number of occurrences
    UnexpectedCount { expected: usize, found: usize },

    /// Empty search string
    EmptySearch,

    /// Identical old and new strings
    IdenticalStrings,
}

// ============================================================================
// SUGGESTION BUILDER
// ============================================================================

/// Context needed to build helpful suggestions
#[derive(Debug, Clone)]
pub struct SuggestionContext {
    pub file_path: String,
    pub search_string: String,
    pub line_number: Option<usize>,

    // Future hooks for EDIT_05 and EDIT_06
    pub log_path: Option<PathBuf>,
    pub execution_time_ms: Option<f64>,
}

/// Actionable suggestion for users
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Main message explaining what happened
    pub message: String,

    /// List of actionable steps the user can take
    pub actions: Vec<String>,
}

impl Suggestion {
    /// Build a suggestion for a specific failure reason
    #[must_use]
    pub fn for_failure(reason: &EditFailureReason, context: &SuggestionContext) -> Self {
        match reason {
            EditFailureReason::FuzzyMatchAboveThreshold {
                similarity,
                is_whitespace_only,
            } => Self::fuzzy_match_above_threshold(*similarity, *is_whitespace_only, context),

            EditFailureReason::FuzzyMatchBelowThreshold {
                similarity,
                threshold,
                found_text,
            } => Self::fuzzy_match_below_threshold(*similarity, *threshold, found_text, context),

            EditFailureReason::UnexpectedCount { expected, found } => {
                Self::unexpected_count(*expected, *found, context)
            }

            EditFailureReason::NoMatchFound => Self::no_match_found(context),

            EditFailureReason::EmptySearch => Self::empty_search(),

            EditFailureReason::IdenticalStrings => Self::identical_strings(),
        }
    }

    /// Format the complete suggestion message
    #[must_use]
    pub fn format(&self) -> String {
        let mut output = String::new();

        if !self.actions.is_empty() {
            output.push_str("\n💡 Suggestions:\n");
            for (i, action) in self.actions.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, action));
            }
        }

        output
    }

    // ========================================================================
    // PRIVATE BUILDERS FOR EACH SCENARIO
    // ========================================================================

    fn fuzzy_match_above_threshold(
        similarity: f64,
        is_whitespace_only: bool,
        context: &SuggestionContext,
    ) -> Self {
        let mut actions = vec![
            "Copy the exact text from the diff above".to_string(),
            "   Example: Use the text after {+...+} in the diff".to_string(),
        ];

        if is_whitespace_only {
            actions.push(
                "The difference is only whitespace - check for extra/missing spaces or tabs"
                    .to_string(),
            );
        }

        // Future hook for EDIT_05
        if let Some(ref log_path) = context.log_path {
            actions.push(format!(
                "For detailed analysis, check log: {}",
                log_path.display()
            ));
        }

        let mut message = if let Some(line_num) = context.line_number {
            format!(
                "Exact match not found in {}, but found similar text at line {} with {:.1}% similarity",
                context.file_path,
                line_num,
                similarity * 100.0
            )
        } else {
            format!(
                "Exact match not found in {}, but found similar text with {:.1}% similarity",
                context.file_path,
                similarity * 100.0
            )
        };

        // Add execution time if available (EDIT_06 hook)
        if let Some(time_ms) = context.execution_time_ms {
            message.push_str(&format!(" (found in {time_ms:.2}ms)"));
        }

        message.push('.');

        Self { message, actions }
    }

    fn fuzzy_match_below_threshold(
        similarity: f64,
        threshold: f64,
        found_text: &str,
        context: &SuggestionContext,
    ) -> Self {
        let mut message = format!(
            "Search content not found in {}. The closest match was \"{}\" \
             with only {:.1}% similarity, which is below the {:.1}% threshold",
            context.file_path,
            found_text,
            similarity * 100.0,
            threshold * 100.0
        );

        // Add execution time if available (EDIT_06 hook)
        if let Some(time_ms) = context.execution_time_ms {
            message.push_str(&format!(" (search completed in {time_ms:.2}ms)"));
        }

        message.push('.');

        let mut actions = vec![
            "Check if you're searching in the correct file".to_string(),
            "Try a smaller, more unique search string".to_string(),
            "   Example: Instead of searching for entire function, search for a unique line within it".to_string(),
            "Check for typos in your search string".to_string(),
        ];

        // Future hook for EDIT_05
        if let Some(ref log_path) = context.log_path {
            actions.push(format!(
                "For detailed analysis, check log: {}",
                log_path.display()
            ));
        }

        Self { message, actions }
    }

    fn unexpected_count(expected: usize, found: usize, _context: &SuggestionContext) -> Self {
        let message = format!("Expected to replace {expected} occurrence(s), but found {found}.");

        let actions = if found > expected {
            vec![
                format!(
                    "If you want to replace all {} occurrences, set expected_replacements to {}",
                    found, found
                ),
                format!("   Example: edit_block(..., expected_replacements: {})", found),
                "To replace specific occurrences, make your search string more unique by including surrounding context".to_string(),
                "   Example: Instead of 'foo', use 'function bar() {\\n  foo\\n}'".to_string(),
            ]
        } else {
            vec![
                "Some occurrences may have already been replaced".to_string(),
                "Check if the file content has changed since you last read it".to_string(),
                format!(
                    "Update expected_replacements to {} to match actual count",
                    found
                ),
            ]
        };

        Self { message, actions }
    }

    fn no_match_found(context: &SuggestionContext) -> Self {
        let message = format!(
            "No occurrences of the search string found in {}",
            context.file_path
        );

        let actions = vec![
            "Verify you're searching in the correct file".to_string(),
            "Check for typos in your search string".to_string(),
            "Try searching for a smaller, more specific substring".to_string(),
        ];

        Self { message, actions }
    }

    fn empty_search() -> Self {
        Self {
            message: "Empty search strings are not allowed.".to_string(),
            actions: vec!["Provide a non-empty string to search for".to_string()],
        }
    }

    fn identical_strings() -> Self {
        Self {
            message: "old_string and new_string are identical.".to_string(),
            actions: vec!["No changes would be made - provide different strings".to_string()],
        }
    }
}
