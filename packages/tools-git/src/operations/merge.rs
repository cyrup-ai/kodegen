//! Git merge operation with comprehensive options.
//!
//! This module provides the `MergeOpts` builder pattern and merge operation
//! implementation for the `GitGix` service.

use crate::{CommitId, GitError, GitResult, RepoHandle};

/// Outcome of a merge operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// Fast-forward with resulting commit ID.
    FastForward(CommitId),
    /// New merge commit created, returning its ID.
    MergeCommit(CommitId),
    /// Already up to date – no changes required.
    AlreadyUpToDate,
}

/// Internal configuration for merge commit creation.
struct MergeCommitConfig {
    squash: bool,
    commit: bool,
    no_ff: bool,
    could_fast_forward: bool,
}

/// Options for `merge` operation with builder pattern.
#[derive(Debug, Clone)]
pub struct MergeOpts {
    pub theirs: String,
    pub no_ff: bool,
    pub squash: bool,
    pub commit: bool,
}

impl MergeOpts {
    /// Create new merge options with the target branch/commit to merge.
    #[inline]
    pub fn new<S: Into<String>>(theirs: S) -> Self {
        Self {
            theirs: theirs.into(),
            no_ff: false,
            squash: false,
            commit: true,
        }
    }

    /// Force a merge commit even if fast-forward is possible.
    #[inline]
    #[must_use]
    pub fn no_ff(mut self, yes: bool) -> Self {
        self.no_ff = yes;
        self
    }

    /// Squash commits from the merged branch into a single commit.
    #[inline]
    #[must_use]
    pub fn squash(mut self, yes: bool) -> Self {
        self.squash = yes;
        self
    }

    /// Whether to create a commit automatically (default: true).
    #[inline]
    #[must_use]
    pub fn commit(mut self, yes: bool) -> Self {
        self.commit = yes;
        self
    }
}

/// Execute merge operation with the given options.
pub async fn merge(repo: RepoHandle, opts: MergeOpts) -> GitResult<MergeOutcome> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let MergeOpts {
            theirs,
            no_ff,
            squash,
            commit,
        } = opts;

        // Resolve the target reference
        use gix::bstr::ByteSlice;
        let parsed = repo_clone
            .rev_parse(theirs.as_bytes().as_bstr())
            .map_err(|e| GitError::InvalidInput(format!("Invalid merge target '{theirs}': {e}")))?;

        let their_commit_id = parsed
            .single()
            .ok_or_else(|| GitError::InvalidInput(format!("Ambiguous merge target: {theirs}")))?;

        // Get current HEAD commit
        let mut head_ref = repo_clone.head().map_err(|e| GitError::Gix(e.into()))?;

        let our_commit_id = head_ref
            .peel_to_commit()
            .map_err(|e| GitError::Gix(e.into()))?
            .id;

        // Check if already up to date
        if our_commit_id == their_commit_id {
            return Ok(MergeOutcome::AlreadyUpToDate);
        }

        let their_commit_id_detached = their_commit_id.detach();

        // Use merge_base to determine commit relationships
        let merge_base = repo_clone
            .merge_base(our_commit_id, their_commit_id_detached)
            .map_err(|e| GitError::Gix(e.into()))?;

        // Case 1: Their commit is the merge base (they're already in our history)
        if merge_base.detach() == their_commit_id_detached {
            return Ok(MergeOutcome::AlreadyUpToDate);
        }

        // Check if we could fast-forward
        let could_fast_forward = merge_base.detach() == our_commit_id;

        // Case 2: Our commit is the merge base (we can fast-forward to them)
        if could_fast_forward && !no_ff {
            fast_forward_merge(&repo_clone, their_commit_id_detached)?;
            return Ok(MergeOutcome::FastForward(their_commit_id_detached));
        }

        // Case 3: Diverged history or forced merge commit - create merge commit
        let config = MergeCommitConfig {
            squash,
            commit,
            no_ff,
            could_fast_forward,
        };
        let merge_commit_id = create_merge_commit(
            &repo_clone,
            our_commit_id,
            their_commit_id_detached,
            &theirs,
            config,
        )?;
        Ok(MergeOutcome::MergeCommit(merge_commit_id))
    })
    .await
    .map_err(|e| GitError::InvalidInput(format!("Task join error: {e}")))?
}

