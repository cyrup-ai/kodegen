//! Shared utilities for file and content search implementations
//!
//! This module provides common helper functions to reduce code duplication
//! between file_search and content_search modules.

use super::super::rg::flags::lowargs::{TypeChange, CaseMode as RgCaseMode};
use super::super::types::{SearchSessionOptions, CaseMode as MyCaseMode};
use ignore::WalkBuilder;

/// Build ripgrep TypeChange vector from SearchSessionOptions
///
/// Converts MCP type/type_not parameters to ripgrep's TypeChange format.
/// Used by both file_search and content_search implementations.
///
/// # Arguments
/// * `options` - Search session options containing type filters
///
/// # Returns
/// Vector of TypeChange entries for ripgrep configuration
pub(super) fn build_type_changes(options: &SearchSessionOptions) -> Vec<TypeChange> {
    let mut type_changes = Vec::with_capacity(
        options.r#type.len() + options.type_not.len()
    );

    // Add selected types (--type rust, --type python, etc.)
    for type_name in &options.r#type {
        type_changes.push(TypeChange::Select {
            name: type_name.clone()
        });
    }

    // Add negated types (--type-not test, --type-not json, etc.)
    for type_name in &options.type_not {
        type_changes.push(TypeChange::Negate {
            name: type_name.clone()
        });
    }

    type_changes
}

/// Convert MCP CaseMode to ripgrep CaseMode
///
/// Maps the MCP case sensitivity enum to ripgrep's equivalent enum.
/// Used by both file_search and content_search implementations.
///
/// # Arguments
/// * `mode` - MCP case mode from search options
///
/// # Returns
/// Ripgrep CaseMode equivalent
pub(super) fn convert_case_mode(mode: MyCaseMode) -> RgCaseMode {
    match mode {
        MyCaseMode::Sensitive => RgCaseMode::Sensitive,
        MyCaseMode::Insensitive => RgCaseMode::Insensitive,
        MyCaseMode::Smart => RgCaseMode::Smart,
    }
}

/// Configure WalkBuilder with common search options
///
/// Sets up directory walker with gitignore support, file size limits,
/// and depth restrictions. Used by both file_search and content_search.
///
/// # Arguments
/// * `walker` - WalkBuilder to configure
/// * `options` - Search session options with traversal settings
pub(super) fn configure_walker(
    walker: &mut WalkBuilder,
    options: &SearchSessionOptions,
) {
    walker
        .hidden(!options.include_hidden)
        .git_ignore(true)       // Respect .gitignore files
        .git_exclude(true)      // Respect .git/info/exclude
        .git_global(true)       // Respect global gitignore
        .parents(true)          // CRITICAL: Respect parent directory .gitignore files
        .threads(0);            // 0 = auto-detect CPU cores

    // Add max_filesize support (skip files larger than limit)
    if let Some(size) = options.max_filesize {
        walker.max_filesize(Some(size));
    }

    // Add max_depth support (limit directory traversal depth)
    if let Some(depth) = options.max_depth {
        walker.max_depth(Some(depth));
    }
}
