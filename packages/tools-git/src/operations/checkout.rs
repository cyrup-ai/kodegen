//! Git checkout operation with comprehensive options.
//!
//! This module provides the `CheckoutOpts` builder pattern and checkout operation
//! implementation for the `GitGix` service.

use std::sync::atomic::AtomicBool;

use gix::bstr::ByteSlice;
use gix::refs::Target;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};

use crate::{GitError, GitResult, RepoHandle};

/// Options for `checkout` operation with builder pattern.
#[derive(Debug, Clone)]
pub struct CheckoutOpts {
    pub reference: String,
    pub force: bool,
    pub paths: Option<Vec<std::path::PathBuf>>,
}

impl CheckoutOpts {
    /// Create new checkout options for the given reference.
    #[inline]
    pub fn new<S: Into<String>>(reference: S) -> Self {
        Self {
            reference: reference.into(),
            force: false,
            paths: None,
        }
    }

    /// Enable force checkout (overwrite local changes).
    #[inline]
    #[must_use]
    pub fn force(mut self, yes: bool) -> Self {
        self.force = yes;
        self
    }

    /// Set specific file paths to checkout (file restoration mode).
    #[inline]
    pub fn paths<I, P>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<std::path::PathBuf>,
    {
        self.paths = Some(paths.into_iter().map(Into::into).collect());
        self
    }
}

/// Checkout specific files from a reference (file restoration mode).
///
/// This restores specific file paths from the given reference without changing HEAD.
/// Steps:
/// 1. Resolve reference to commit
/// 2. Get tree from commit
/// 3. For each path: lookup in tree, read blob, write to disk, update index
fn checkout_files(
    repo: &gix::Repository,
    reference: &str,
    paths: Vec<std::path::PathBuf>,
    force: bool,
) -> GitResult<()> {
    // Step 1: Resolve reference to commit
    let parsed = repo
        .rev_parse(reference.as_bytes().as_bstr())
        .map_err(|e| {
            GitError::InvalidInput(format!("Failed to resolve reference '{reference}': {e}"))
        })?;

    let object_id = parsed.single().ok_or_else(|| {
        GitError::InvalidInput(format!(
            "Reference '{reference}' is ambiguous (matches multiple objects)"
        ))
    })?;

    // Step 2: Get commit and tree
    let commit = repo
        .find_object(object_id)
        .map_err(|e| {
            GitError::InvalidInput(format!("Failed to find object for '{reference}': {e}"))
        })?
        .try_into_commit()
        .map_err(|_| {
            GitError::InvalidInput(format!(
                "Reference '{reference}' does not point to a commit"
            ))
        })?;

    let tree = commit
        .tree()
        .map_err(|e| GitError::Gix(format!("Failed to get tree from commit: {e}").into()))?;

    // Step 3: Get worktree path
    let worktree = repo.worktree().ok_or_else(|| {
        GitError::InvalidInput(
            "Cannot checkout files in bare repository (no working directory)".to_string(),
        )
    })?;
    let worktree_path = worktree.base();

    // Step 4: Open index for updates
    let mut index = repo.open_index().map_err(|e| GitError::Gix(e.into()))?;

    // Step 5: Process each file path
    for path in paths {
        // Lookup entry in tree
        let entry = tree
            .lookup_entry_by_path(&path)
            .map_err(|e| GitError::Gix(format!("Failed to lookup path in tree: {e}").into()))?
            .ok_or_else(|| {
                GitError::InvalidInput(format!("Path not found in tree: {}", path.display()))
            })?;

        // Only handle files, not directories
        if entry.mode().is_tree() {
            return Err(GitError::InvalidInput(format!(
                "Cannot checkout directory '{}', only files are supported",
                path.display()
            )));
        }

        // Get blob object
        let object = entry
            .object()
            .map_err(|e| GitError::Gix(format!("Failed to read object: {e}").into()))?;

        let blob_data = &object.data;

        // Check if file exists and would be overwritten
        let full_path = worktree_path.join(&path);
        if full_path.exists() && !force {
            // Check if file has local modifications
            let current_content = std::fs::read(&full_path).map_err(|e| {
                GitError::Io(std::io::Error::new(
                    e.kind(),
                    format!(
                        "Cannot read existing file '{}' to check for modifications: {}. Use force=true to overwrite anyway.",
                        path.display(),
                        e
                    ),
                ))
            })?;

            if current_content != *blob_data {
                return Err(GitError::Gix(
                    format!(
                        "File '{}' has local changes. Use force=true to overwrite.",
                        path.display()
                    )
                    .into(),
                ));
            }
        }

        // Write blob to working directory
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, blob_data)?;

        // Update index entry
        use gix::index::entry::{Flags, Mode, Stat};

        let gix_metadata = gix::index::fs::Metadata::from_path_no_follow(&full_path)?;

        let mode = if gix_metadata.is_executable() {
            Mode::FILE_EXECUTABLE
        } else {
            Mode::FILE
        };

        let stat = Stat::from_fs(&gix_metadata).map_err(|e| {
            GitError::InvalidInput(format!(
                "Failed to create stat for {}: {}",
                path.display(),
                e
            ))
        })?;

        // Convert path to gix format
        let path_bytes = gix::path::os_str_into_bstr(path.as_os_str()).map_err(|_| {
            GitError::InvalidInput(format!("Invalid UTF-8 in path: {}", path.display()))
        })?;

        // Add/update the index entry
        index.dangerously_push_entry(
            stat,
            entry.oid().to_owned(),
            Flags::empty(),
            mode,
            path_bytes.as_ref(),
        );
    }

    // Step 6: Sort entries to maintain index invariants
    index.sort_entries();

    // Step 7: Write index to disk with proper locking and checksum
    use gix::index::write::Options;
    index
        .write(Options::default())
        .map_err(|e| GitError::Gix(format!("Failed to write index: {e}").into()))?;

    Ok(())
}

