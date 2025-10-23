//! Branch types and constants.
//!
//! This module contains data structures and constants used for branch operations.

pub(super) const REFS_HEADS_PREFIX: &str = "refs/heads/";
pub(super) const REFS_REMOTES_PREFIX: &str = "refs/remotes/";

/// Options for `branch` operation with builder pattern.
#[derive(Debug, Clone)]
pub struct BranchOpts {
    pub name: String,
    pub start_point: Option<String>,
    pub force: bool,
    pub checkout: bool,
    pub track: bool,
}

impl BranchOpts {
    /// Create new branch options with the given name.
    #[inline]
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            start_point: None,
            force: false,
            checkout: false,
            track: false,
        }
    }

    /// Set the start point (commit, branch, or tag) for the new branch.
    #[inline]
    pub fn start_point<S: Into<String>>(mut self, start_point: S) -> Self {
        self.start_point = Some(start_point.into());
        self
    }

    /// Enable force mode (overwrite existing branch).
    #[inline]
    #[must_use]
    pub fn force(mut self, enabled: bool) -> Self {
        self.force = enabled;
        self
    }

    /// Checkout the branch after creation.
    #[inline]
    #[must_use]
    pub fn checkout(mut self, enabled: bool) -> Self {
        self.checkout = enabled;
        self
    }

    /// Set up tracking relationship with upstream branch.
    #[inline]
    #[must_use]
    pub fn track(mut self, enabled: bool) -> Self {
        self.track = enabled;
        self
    }
}
