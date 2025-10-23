//! Git reset operations
//!
//! Provides functionality for resetting repository state to a specific commit.

use crate::{GitError, GitResult, RepoHandle};
use gix::bstr::ByteSlice;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Reset mode determines what gets reset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    /// Soft reset - move HEAD but keep index and working directory
    Soft,
    /// Mixed reset - move HEAD and reset index, keep working directory
    Mixed,
    /// Hard reset - move HEAD, reset index, and reset working directory
    Hard,
}

/// Options for reset operation
#[derive(Debug, Clone)]
pub struct ResetOpts {
    /// Target commit (hash, ref, or symbolic name like "HEAD~1")
    pub target: String,
    /// Reset mode
    pub mode: ResetMode,
    /// Optional cancellation token for graceful abort
    /// When set to true, operation will abort and return `GitError::Aborted`
    pub cancel_token: Option<Arc<AtomicBool>>,
}

/// Validate preconditions for reset operation
fn validate_reset_preconditions(
    repo: &gix::Repository,
    opts: &ResetOpts,
    _target_commit: &gix::Commit,
) -> GitResult<()> {
    // Check 1: For hard reset, ensure repository has worktree
    if opts.mode == ResetMode::Hard {
        repo.worktree().ok_or_else(|| {
            GitError::InvalidInput("Cannot perform hard reset on bare repository".to_string())
        })?;
    }

    // Check 2: Verify index is writable
    let index_path = repo.index_path();
    if index_path.exists() {
        // Try to open for write (doesn't actually write)
        std::fs::OpenOptions::new()
            .write(true)
            .open(&index_path)
            .map_err(|e| GitError::InvalidInput(format!("Index file is not writable: {e}")))?;
    }

    Ok(())
}

/// Reset repository to a specific commit
///
/// Resets the repository state based on the specified mode:
/// - Soft: Only moves HEAD
/// - Mixed: Moves HEAD and resets index
/// - Hard: Moves HEAD, resets index, and resets working directory
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `opts` - Reset options
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, reset, ResetOpts, ResetMode};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// reset(&repo, ResetOpts {
///     target: "HEAD~1".to_string(),
///     mode: ResetMode::Mixed,
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn reset(repo: &RepoHandle, opts: ResetOpts) -> GitResult<()> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        // Helper to check cancellation
        let check_cancelled = || -> GitResult<()> {
            if let Some(ref token) = opts.cancel_token
                && token.load(Ordering::Relaxed)
            {
                return Err(GitError::Aborted);
            }
            Ok(())
        };

        // Resolve target commit
        let target_id = repo_clone
            .rev_parse_single(opts.target.as_bytes().as_bstr())
            .map_err(|e| GitError::Parse(format!("Invalid target '{}': {}", opts.target, e)))?;

        // Get target commit
        let target_commit = repo_clone
            .find_object(target_id)
            .map_err(|e| GitError::Gix(Box::new(e)))?
            .try_into_commit()
            .map_err(|_| GitError::Parse("Target is not a commit".to_string()))?;

        // Phase 1: Validation (fail fast before any changes)
        validate_reset_preconditions(&repo_clone, &opts, &target_commit)?;

        // Check cancellation before starting
        check_cancelled()?;

        // Phase 2: Save original state for error messages
        let original_head = repo_clone
            .head()
            .ok()
            .and_then(|mut h| h.try_peel_to_id().ok().flatten())
            .map(|id| id.to_string());

        // Phase 3: Execute in SAFE order (risky → safe)

        // Step 1: Reset working directory FIRST (most likely to fail)
        if opts.mode == ResetMode::Hard {
            check_cancelled()?;

            reset_working_directory(
                &repo_clone,
                &target_commit,
                None,
                opts.cancel_token.as_ref(),
            )
            .map_err(|e| {
                GitError::InvalidInput(format!(
                    "Reset failed: Could not update working directory: {}. \
                         Repository is unchanged. HEAD is still at {}.",
                    e,
                    original_head.as_deref().unwrap_or("unknown")
                ))
            })?;
        }

        // Step 2: Reset index SECOND (can fail, but worktree already consistent)
        if opts.mode == ResetMode::Mixed || opts.mode == ResetMode::Hard {
            check_cancelled()?;

            reset_index(&repo_clone, &target_commit).map_err(|e| {
                let state_msg = if opts.mode == ResetMode::Hard {
                    "Working directory was updated but index write failed. \
                         Repository is in an inconsistent state. \
                         Run 'git status' to see current state."
                } else {
                    "Index write failed. Repository is unchanged."
                };

                GitError::InvalidInput(format!(
                    "Reset failed: Could not update index: {e}. {state_msg}"
                ))
            })?;
        }

        // Step 3: Move HEAD LAST (least likely to fail)
        check_cancelled()?;

        reset_head(&repo_clone, target_id.into(), &opts.target).map_err(|e| {
            let state_msg = match opts.mode {
                ResetMode::Soft => "Repository is unchanged.",
                ResetMode::Mixed => {
                    "Index was updated but HEAD was not moved. \
                         Run 'git status' - you should see all changes as staged."
                }
                ResetMode::Hard => {
                    "Working directory and index were updated but HEAD was not moved. \
                         Run 'git status' - you should see all changes as staged. \
                         This is a valid state but not what reset intended."
                }
            };

            GitError::InvalidInput(format!(
                "Reset failed: Could not update HEAD: {e}. {state_msg}"
            ))
        })?;

        Ok(())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Reset HEAD to a specific commit
