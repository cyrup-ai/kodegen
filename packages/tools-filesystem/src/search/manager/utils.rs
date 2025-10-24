//! Shared utilities for file and content search implementations
//!
//! This module provides common helper functions to reduce code duplication
//! between `file_search` and `content_search` modules.

use super::super::rg::flags::lowargs::{CaseMode as RgCaseMode, TypeChange};
use super::super::types::{CaseMode as MyCaseMode, SearchSessionOptions};
use ignore::WalkBuilder;

/// Build ripgrep `TypeChange` vector from `SearchSessionOptions`
///
/// Converts MCP `type/type_not` parameters to ripgrep's `TypeChange` format.
/// Used by both `file_search` and `content_search` implementations.
///
/// # Arguments
/// * `options` - Search session options containing type filters
///
/// # Returns
/// Vector of `TypeChange` entries for ripgrep configuration
pub(super) fn build_type_changes(options: &SearchSessionOptions) -> Vec<TypeChange> {
    let mut type_changes = Vec::with_capacity(options.r#type.len() + options.type_not.len());

    // Add selected types (--type rust, --type python, etc.)
    for type_name in &options.r#type {
        type_changes.push(TypeChange::Select {
            name: type_name.clone(),
        });
    }

    // Add negated types (--type-not test, --type-not json, etc.)
    for type_name in &options.type_not {
        type_changes.push(TypeChange::Negate {
            name: type_name.clone(),
        });
    }

    type_changes
}

/// Convert MCP `CaseMode` to ripgrep `CaseMode`
///
/// Maps the MCP case sensitivity enum to ripgrep's equivalent enum.
/// Used by both `file_search` and `content_search` implementations.
///
/// # Arguments
/// * `mode` - MCP case mode from search options
///
/// # Returns
/// Ripgrep `CaseMode` equivalent
pub(super) fn convert_case_mode(mode: MyCaseMode) -> RgCaseMode {
    match mode {
        MyCaseMode::Sensitive => RgCaseMode::Sensitive,
        MyCaseMode::Insensitive => RgCaseMode::Insensitive,
        MyCaseMode::Smart => RgCaseMode::Smart,
    }
}

/// Configure `WalkBuilder` with `HiArgs` settings
///
/// Sets up directory walker using ripgrep-compatible configuration from `HiArgs`.
/// Properly respects `no_ignore`_* flags matching ripgrep's --no-ignore behavior.
///
/// # Arguments
/// * `walker` - `WalkBuilder` to configure
/// * `hi_args` - High-level ripgrep arguments containing all ignore settings
pub(super) fn configure_walker(
    walker: &mut WalkBuilder,
    hi_args: &super::super::rg::flags::hiargs::HiArgs,
) {
    log::debug!(
        "configure_walker: hi_args.hidden = {}, setting walker.hidden({})",
        hi_args.hidden,
        !hi_args.hidden
    );
    walker
        .hidden(!hi_args.hidden)
        .parents(!hi_args.no_ignore_parent)
        .ignore(!hi_args.no_ignore_dot)
        .git_global(!hi_args.no_ignore_vcs && !hi_args.no_ignore_global)
        .git_ignore(!hi_args.no_ignore_vcs)
        .git_exclude(!hi_args.no_ignore_vcs && !hi_args.no_ignore_exclude)
        .threads(0);

    if let Some(size) = hi_args.max_filesize {
        walker.max_filesize(Some(size));
    }

    if let Some(depth) = hi_args.max_depth {
        walker.max_depth(Some(depth));
    }
}
