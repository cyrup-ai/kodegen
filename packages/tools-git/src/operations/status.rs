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
        let is_dirty = repo_clone.is_dirty()
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
        let mut head = repo_clone
            .head()
            .map_err(|e| GitError::Gix(Box::new(e)))?;
        
        let branch_name = head
            .referent_name()
            .and_then(|name| name.shorten().to_str().ok().map(std::string::ToString::to_string))
            .unwrap_or_else(|| "detached HEAD".to_string());
        
        let commit = head
            .peel_to_commit()
            .map_err(|e| GitError::Gix(Box::new(e)))?;
        
        let commit_hash = commit.id().to_string();
        
        // Try to get upstream information
        let (upstream, ahead_count, behind_count) = get_upstream_info(&repo_clone, &head)?;
        
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

/// Get upstream tracking information for a branch
fn get_upstream_info(
    repo: &gix::Repository,
    head: &gix::Head,
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
            Some(format!("{}/{}", remote, merge.trim_start_matches("refs/heads/")))
        } else {
            None
        }
    } else {
        None
    };
    
    // For now, we don't calculate ahead/behind counts
    // This would require walking the commit graph which is complex
    let ahead_count = None;
    let behind_count = None;
    
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
                    .url(gix::remote::Direction::Fetch).map_or_else(|| "unknown".to_string(), std::string::ToString::to_string);
                
                let push_url = remote
                    .url(gix::remote::Direction::Push).map_or_else(|| fetch_url.clone(), std::string::ToString::to_string);
                
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
        Ok(repo_clone.find_remote(remote_name.as_bytes().as_bstr()).is_ok())
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
        let mut head = repo_clone
            .head()
            .map_err(|e| GitError::Gix(Box::new(e)))?;
        
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
        let head = repo_clone
            .head()
            .map_err(|e| GitError::Gix(Box::new(e)))?;
        
        Ok(head.referent_name().is_none())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}
