//! Git fetch operation with comprehensive options.
//!
//! This module provides the `FetchOpts` builder pattern and fetch operation
//! implementation for the `GitGix` service.

use std::collections::HashSet;
use std::sync::atomic::AtomicBool;

use gix::bstr::ByteSlice;
use gix::progress::Discard;
use gix::remote::ref_map;

use crate::{GitError, GitResult, RepoHandle};

/// Options for `fetch` operation with builder pattern.
#[derive(Debug, Clone)]
pub struct FetchOpts {
    pub remote: String,
    pub refspecs: Vec<String>,
    pub prune: bool,
}

impl FetchOpts {
    /// Create new fetch options for the given remote.
    #[inline]
    pub fn from_remote<R: Into<String>>(remote: R) -> Self {
        Self {
            remote: remote.into(),
            refspecs: Vec::new(),
            prune: false,
        }
    }

    /// Add a refspec to fetch.
    pub fn add_refspec<S: Into<String>>(mut self, spec: S) -> Self {
        self.refspecs.push(spec.into());
        self
    }

    /// Enable pruning of remote-tracking branches.
    #[must_use]
    pub fn prune(mut self, yes: bool) -> Self {
        self.prune = yes;
        self
    }
}

impl Default for FetchOpts {
    fn default() -> Self {
        Self {
            remote: "origin".to_string(),
            refspecs: Vec::new(),
            prune: false,
        }
    }
}

/// Execute fetch operation with the given options.
pub async fn fetch(repo: RepoHandle, opts: FetchOpts) -> GitResult<()> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let FetchOpts {
            remote,
            refspecs,
            prune,
        } = opts;

        // Store remote name for pruning
        let remote_name = remote.clone();

        // Find the remote
        let remote_bstr = remote.as_bytes().as_bstr();
        let remote_ref = repo_clone
            .find_remote(remote_bstr)
            .map_err(|e| GitError::InvalidInput(format!("Remote '{remote}' not found: {e}")))?;

        // Connect to the remote
        let connection = remote_ref
            .connect(gix::remote::Direction::Fetch)
            .map_err(|e| GitError::Gix(e.into()))?;

        // Parse custom refspecs if provided
        let parsed_refspecs = if refspecs.is_empty() {
            Vec::new()
        } else {
            refspecs
                .iter()
                .map(|spec| {
                    gix::refspec::parse(
                        spec.as_bytes().as_bstr(),
                        gix::refspec::parse::Operation::Fetch,
                    )
                    .map(|r| r.to_owned())
                    .map_err(|e| GitError::InvalidInput(format!("Invalid refspec '{spec}': {e}")))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        // Create ref_map options with custom refspecs
        let ref_map_options = ref_map::Options {
            extra_refspecs: parsed_refspecs,
            ..Default::default()
        };

        // Prepare fetch operation
        let fetch_prep = connection
            .prepare_fetch(Discard, ref_map_options)
            .map_err(|e| GitError::Gix(e.into()))?;

        // Execute the fetch
        let outcome = fetch_prep
            .receive(Discard, &AtomicBool::new(false))
            .map_err(|e| GitError::Gix(e.into()))?;

        // Implement pruning if enabled
        if prune {
            prune_stale_refs(&repo_clone, &remote_name, &outcome.ref_map)?;
        }

        Ok(())
    })
    .await
    .map_err(|e| GitError::InvalidInput(format!("Task join error: {e}")))?
}

/// Helper function to prune stale remote-tracking refs
fn prune_stale_refs(
    repo: &gix::Repository,
    remote_name: &str,
    ref_map: &gix::remote::fetch::RefMap,
) -> GitResult<()> {
    use gix::protocol::handshake::Ref;

    // Build set of branch names that exist on the remote
    // We extract branch names from refs/heads/* on the remote
    let remote_branches: HashSet<String> = ref_map
        .remote_refs
        .iter()
        .filter_map(|remote_ref| {
            // Get the full name of the remote reference based on the variant
            let full_ref_name: &gix::bstr::BStr = match remote_ref {
                Ref::Peeled { full_ref_name, .. } => full_ref_name.as_ref(),
                Ref::Direct { full_ref_name, .. } => full_ref_name.as_ref(),
                Ref::Symbolic { full_ref_name, .. } => full_ref_name.as_ref(),
                Ref::Unborn { full_ref_name, .. } => full_ref_name.as_ref(),
            };

            // Only process refs/heads/* (branches)
            full_ref_name
                .strip_prefix(b"refs/heads/")
                .map(|branch_name| branch_name.to_str_lossy().into_owned())
        })
        .collect();

    // List all local remote-tracking refs for this remote
    let prefix = format!("refs/remotes/{remote_name}/");
    let mut refs_to_delete = Vec::new();

    // Iterate through local remote-tracking refs
    let all_refs = repo.references().map_err(|e| GitError::Gix(e.into()))?;

    for reference_result in all_refs.all().map_err(|e| GitError::Gix(e.into()))? {
        let reference = reference_result.map_err(GitError::Gix)?;
        let ref_name = reference.name().as_bstr().to_str_lossy();

        // Check if this is a remote-tracking ref for our remote
        if let Some(branch_name) = ref_name.strip_prefix(&prefix) {
            // If the branch doesn't exist on the remote, mark it for deletion
            if !remote_branches.contains(branch_name) {
                refs_to_delete.push(reference);
            }
        }
    }

    // Delete stale refs
    for reference in refs_to_delete {
        reference.delete().map_err(|e| GitError::Gix(e.into()))?;
    }

    Ok(())
}