/// Perform a fast-forward merge by updating HEAD, index, and working tree to the target commit.
///
/// This function performs a complete fast-forward merge:
/// 1. Gets the target commit's tree
/// 2. Creates a new index from that tree
/// 3. Checks out the tree to the working directory
/// 4. Writes the updated index to disk
/// 5. Updates HEAD to point to the target commit
///
/// # Errors
///
/// Returns an error if:
/// - Repository is bare (no working tree)
/// - Target commit or tree cannot be found
/// - Checkout encounters errors or collisions
/// - Index cannot be written
/// - HEAD reference update fails
fn fast_forward_merge(repo: &gix::Repository, target_commit: CommitId) -> GitResult<()> {
    // Step 1: Validate repository has a working tree
    let workdir = repo.workdir().ok_or_else(|| {
        GitError::InvalidInput("Cannot fast-forward merge in bare repository".to_string())
    })?;

    // Step 2: Get target commit and its tree
    let commit = repo
        .find_object(target_commit)
        .map_err(|e| GitError::Gix(e.into()))?
        .try_into_commit()
        .map_err(|_| GitError::InvalidInput("Target does not point to a commit".to_string()))?;

    let tree_id = commit.tree_id().map_err(|e| GitError::Gix(e.into()))?;

    // Step 3: Create index from target tree
    let mut index = repo
        .index_from_tree(&tree_id)
        .map_err(|e| GitError::Gix(e.into()))?;

    // Step 4: Get checkout options
    let checkout_opts = repo
        .checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)
        .map_err(|e| GitError::Gix(e.into()))?;

    // Step 5: Checkout tree to working directory
    let outcome = gix::worktree::state::checkout(
        &mut index,
        workdir,
        repo.objects
            .clone()
            .into_arc()
            .map_err(|e| GitError::Gix(e.into()))?,
        &gix::progress::Discard,
        &gix::progress::Discard,
        &std::sync::atomic::AtomicBool::new(false),
        checkout_opts,
    )
    .map_err(|e| GitError::Gix(e.into()))?;

    // Step 6: Handle checkout errors
    if !outcome.errors.is_empty() {
        return Err(GitError::InvalidInput(format!(
            "Fast-forward checkout encountered {} error(s)",
            outcome.errors.len()
        )));
    }

    // Step 7: Handle collisions (should not occur in fast-forward, but check anyway)
    if !outcome.collisions.is_empty() {
        return Err(GitError::InvalidInput(format!(
            "Fast-forward checkout encountered {} collision(s)",
            outcome.collisions.len()
        )));
    }

    // Step 8: Write index to disk
    index
        .write(Default::default())
        .map_err(|e| GitError::Gix(e.into()))?;

    // Step 9: Update HEAD reference
    repo.reference(
        "HEAD",
        target_commit,
        gix::refs::transaction::PreviousValue::Any,
        "merge: Fast-forward",
    )
    .map_err(|e| GitError::Gix(e.into()))?;

    Ok(())
}

