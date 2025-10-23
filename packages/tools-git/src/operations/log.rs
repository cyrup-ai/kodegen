//! Git log operation with comprehensive options.
//!
//! This module provides the `LogOpts` builder pattern and log operation
//! implementation for the `GitGix` service.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use crate::runtime::AsyncStream;
use crate::{CommitInfo, GitError, GitResult, RepoHandle, Signature};

/// Options for `log` operation with builder pattern.
#[derive(Debug, Clone)]
pub struct LogOpts {
    pub max_count: Option<usize>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub path: Option<PathBuf>,
}

impl LogOpts {
    /// Create new log options.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_count: None,
            since: None,
            until: None,
            path: None,
        }
    }

    /// Set maximum number of commits to return.
    #[inline]
    #[must_use]
    pub fn max_count(mut self, count: usize) -> Self {
        self.max_count = Some(count);
        self
    }

    /// Set start date filter (only commits after this date).
    #[inline]
    #[must_use]
    pub fn since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    /// Set end date filter (only commits before this date).
    #[inline]
    #[must_use]
    pub fn until(mut self, until: DateTime<Utc>) -> Self {
        self.until = Some(until);
        self
    }

    /// Set path filter (only commits affecting this path).
    #[inline]
    pub fn path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.path = Some(path.into());
        self
    }
}

impl Default for LogOpts {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute log operation with the given options, returning a stream of commits.
pub fn log(repo: RepoHandle, opts: LogOpts) -> AsyncStream<GitResult<CommitInfo>> {
    let (tx, rx) = mpsc::unbounded_channel();
    let repo = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let LogOpts {
            max_count,
            since,
            until,
            path,
        } = opts;

        // Normalize path if provided
        let normalized_path = if let Some(ref p) = path {
            Some(match normalize_path(&repo, p) {
                Ok(normalized) => normalized,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            })
        } else {
            None
        };

        // Create revision walker
        let head_id = match repo.head_id() {
            Ok(id) => id,
            Err(e) => {
                let _ = tx.send(Err(GitError::Gix(Box::new(e))));
                return;
            }
        };
        let rev_walk = match repo.rev_walk([head_id.detach()]).all() {
            Ok(walker) => walker,
            Err(e) => {
                let _ = tx.send(Err(GitError::Gix(e.into())));
                return;
            }
        };

        let mut count = 0;

        // Stream commits one at a time
        for commit_result in rev_walk {
            // Check max_count limit
            if let Some(max) = max_count
                && count >= max
            {
                break;
            }

            match commit_result {
                Ok(info) => {
                    match repo.find_object(info.id).map(gix::Object::into_commit) {
                        Ok(commit) => {
                            // Get commit time with proper error handling
                            let time = match commit.time() {
                                Ok(t) => t,
                                Err(e) => {
                                    let _ = tx.send(Err(GitError::Gix(Box::new(e))));
                                    continue;
                                }
                            };

                            let commit_time = {
                                use chrono::TimeZone;
                                if let Some(t) = Utc.timestamp_opt(time.seconds, 0).single() {
                                    t
                                } else {
                                    let _ = tx.send(Err(GitError::InvalidInput(format!(
                                        "Invalid timestamp {} for commit {}",
                                        time.seconds, info.id
                                    ))));
                                    continue;
                                }
                            };

                            // Apply time filters (cheapest checks first)
                            if let Some(since_time) = since
                                && commit_time < since_time
                            {
                                continue;
                            }

                            if let Some(until_time) = until
                                && commit_time > until_time
                            {
                                continue;
                            }

                            // Apply path filter if specified (most expensive check)
                            if let Some(ref filter_path) = normalized_path {
                                match commit_touches_path(&repo, &commit, filter_path) {
                                    Ok(touches) => {
                                        if !touches {
                                            continue;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Err(e));
                                        return;
                                    }
                                }
                            }

                            // Get author information only after all filters pass
                            let author_sig = match commit.author() {
                                Ok(sig) => sig,
                                Err(e) => {
                                    let _ = tx.send(Err(GitError::Gix(Box::new(e))));
                                    continue;
                                }
                            };

                            let author_owned = match author_sig.to_owned() {
                                Ok(sig) => sig,
                                Err(e) => {
                                    let _ = tx.send(Err(GitError::Gix(Box::new(e))));
                                    continue;
                                }
                            };

                            use gix::bstr::ByteSlice;
                            let commit_info = CommitInfo {
                                id: info.id,
                                author: Signature::from(author_owned),
                                summary: commit
                                    .message()
                                    .map(|msg| msg.summary().as_bstr().to_string())
                                    .unwrap_or_default(),
                                time: commit_time,
                            };

                            // Send to stream - if receiver dropped, stop
                            if tx.send(Ok(commit_info)).is_err() {
                                break;
                            }
                            count += 1;
                        }
                        Err(e) => {
                            let _ = tx.send(Err(GitError::Gix(e.into())));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(GitError::Gix(e.into())));
                }
            }
        }
    });

    AsyncStream::new(rx)
}

/// Normalize path to repo-relative format
fn normalize_path(repo: &gix::Repository, path: &std::path::Path) -> GitResult<PathBuf> {
    // Get repository workdir
    let workdir = repo.workdir().ok_or_else(|| {
        GitError::InvalidInput("Cannot filter by path in bare repository".to_string())
    })?;

    // Make path absolute if it's relative
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| GitError::InvalidInput(format!("Cannot get current directory: {e}")))?
            .join(path)
    };

    // Convert to repo-relative
    let relative_path = absolute_path.strip_prefix(workdir).map_err(|_| {
        GitError::InvalidInput(format!("Path {} is not within repository", path.display()))
    })?;

    // Validate no escape attempts
    if relative_path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(GitError::InvalidInput(
            "Path cannot contain '..' components".to_string(),
        ));
    }

    Ok(relative_path.to_path_buf())
}

