//! Git repository status operations
//!
//! Provides functionality for checking repository state, branch information, and remote details.

use crate::{GitError, GitResult, RepoHandle};
use gix::bstr::ByteSlice;

/// Information about a Git branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this is the current branch
    pub is_current: bool,
    /// Current commit hash
    pub commit_hash: String,
    /// Tracking remote branch (if any)
    pub upstream: Option<String>,
    /// Number of commits ahead of upstream
    pub ahead_count: Option<usize>,
    /// Number of commits behind upstream
    pub behind_count: Option<usize>,
}

/// Information about a Git remote
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    /// Remote name
    pub name: String,
    /// Fetch URL
    pub fetch_url: String,
    /// Push URL (may be different from fetch)
    pub push_url: String,
}

/// Check if the working directory is clean
///
/// Returns `true` if there are no uncommitted changes, `false` otherwise.
///
/// # Arguments
///
/// * `repo` - Repository handle
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, is_clean};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// if is_clean(&repo).await? {
///     println!("Working directory is clean");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn is_clean(repo: &RepoHandle) -> GitResult<bool> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        // Use is_dirty() which is the proper API for checking if repo has changes
        let is_dirty = repo_clone
            .is_dirty()
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        Ok(!is_dirty)
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Get information about the current branch
///
/// # Arguments
///
/// * `repo` - Repository handle
///
/// # Returns
///
/// Returns `BranchInfo` containing details about the current branch.
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, current_branch};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// let branch = current_branch(&repo).await?;
/// println!("Current branch: {}", branch.name);
/// # Ok(())
/// # }
/// ```
pub async fn current_branch(repo: &RepoHandle) -> GitResult<BranchInfo> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let mut head = repo_clone.head().map_err(|e| GitError::Gix(Box::new(e)))?;

        let branch_name = head
            .referent_name()
            .and_then(|name| {
                name.shorten()
                    .to_str()
                    .ok()
                    .map(std::string::ToString::to_string)
            })
            .unwrap_or_else(|| "detached HEAD".to_string());

        let commit = head
            .peel_to_commit()
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        let commit_hash = commit.id().to_string();

        // Try to get upstream information
        let (upstream, ahead_count, behind_count) = get_upstream_info(&repo_clone, &mut head)?;

        Ok(BranchInfo {
            name: branch_name,
            is_current: true,
            commit_hash,
            upstream,
            ahead_count,
            behind_count,
        })
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Calculate ahead/behind commit counts between local and upstream branches
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `local_commit_id` - Object ID of the local branch HEAD
/// * `upstream_ref` - Upstream reference string (e.g., "origin/main")
///
/// # Returns
///
/// Returns a tuple of (ahead_count, behind_count) where:
/// - Both are `None` if upstream doesn't exist
/// - Both are `Some(0)` if branches point to the same commit
/// - Otherwise, contains actual commit counts
fn calculate_ahead_behind(
    repo: &gix::Repository,
    local_commit_id: gix::ObjectId,
    upstream_ref: &str,
) -> GitResult<(Option<usize>, Option<usize>)> {
    use gix::bstr::ByteSlice;

    // Convert upstream ref string to full reference path
    // e.g., "origin/main" -> "refs/remotes/origin/main"
    let upstream_ref_path = if upstream_ref.starts_with("refs/") {
        upstream_ref.to_string()
    } else {
        format!("refs/remotes/{upstream_ref}")
    };

    // Try to find the upstream reference
    let mut upstream_reference =
        match repo.try_find_reference(upstream_ref_path.as_bytes().as_bstr()) {
            Ok(Some(r)) => r,
            Ok(None) => return Ok((None, None)), // Upstream doesn't exist
            Err(e) => return Err(GitError::Gix(Box::new(e))),
        };

    // Get the upstream commit ID
    let upstream_commit_id = match upstream_reference.peel_to_id() {
        Ok(id) => id.detach(),
        Err(e) => return Err(GitError::Gix(Box::new(e))),
    };

    // If both commits are the same, return (0, 0)
    if local_commit_id == upstream_commit_id {
        return Ok((Some(0), Some(0)));
    }

    // Find merge base (common ancestor)
    // Create a graph for merge base calculation
    let mut graph = repo.revision_graph(None);

    let merge_base_id =
        match repo.merge_base_with_graph(local_commit_id, upstream_commit_id, &mut graph) {
            Ok(base_id) => base_id.detach(),
            Err(e) => {
                // If no merge base found (completely diverged histories),
                // we can't calculate ahead/behind in a meaningful way
                return Err(GitError::Gix(Box::new(e)));
            }
        };

    // Count commits ahead (from merge_base to local_commit_id)
    let ahead_count = count_commits_between(repo, merge_base_id, local_commit_id)?;

    // Count commits behind (from merge_base to upstream_commit_id)
    let behind_count = count_commits_between(repo, merge_base_id, upstream_commit_id)?;

    Ok((Some(ahead_count), Some(behind_count)))
}