/// Create a merge commit combining two parent commits.
///
/// This function performs the actual merge operation, combining the trees from
/// `our_commit` and `their_commit` into a merged tree. Depending on the options,
/// it either creates a new merge commit immediately or prepares the merge state
/// for manual completion.
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `our_commit` - The current HEAD commit (ours)
/// * `their_commit` - The commit being merged in (theirs)
/// * `their_name` - Name/ref of the branch/commit being merged (for commit message)
/// * `squash` - If true, create a single-parent commit instead of a merge commit
/// * `commit` - If false, prepare the merge without creating a commit
/// * `no_ff` - If true, this was a forced merge commit (--no-ff flag)
/// * `could_fast_forward` - If true, a fast-forward was possible but skipped
///
/// # Returns
///
/// Returns the commit ID of the new merge commit, or the current HEAD commit ID
/// if `commit` is false (merge prepared but not committed).
///
/// # Errors
///
/// Returns an error if:
/// - Merge conflicts are detected
/// - Repository is bare (when `commit` is false)
/// - Any git operations fail (tree merge, checkout, commit creation)
fn create_merge_commit(
    repo: &gix::Repository,
    our_commit: CommitId,
    their_commit: CommitId,
    their_name: &str,
    config: MergeCommitConfig,
) -> GitResult<CommitId> {
    // Step 1: Get merge options from repository config
    let tree_merge_opts = repo
        .tree_merge_options()
        .map_err(|e| GitError::Gix(e.into()))?;

    let commit_merge_opts: gix::merge::commit::Options = tree_merge_opts.into();

    // Step 2: Set up labels for conflict markers
    use gix::merge::blob::builtin_driver::text::Labels;
    let labels = Labels {
        ancestor: None,
        current: Some("HEAD".into()),
        other: Some(their_name.into()),
    };

    // Step 3: Perform the merge using gix
    let mut merge_outcome = repo
        .merge_commits(our_commit, their_commit, labels, commit_merge_opts)
        .map_err(|e| GitError::Gix(e.into()))?;

    // Step 4: Check for unresolved conflicts
    use gix::merge::tree::TreatAsUnresolved;
    if merge_outcome
        .tree_merge
        .has_unresolved_conflicts(TreatAsUnresolved::default())
    {
        return Err(GitError::MergeConflict(
            "Merge has conflicts that must be resolved manually".to_string(),
        ));
    }

    // Step 5: Write the merged tree to ODB
    let merged_tree_id = merge_outcome
        .tree_merge
        .tree
        .write()
        .map_err(|e| GitError::Gix(e.into()))?;

    // Step 6: Handle commit option
    if !config.commit {
        // Prepare merge without committing
        // This updates the index and worktree but doesn't create a commit

        // 6a. Create index from merged tree using gix's public API
        let mut index = repo
            .index_from_tree(&merged_tree_id)
            .map_err(|e| GitError::Gix(e.into()))?;

        // 6b. Get checkout options
        let opts = repo
            .checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)
            .map_err(|e| GitError::Gix(e.into()))?;

        // 6c. Checkout merged tree to worktree
        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::InvalidInput("Cannot merge in bare repository".to_string()))?;

        let outcome = gix::worktree::state::checkout(
            &mut index,
            workdir,
            repo.objects
                .clone()
                .into_arc()
                .map_err(|e| GitError::Gix(e.into()))?,
            &gix::progress::Discard,
            &gix::progress::Discard,
            &std::sync::atomic::AtomicBool::new(false),
            opts,
        )
        .map_err(|e| GitError::Gix(e.into()))?;

        // 6d. Handle checkout errors
        if !outcome.errors.is_empty() {
            return Err(GitError::InvalidInput(format!(
                "Merge checkout encountered {} error(s)",
                outcome.errors.len()
            )));
        }

        // 6e. Handle checkout collisions
        if !outcome.collisions.is_empty() {
            return Err(GitError::InvalidInput(format!(
                "Merge checkout encountered {} collision(s)",
                outcome.collisions.len()
            )));
        }

        // 6f. Write MERGE_HEAD to mark merge in progress
        let merge_head_path = repo.path().join("MERGE_HEAD");
        std::fs::write(&merge_head_path, format!("{their_commit}\n"))
            .map_err(|e| GitError::Gix(e.into()))?;

        // 6g. Write MERGE_MSG with the merge message
        let merge_msg_path = repo.path().join("MERGE_MSG");
        let message = format!("Merge '{their_name}'\n");
        std::fs::write(&merge_msg_path, message).map_err(|e| GitError::Gix(e.into()))?;

        // 6h. Write MERGE_MODE if this is a forced merge commit (--no-ff on fast-forwardable)
        if config.no_ff && config.could_fast_forward {
            let merge_mode_path = repo.path().join("MERGE_MODE");
            std::fs::write(&merge_mode_path, "no-ff\n").map_err(|e| GitError::Gix(e.into()))?;
        }

        // 6i. Write index to disk
        index
            .write(Default::default())
            .map_err(|e| GitError::Gix(e.into()))?;

        // Return our_commit since HEAD doesn't move (merge prepared but not committed)
        return Ok(our_commit);
    }

    // Step 7: Create commit with appropriate parents
    let message = format!("Merge '{their_name}'");
    let parents = if config.squash {
        vec![our_commit]
    } else {
        vec![our_commit, their_commit]
    };

    let merge_commit_id = repo
        .commit("HEAD", &message, merged_tree_id, parents)
        .map_err(|e| GitError::Gix(e.into()))?;

    Ok(merge_commit_id.detach())
}