fn reset_head(
    repo: &gix::Repository,
    target_id: gix::hash::ObjectId,
    target_ref: &str,
) -> GitResult<()> {
    let head = repo.head().map_err(|e| GitError::Gix(Box::new(e)))?;

    // Check if HEAD is symbolic (on a branch) to determine deref behavior
    // For symbolic refs (HEAD -> refs/heads/main), we update the branch reference
    // For direct refs (detached HEAD), we update HEAD directly
    let is_symbolic = matches!(
        head.kind,
        gix::head::Kind::Symbolic(_) | gix::head::Kind::Unborn(_)
    );

    if is_symbolic {
        // Symbolic HEAD: Update the branch reference that HEAD points to
        // This uses edit_reference which handles reflog creation automatically
        use gix::bstr::ByteSlice;
        let head_name = head.name().as_bstr();
        let ref_name = gix::refs::FullName::try_from(head_name.as_bstr())
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        use gix::refs::Target;
        use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};

        repo.edit_reference(RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: false,
                    message: format!("reset: moving to {target_ref}").into(),
                },
                expected: PreviousValue::Any,
                new: Target::Object(target_id),
            },
            name: ref_name,
            deref: true,
        })
        .map_err(|e| GitError::Gix(Box::new(e)))?;
    } else {
        // Detached HEAD: Update HEAD directly
        // Use high-level reference API which handles reflog automatically
        use gix::refs::transaction::PreviousValue;

        repo.reference(
            "HEAD",
            target_id,
            PreviousValue::Any,
            format!("reset: moving to {target_ref}"),
        )
        .map_err(|e| GitError::Gix(Box::new(e)))?;
    }

    Ok(())
}

/// Reset index to match a specific commit
fn reset_index(repo: &gix::Repository, target_commit: &gix::Commit) -> GitResult<()> {
    // Step 1: Get tree ID from target commit
    let tree_id = target_commit
        .tree_id()
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    // Step 2: Create new index from target tree
    // This uses gix_index::State::from_tree internally
    // See: packages/git/tmp/gitoxide/gix-index/src/init.rs:48-64
    let mut new_index = repo
        .index_from_tree(&tree_id)
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    // Step 3: Write new index to disk with proper locking and checksum
    // Note: index.write() handles flushing, syncing, and mtime updates automatically
    use gix::index::write::Options;
    new_index
        .write(Options::default())
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    Ok(())
}

