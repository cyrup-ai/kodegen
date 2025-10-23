//! Helper utility functions

/// Helper function to check if entry is hidden
pub(crate) fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.'))
}

/// Helper function to check if entry is .git directory
pub(crate) fn is_git_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_name().to_str() == Some(".git")
}

/// Helper function to check if entry is `vendor/node_modules`
pub(crate) fn is_vendor_dir(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_str().unwrap_or("");
    name == "node_modules" || name == "vendor" || name == "target"
}

/// Calculate structure quality score
pub(crate) fn calculate_structure_score(
    has_src: bool,
    has_lib: bool,
    has_tests: bool,
    has_docs: bool,
    has_examples: bool,
    has_bin: bool,
) -> f32 {
    let mut score: f32 = 0.0;

    if has_src || has_lib {
        score += 0.3;
    }
    if has_tests {
        score += 0.25;
    }
    if has_docs {
        score += 0.2;
    }
    if has_examples {
        score += 0.1;
    }
    if has_bin {
        score += 0.05;
    }
    if (has_src || has_lib) && has_tests {
        score += 0.1;
    }

    score.min(1.0)
}