/// Count commits between two points in the commit graph
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `from` - Starting commit (exclusive)
/// * `to` - Ending commit (inclusive)
///
/// # Returns
///
/// Returns the number of commits between `from` and `to`
fn count_commits_between(
    repo: &gix::Repository,
    from: gix::ObjectId,
    to: gix::ObjectId,
) -> GitResult<usize> {
    // If from and to are the same, there are 0 commits between them
    if from == to {
        return Ok(0);
    }

    // Collect all commits reachable from 'from' (the merge base)
    let mut from_commits = std::collections::HashSet::new();

    let from_walker = repo
        .rev_walk([from])
        .all()
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    for commit_result in from_walker {
        match commit_result {
            Ok(info) => {
                from_commits.insert(info.id);
            }
            Err(e) => return Err(GitError::Gix(Box::new(e))),
        }
    }

    // Count commits reachable from 'to' that are NOT in from_commits
    let mut count = 0;

    let to_walker = repo
        .rev_walk([to])
        .all()
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    for commit_result in to_walker {
        match commit_result {
            Ok(info) => {
                if !from_commits.contains(&info.id) {
                    count += 1;
                }
            }
            Err(e) => return Err(GitError::Gix(Box::new(e))),
        }
    }

    Ok(count)
}

/// Get upstream tracking information for a branch
fn get_upstream_info(
    repo: &gix::Repository,
    head: &mut gix::Head,
) -> GitResult<(Option<String>, Option<usize>, Option<usize>)> {
    // Try to get upstream branch
    let upstream = if let Some(branch_ref) = head.referent_name() {
        let branch_name = branch_ref.shorten();

        // Look for branch.{name}.remote and branch.{name}.merge in config
        let config = repo.config_snapshot();
        let branch_section = format!("branch.{branch_name}");

        let remote_name = config
            .string(format!("{branch_section}.remote"))
            .map(|s| s.to_string());

        let merge_ref = config
            .string(format!("{branch_section}.merge"))
            .map(|s| s.to_string());

        if let (Some(remote), Some(merge)) = (remote_name, merge_ref) {
            Some(format!(
                "{}/{}",
                remote,
                merge.trim_start_matches("refs/heads/")
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Calculate ahead/behind counts if upstream exists
    let (ahead_count, behind_count) = if let Some(ref upstream_ref) = upstream {
        // Get the local commit ID from HEAD
        let local_commit_id = match head.peel_to_commit() {
            Ok(commit) => commit.id().detach(),
            Err(_) => {
                // If we can't get the commit (e.g., detached HEAD with invalid ref),
                // return None for counts
                return Ok((upstream, None, None));
            }
        };

        // Calculate ahead/behind counts
        // If calculation fails (e.g., no merge base), return None for counts
        // but still return the upstream ref name
        calculate_ahead_behind(repo, local_commit_id, upstream_ref).unwrap_or_default()
    } else {
        // No upstream configured
        (None, None)
    };

    Ok((upstream, ahead_count, behind_count))
}

/// List all remotes in the repository
///
/// # Arguments
///
/// * `repo` - Repository handle
///
/// # Returns
///
/// Returns a vector of `RemoteInfo` for all configured remotes.
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, list_remotes};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// let remotes = list_remotes(&repo).await?;
/// for remote in remotes {
///     println!("Remote: {} -> {}", remote.name, remote.fetch_url);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn list_remotes(repo: &RepoHandle) -> GitResult<Vec<RemoteInfo>> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let mut remotes = Vec::new();

        for remote_name in repo_clone.remote_names() {
            if let Ok(remote) = repo_clone.find_remote(remote_name.as_ref()) {
                let fetch_url = remote
                    .url(gix::remote::Direction::Fetch)
                    .map_or_else(|| "unknown".to_string(), std::string::ToString::to_string);

                let push_url = remote
                    .url(gix::remote::Direction::Push)
                    .map_or_else(|| fetch_url.clone(), std::string::ToString::to_string);

                remotes.push(RemoteInfo {
                    name: remote_name.to_string(),
                    fetch_url,
                    push_url,
                });
            }
        }

        Ok(remotes)
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Check if a remote exists
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote_name` - Name of the remote to check
///
/// # Returns
///
/// Returns `true` if the remote exists, `false` otherwise.
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, remote_exists};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// if remote_exists(&repo, "origin").await? {
///     println!("Origin remote exists");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn remote_exists(repo: &RepoHandle, remote_name: &str) -> GitResult<bool> {
    let repo_clone = repo.clone_inner();
    let remote_name = remote_name.to_string();

    tokio::task::spawn_blocking(move || {
        use gix::bstr::ByteSlice;
        Ok(repo_clone
            .find_remote(remote_name.as_bytes().as_bstr())
            .is_ok())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Get the current HEAD commit hash
///
/// # Arguments
///
/// * `repo` - Repository handle
///
/// # Returns
///
/// Returns the commit hash as a string.
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, head_commit};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// let commit_hash = head_commit(&repo).await?;
/// println!("HEAD: {}", commit_hash);
/// # Ok(())
/// # }
/// ```
pub async fn head_commit(repo: &RepoHandle) -> GitResult<String> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let mut head = repo_clone.head().map_err(|e| GitError::Gix(Box::new(e)))?;

        let commit = head
            .peel_to_commit()
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        Ok(commit.id().to_string())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Check if repository is in a detached HEAD state
///
/// # Arguments
///
/// * `repo` - Repository handle
///
/// # Returns
///
/// Returns `true` if HEAD is detached, `false` if on a branch.
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, is_detached};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// if is_detached(&repo).await? {
///     println!("Warning: Detached HEAD state");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn is_detached(repo: &RepoHandle) -> GitResult<bool> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let head = repo_clone.head().map_err(|e| GitError::Gix(Box::new(e)))?;

        Ok(head.referent_name().is_none())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}