/// Reset working directory to match a specific commit
fn reset_working_directory(
    repo: &gix::Repository,
    target_commit: &gix::Commit,
    progress: Option<&dyn gix::progress::Progress>,
    cancel_token: Option<&Arc<AtomicBool>>,
) -> GitResult<()> {
    use std::sync::atomic::AtomicBool;

    // Step 1: Get tree ID from target commit
    let tree_id = target_commit
        .tree_id()
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    // Step 2: Create index from target tree
    let mut index = repo
        .index_from_tree(&tree_id)
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    // Step 3: Get worktree path (fail if bare repository)
    let worktree = repo.worktree().ok_or_else(|| {
        GitError::InvalidInput("Cannot reset working directory in bare repository".to_string())
    })?;
    let worktree_path = worktree.base().to_owned();

    // Step 4: Configure checkout options for force overwrite
    let mut checkout_opts = repo
        .checkout_options(gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping)
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    // Force overwrite all files (this is --hard reset behavior)
    checkout_opts.overwrite_existing = true;
    checkout_opts.destination_is_initially_empty = false;

    // Step 5: Perform the actual file checkout
    // Accept progress parameter or use Discard
    let progress_ref: &dyn gix::progress::Progress = match progress {
        Some(p) => p,
        None => &gix::progress::Discard,
    };

    // Use caller's token or create a default false one
    let default_token = AtomicBool::new(false);
    let cancel_ref: &AtomicBool = match cancel_token {
        Some(token) => token.as_ref(),
        None => &default_token,
    };

    let outcome = gix::worktree::state::checkout(
        &mut index,
        &worktree_path,
        repo.objects
            .clone()
            .into_arc()
            .map_err(|e| GitError::Gix(Box::new(e)))?,
        progress_ref,
        progress_ref,
        cancel_ref,
        checkout_opts,
    )
    .map_err(|e| GitError::Gix(Box::new(e)))?;

    // Check if cancelled after checkout
    if let Some(token) = cancel_token
        && token.load(Ordering::Relaxed)
    {
        return Err(GitError::Aborted);
    }

    // Step 6: Check for errors
    if !outcome.errors.is_empty() {
        // Collect error details for helpful error message
        let error_details: Vec<String> = outcome
            .errors
            .iter()
            .take(10) // Show first 10 to avoid overwhelming output
            .map(|err| {
                let path_str = std::str::from_utf8(err.path.as_ref()).map_or_else(
                    |_| format!("{:?}", err.path),
                    std::string::ToString::to_string,
                );
                format!("{}: {}", path_str, err.error)
            })
            .collect();

        let error_summary = if outcome.errors.len() > 10 {
            format!(
                "Reset failed with {} error(s). First 10:\n{}",
                outcome.errors.len(),
                error_details.join("\n")
            )
        } else {
            format!(
                "Reset failed with {} error(s):\n{}",
                outcome.errors.len(),
                error_details.join("\n")
            )
        };

        return Err(GitError::Gix(error_summary.into()));
    }

    Ok(())
}

/// Soft reset to a commit (only moves HEAD)
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, reset_soft};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// reset_soft(&repo, "HEAD~1").await?;
/// # Ok(())
/// # }
/// ```
pub async fn reset_soft(repo: &RepoHandle, target: &str) -> GitResult<()> {
    reset(
        repo,
        ResetOpts {
            target: target.to_string(),
            mode: ResetMode::Soft,
            cancel_token: None,
        },
    )
    .await
}

/// Mixed reset to a commit (moves HEAD and resets index)
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, reset_mixed};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// reset_mixed(&repo, "HEAD~1").await?;
/// # Ok(())
/// # }
/// ```
pub async fn reset_mixed(repo: &RepoHandle, target: &str) -> GitResult<()> {
    reset(
        repo,
        ResetOpts {
            target: target.to_string(),
            mode: ResetMode::Mixed,
            cancel_token: None,
        },
    )
    .await
}

/// Hard reset to a commit (moves HEAD, resets index and working directory)
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, reset_hard};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// reset_hard(&repo, "HEAD~1").await?;
/// # Ok(())
/// # }
/// ```
pub async fn reset_hard(repo: &RepoHandle, target: &str) -> GitResult<()> {
    reset(
        repo,
        ResetOpts {
            target: target.to_string(),
            mode: ResetMode::Hard,
            cancel_token: None,
        },
    )
    .await
}
