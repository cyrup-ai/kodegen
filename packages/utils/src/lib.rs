pub mod char_analysis;
pub mod char_diff;
pub mod edit_log;
pub mod fuzzy_logger;
pub mod fuzzy_search;
pub mod line_endings;
pub mod suggestions;
pub mod usage_tracker;

// Re-export commonly used types
pub use edit_log::{EditBlockLogEntry, EditBlockLogger, EditBlockResult, get_edit_logger};

pub use fuzzy_logger::{FuzzyLogger, FuzzySearchLogEntry, get_logger};

pub use char_analysis::{
    CharCodeClassification, CharCodeData, CharDistribution, EncodingIssue, UnicodeAnalysis,
    WhitespaceIssue,
};
