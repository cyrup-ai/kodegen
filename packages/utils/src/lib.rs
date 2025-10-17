pub mod fuzzy_search;
pub mod usage_tracker;
pub mod edit_log;
pub mod char_diff;
pub mod line_endings;
pub mod suggestions;
pub mod fuzzy_logger;
pub mod char_analysis;

// Re-export commonly used types
pub use edit_log::{
    EditBlockLogEntry,
    EditBlockResult,
    EditBlockLogger,
    get_edit_logger,
};

pub use fuzzy_logger::{
    FuzzyLogger,
    FuzzySearchLogEntry,
    get_logger,
};

pub use char_analysis::{
    CharCodeData,
    CharCodeClassification,
    WhitespaceIssue,
    EncodingIssue,
    CharDistribution,
    UnicodeAnalysis,
};