/// Execute checkout operation with the given options.
///
/// This performs a complete Git checkout operation:
/// 1. Resolves the reference to a commit
/// 2. Extracts the tree from the commit
/// 3. Updates the working directory files
/// 4. Updates the index
/// 5. Updates HEAD (symbolic for branches, direct for commits/tags)
///
/// # Symbolic vs Detached HEAD
///
/// - Local branches (e.g., "main", "refs/heads/feature") → Symbolic HEAD
/// - Remote branches (e.g., "origin/main") → Detached HEAD
/// - Tags (e.g., "v1.0", "refs/tags/v1.0") → Detached HEAD
/// - Commit SHAs (e.g., "abc123") → Detached HEAD
pub async fn checkout(repo: RepoHandle, opts: CheckoutOpts) -> GitResult<()> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let CheckoutOpts { reference, force, paths } = opts;

        // Branch on operation type: file checkout vs full checkout
        if let Some(file_paths) = paths {
            return checkout_files(&repo_clone, &reference, file_paths, force);
        }

        // Step 1: Resolve reference to object ID (full checkout path)
        let parsed = repo_clone
            .rev_parse(reference.as_bytes().as_bstr())
            .map_err(|e| {
                GitError::InvalidInput(format!(
                    "Failed to resolve reference '{reference}': {e}"
                ))
            })?;

        let object_id = parsed.single().ok_or_else(|| {
            GitError::InvalidInput(format!(
                "Reference '{reference}' is ambiguous (matches multiple objects)"
            ))
        })?;

        // Step 2: Get commit object
        let commit = repo_clone
            .find_object(object_id)
            .map_err(|e| {
                GitError::InvalidInput(format!("Failed to find object for '{reference}': {e}"))
            })?
            .try_into_commit()
            .map_err(|_| {
                GitError::InvalidInput(format!(
                    "Reference '{reference}' (oid: {object_id}) does not point to a commit"
                ))
            })?;

        // Step 3: Extract tree ID from commit
        let tree_id = commit.tree_id().map_err(|e| {
            GitError::InvalidInput(format!(
                "Failed to get tree from commit {object_id}: {e}"
            ))
        })?;

        // Step 3.5: Get current index to track which files to remove
        let old_index = repo_clone.open_index().ok();

        // Step 4: Create index from tree
        let mut index = repo_clone.index_from_tree(&tree_id).map_err(|e| {
            GitError::Gix(format!("Failed to create index from tree {tree_id}: {e}").into())
        })?;

        // Step 5: Get worktree path (fail if bare repository)
        let worktree = repo_clone.worktree().ok_or_else(|| {
            GitError::InvalidInput(
                "Cannot checkout in bare repository (no working directory)".to_string(),
            )
        })?;
        let worktree_path = worktree.base().to_owned();

        // Step 6: Configure checkout options
        let mut checkout_opts = repo_clone
            .checkout_options(
                gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
            )
            .map_err(|e| {
                GitError::Gix(format!("Failed to create checkout options: {e}").into())
            })?;
        checkout_opts.overwrite_existing = force;
        checkout_opts.destination_is_initially_empty = false;

        // Step 7: Perform the actual file checkout
        let outcome = gix::worktree::state::checkout(
            &mut index,
            worktree_path.clone(),
            repo_clone.objects.clone().into_arc().map_err(|e| {
                GitError::Gix(format!("Failed to access object database: {e}").into())
            })?,
            &gix::progress::Discard,
            &gix::progress::Discard,
            &AtomicBool::new(false),
            checkout_opts,
        )
        .map_err(|e| GitError::Gix(format!("Checkout operation failed: {e}").into()))?;

        // Step 8: Handle errors and collisions
        if !outcome.errors.is_empty() {
            // Collect error details for helpful error message  
            let error_details: Vec<String> = outcome
                .errors
                .iter()
                .take(10) // Show first 10 to avoid overwhelming output
                .map(|err| {
                    let path_str = std::str::from_utf8(err.path.as_ref()).map_or_else(|_| format!("{:?}", err.path), std::string::ToString::to_string);
                    format!("{}: {}", path_str, err.error)
                })
                .collect();

            let error_summary = if outcome.errors.len() > 10 {
                format!(
                    "Checkout failed with {} error(s). First 10:\n{}\nWorking directory may be in a partial state.",
                    outcome.errors.len(),
                    error_details.join("\n")
                )
            } else {
                format!(
                    "Checkout failed with {} error(s):\n{}\nWorking directory may be in a partial state.",
                    outcome.errors.len(),
                    error_details.join("\n")
                )
            };

            return Err(GitError::Gix(error_summary.into()));
        }

        if !outcome.collisions.is_empty() && !force {
            // Collect collision details for helpful error message
            let collision_paths: Vec<String> = outcome
                .collisions
                .iter()
                .take(10) // Show first 10 to avoid overwhelming output
                .map(|entry| {
                    std::str::from_utf8(entry.path.as_ref()).map_or_else(|_| format!("{:?}", entry.path), std::string::ToString::to_string)
                })
                .collect();

            let collision_summary = if outcome.collisions.len() > 10 {
                format!("{} (showing first 10)", collision_paths.join(", "))
            } else {
                collision_paths.join(", ")
            };

            return Err(GitError::Gix(
                format!(
                    "Checkout blocked: {} file collision(s) detected. Files: {}. Use force=true to overwrite.",
                    outcome.collisions.len(),
                    collision_summary
                )
                .into(),
            ));
        }

        // Step 8.5: Remove files that existed in old index but not in new index
        // This ensures files from previous branch are cleaned up
        if let Some(old_idx) = old_index {
            use gix::bstr::ByteSlice;
            use std::collections::HashSet;

            // Build set of paths in new index
            let new_paths: HashSet<Vec<u8>> = index
                .entries()
                .iter()
                .map(|e| e.path(&index).to_vec())
                .collect();

            // Find paths in old index but not in new index
            for old_entry in old_idx.entries() {
                let old_path_bytes = old_entry.path(&old_idx);

                if !new_paths.contains(old_path_bytes) {
                    // This file should be removed
                    if let Ok(path_str) = std::str::from_utf8(old_path_bytes.as_bytes()) {
                        let file_path = worktree_path.join(path_str);

                        // Only remove if it exists and is a file (not directory)
                        if file_path.exists() && file_path.is_file() {
                            // Ignore errors during cleanup - file may already be gone
                            let _ = std::fs::remove_file(&file_path);
                        }
                    }
                }
            }
        }

        // Step 9: Write updated index to disk with proper locking and checksum
        use gix::index::write::Options;
        index.write(Options::default()).map_err(|e| {
            GitError::Gix(
                format!("Failed to write index: {e}").into(),
            )
        })?;

        // Step 10: Update HEAD (symbolic for local branches, direct otherwise)
        // Determine if this is a local branch that should use symbolic HEAD
        let is_local_branch = if reference.starts_with("refs/heads/") {
            // Already fully qualified - check if it exists
            repo_clone.try_find_reference(reference.as_bytes().as_bstr())
                .map_err(|e| GitError::Gix(format!("Failed to check reference: {e}").into()))?
                .is_some()
        } else if !reference.starts_with("refs/") && !reference.contains('/') {
            // Short name without path separator - might be local branch
            // Check refs/heads/{reference}
            let full_ref_name = format!("refs/heads/{reference}");
            repo_clone.try_find_reference(full_ref_name.as_bytes().as_bstr())
                .map_err(|e| GitError::Gix(format!("Failed to check reference: {e}").into()))?
                .is_some()
        } else {
            // Other patterns (origin/main, tags/v1.0, commit SHA) → detached HEAD
            false
        };

        if is_local_branch {
            // Symbolic HEAD update: HEAD → refs/heads/branch
            let full_ref_name = if reference.starts_with("refs/heads/") {
                reference.clone()
            } else {
                format!("refs/heads/{reference}")
            };

            // Create symbolic reference using edit_reference
            let sym_target: gix::refs::FullName =
                full_ref_name.as_str().try_into().map_err(|e| {
                    GitError::InvalidInput(format!(
                        "Invalid reference name '{full_ref_name}': {e}"
                    ))
                })?;

            repo_clone.edit_reference(RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: format!("checkout: moving from HEAD to {reference}").into(),
                    },
                    expected: PreviousValue::Any,
                    new: Target::Symbolic(sym_target),
                },
                name: "HEAD".try_into().map_err(|e| {
                    GitError::InvalidInput(format!("Invalid HEAD reference: {e}"))
                })?,
                deref: false,
            })
            .map_err(|e| {
                GitError::Gix(
                    format!("Failed to update HEAD to branch '{full_ref_name}': {e}").into(),
                )
            })?;
        } else {
            // Direct HEAD update: HEAD → commit (detached HEAD)
            repo_clone.reference(
                "HEAD",
                object_id,
                PreviousValue::Any,
                format!("checkout: moving from HEAD to {reference}"),
            )
            .map_err(|e| {
                GitError::Gix(
                    format!("Failed to update HEAD to commit {object_id}: {e}").into(),
                )
            })?;
        }

        Ok(())
    })
    .await
    .map_err(|e| GitError::InvalidInput(format!("Task join error: {e}")))?
}
