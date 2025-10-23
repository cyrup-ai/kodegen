//! Git clone operation with comprehensive options.
//!
//! This module provides the `CloneOpts` builder pattern and clone operation
//! implementation for the `GitGix` service.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use gix::progress::Discard;
use gix::remote;

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

/// Shared cancellation token for operations that don't need interruption.
static NEVER_INTERRUPT: AtomicBool = AtomicBool::new(false);

/// Options for `clone` operation with builder pattern.
#[derive(Debug, Clone)]
pub struct CloneOpts {
    pub url: String,
    pub destination: PathBuf,
    pub shallow: Option<u32>,
    pub branch: Option<String>,
    pub bare: bool,
}

impl CloneOpts {
    /// Create new clone options with URL and destination.
    #[inline]
    pub fn new<U: Into<String>, P: Into<PathBuf>>(url: U, destination: P) -> Self {
        Self {
            url: url.into(),
            destination: destination.into(),
            shallow: None,
            branch: None,
            bare: false,
        }
    }

    /// Set shallow clone depth.
    #[inline]
    #[must_use]
    pub fn shallow(mut self, depth: u32) -> Self {
        self.shallow = Some(depth);
        self
    }

    /// Set specific branch to clone.
    #[inline]
    pub fn branch<S: Into<String>>(mut self, branch: S) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Set bare repository flag.
    #[inline]
    #[must_use]
    pub fn bare(mut self, yes: bool) -> Self {
        self.bare = yes;
        self
    }
}

/// Execute clone operation with the given options.
#[must_use]
pub fn clone_repo(opts: CloneOpts) -> AsyncTask<GitResult<RepoHandle>> {
    AsyncTask::spawn(move || {
        let CloneOpts {
            url,
            destination,
            shallow,
            branch,
            bare,
        } = opts;

        // Validate parent directory exists (cheap syscall before expensive operations)
        if let Some(parent) = destination.parent()
            && !parent.exists()
        {
            return Err(GitError::InvalidInput(format!(
                "Parent directory does not exist: {}",
                parent.display()
            )));
        }

        // Check if destination already exists
        if destination.exists() {
            return Err(GitError::InvalidInput(format!(
                "Destination path already exists: {}",
                destination.display()
            )));
        }

        // Parse URL
        let parsed_url = gix::url::parse(url.as_str().into())
            .map_err(|e| GitError::InvalidInput(format!("Invalid URL '{url}': {e}")))?;

        // Prepare clone operation
        let mut prepare = gix::prepare_clone(parsed_url, &destination).map_err(GitError::from)?;

        // Configure shallow clone if requested (NonZeroU32 validates depth > 0)
        if let Some(depth) = shallow {
            let depth_value = NonZeroU32::new(depth).ok_or_else(|| {
                GitError::InvalidInput("Shallow clone depth must be greater than 0".to_string())
            })?;
            prepare = prepare.with_shallow(remote::fetch::Shallow::DepthAtRemote(depth_value));
        }

        // Configure specific branch if requested
        if let Some(branch_name) = branch.as_deref() {
            prepare = prepare
                .with_ref_name(Some(branch_name))
                .map_err(|e| GitError::Gix(Box::new(e)))?;
        }

        // Execute fetch with appropriate method based on bare flag
        let repo = if bare {
            // Bare clone: fetch only, no working tree
            let (repo, _outcome) = prepare
                .fetch_only(Discard, &NEVER_INTERRUPT)
                .map_err(|e| GitError::Gix(Box::new(e)))?;
            repo
        } else {
            // Full clone: fetch and checkout working tree
            let (mut prepare_checkout, _outcome) = prepare
                .fetch_then_checkout(Discard, &NEVER_INTERRUPT)
                .map_err(|e| GitError::Gix(Box::new(e)))?;

            let (repo, _outcome) = prepare_checkout
                .main_worktree(Discard, &NEVER_INTERRUPT)
                .map_err(|e| GitError::Gix(Box::new(e)))?;
            repo
        };

        Ok(RepoHandle::new(repo))
    })
}