/// Check if a change location matches the filter path.
///
/// Performs path matching with the following semantics:
/// - Exact match: `src/main.rs` matches `src/main.rs`
/// - Directory match: `src` matches `src/main.rs` and `src/lib.rs`
/// - Directory with trailing slash: `src/` matches `src/main.rs`
/// - Non-match: `src` does not match `src2/main.rs`
///
/// This function is heavily optimized for performance as it's called
/// in the hot path of tree diff operations. Uses platform-specific
/// byte access to avoid UTF-8 validation overhead.
#[inline(always)]
fn change_matches_path(change_location: &gix::bstr::BStr, filter_path: &std::path::Path) -> bool {
    // Platform-optimized byte extraction to avoid UTF-8 validation
    #[cfg(unix)]
    let filter_bytes = {
        use std::os::unix::ffi::OsStrExt;
        filter_path.as_os_str().as_bytes()
    };

    #[cfg(windows)]
    let filter_bytes = {
        use std::os::windows::ffi::OsStrExt;
        // Windows paths are UTF-16, need to convert to UTF-8 bytes
        // Fall back to string conversion
        match filter_path.to_str() {
            Some(s) => s.as_bytes(),
            None => return false,
        }
    };

    #[cfg(not(any(unix, windows)))]
    let filter_bytes = {
        match filter_path.to_str() {
            Some(s) => s.as_bytes(),
            None => return false,
        }
    };

    // Exact file match
    if change_location == filter_bytes {
        return true;
    }

    // Directory prefix match - check if path ends with '/'
    let has_trailing_slash = filter_bytes.last() == Some(&b'/');

    if has_trailing_slash {
        // Already has trailing slash: "src/" matches "src/file.rs"
        change_location.starts_with(filter_bytes)
    } else {
        // No trailing slash: "src" should match "src/file.rs" but not "src2/file.rs"
        change_location.starts_with(filter_bytes)
            && change_location.len() > filter_bytes.len()
            && change_location[filter_bytes.len()] == b'/'
    }
}

/// Check if a commit modifies the specified path.
///
/// For root commits (no parents), checks if the path exists in the commit's tree.
/// For regular commits, diffs against each parent to detect changes.
/// For merge commits with multiple parents, returns true if the path was modified
/// relative to *any* parent.
///
/// Uses zero-allocation iteration over parent IDs and early-exit optimization
/// to minimize overhead in the path filtering hot path.
#[inline]
fn commit_touches_path(
    repo: &gix::Repository,
    commit: &gix::Commit,
    filter_path: &std::path::Path,
) -> GitResult<bool> {
    // Get commit's tree
    let commit_tree = commit.tree().map_err(|e| GitError::Gix(Box::new(e)))?;

    // Peek at parent count without allocation
    let mut parent_iter = commit.parent_ids();
    let first_parent = match parent_iter.next() {
        Some(p) => p,
        None => {
            // Root commit (no parents) - check if path exists in tree
            return Ok(commit_tree
                .lookup_entry_by_path(filter_path)
                .map_err(|e| GitError::Gix(Box::new(e)))?
                .is_some());
        }
    };

    // Compare with first parent
    let parent_obj = repo
        .find_object(first_parent.detach())
        .map_err(|e| GitError::Gix(Box::new(e)))?;
    let parent_commit = parent_obj
        .try_into_commit()
        .map_err(|e| GitError::Gix(Box::new(e)))?;
    let parent_tree = parent_commit
        .tree()
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    if check_tree_diff_touches_path(&commit_tree, &parent_tree, filter_path)? {
        return Ok(true);
    }

    // For merge commits, check remaining parents
    for parent_id in parent_iter {
        let parent_obj = repo
            .find_object(parent_id.detach())
            .map_err(|e| GitError::Gix(Box::new(e)))?;
        let parent_commit = parent_obj
            .try_into_commit()
            .map_err(|e| GitError::Gix(Box::new(e)))?;
        let parent_tree = parent_commit
            .tree()
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        if check_tree_diff_touches_path(&commit_tree, &parent_tree, filter_path)? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Helper function to check if a tree diff touches the specified path.
/// Extracted to avoid code duplication and enable potential inlining.
#[inline]
fn check_tree_diff_touches_path(
    commit_tree: &gix::Tree,
    parent_tree: &gix::Tree,
    filter_path: &std::path::Path,
) -> GitResult<bool> {
    let mut touched = false;
    let mut diff_platform = commit_tree
        .changes()
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    diff_platform
        .for_each_to_obtain_tree(parent_tree, |change| {
            use gix::object::tree::diff::{Action, Change};

            let location = match change {
                Change::Addition { location, .. } => location,
                Change::Deletion { location, .. } => location,
                Change::Modification { location, .. } => location,
                Change::Rewrite { location, .. } => location,
            };

            if change_matches_path(location, filter_path) {
                touched = true;
                Ok::<Action, std::convert::Infallible>(Action::Cancel)
            } else {
                Ok::<Action, std::convert::Infallible>(Action::Continue)
            }
        })
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    Ok(touched)
}
