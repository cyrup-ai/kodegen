//! Git add operation with comprehensive options.
//!
//! This module provides the `AddOpts` builder pattern and add operation
//! implementation for the `GitGix` service. Supports:
//! - Individual files and directories (recursive)
//! - Simple glob patterns (*, ?)
//! - .gitignore respect (force flag to override)
//! - Symlink handling per core.symlinks config
//! - Update-only mode for tracked files

use std::path::{Path, PathBuf};

use gix::bstr::ByteSlice;
use walkdir::WalkDir;

use crate::{GitError, GitResult, RepoHandle};

/// Options for `add` operation with builder pattern.
#[derive(Debug, Clone)]
pub struct AddOpts {
    pub paths: Vec<PathBuf>,
    pub update_only: bool,
    pub force: bool,
}

impl AddOpts {
    /// Create new add options with the given paths.
    #[inline]
    pub fn new<I, P>(paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        Self {
            paths: paths.into_iter().map(Into::into).collect(),
            update_only: false,
            force: false,
        }
    }

    /// Add a single path to be staged.
    #[inline]
    pub fn add_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.paths.push(path.into());
        self
    }

    /// Enable update-only mode (only add already tracked files).
    #[inline]
    #[must_use]
    pub fn update_only(mut self, yes: bool) -> Self {
        self.update_only = yes;
        self
    }

    /// Force add files even if they're in .gitignore.
    #[inline]
    #[must_use]
    pub fn force(mut self, yes: bool) -> Self {
        self.force = yes;
        self
    }
}

/// Check if a path string contains glob pattern characters.
/// Single-pass check with zero allocations.
#[inline]
fn has_glob_pattern(path: &Path) -> bool {
    path.as_os_str()
        .as_encoded_bytes()
        .iter()
        .any(|&b| b == b'*' || b == b'?')
}

/// Simple glob pattern matching for * and ? wildcards.
/// Works with byte slices for zero-allocation matching of both UTF-8 and non-UTF8 paths.
#[inline]
fn simple_glob_match(pattern: &[u8], text: &[u8]) -> bool {
    simple_glob_match_impl(pattern, text, 0, 0)
}

/// Internal implementation using indices to avoid allocations.
#[inline]
fn simple_glob_match_impl(
    pattern: &[u8],
    text: &[u8],
    mut pat_idx: usize,
    mut text_idx: usize,
) -> bool {
    while pat_idx < pattern.len() || text_idx < text.len() {
        if pat_idx < pattern.len() {
            match pattern[pat_idx] {
                b'*' => {
                    pat_idx += 1;
                    // Handle trailing *
                    if pat_idx >= pattern.len() {
                        return true;
                    }
                    // Try matching rest of pattern at each text position
                    while text_idx <= text.len() {
                        if simple_glob_match_impl(pattern, text, pat_idx, text_idx) {
                            return true;
                        }
                        text_idx += 1;
                    }
                    return false;
                }
                b'?' => {
                    if text_idx >= text.len() {
                        return false;
                    }
                    pat_idx += 1;
                    text_idx += 1;
                }
                byte => {
                    if text_idx >= text.len() || text[text_idx] != byte {
                        return false;
                    }
                    pat_idx += 1;
                    text_idx += 1;
                }
            }
        } else {
            // Pattern exhausted but text remains
            return false;
        }
    }
    // Both exhausted - match
    pat_idx >= pattern.len() && text_idx >= text.len()
}

/// Expand input paths to concrete file paths.
/// Handles directories (recursive), glob patterns, and individual files.
///
/// # Contract
/// All returned paths are absolute. Callers can rely on this guarantee.
#[inline]
fn expand_paths(paths: &[PathBuf], repo_path: &Path) -> GitResult<Vec<PathBuf>> {
    let mut result = Vec::with_capacity(paths.len() * 4);

    for input_path in paths {
        let full_path = if input_path.is_absolute() {
            input_path.clone()
        } else {
            repo_path.join(input_path)
        };

        if has_glob_pattern(input_path) {
            // Glob pattern: walk parent directory and match
            let parent = full_path.parent().ok_or_else(|| {
                GitError::InvalidInput(format!("Invalid glob pattern: {}", input_path.display()))
            })?;

            let pattern_bytes = full_path
                .file_name()
                .ok_or_else(|| {
                    GitError::InvalidInput(format!("Invalid pattern: {}", input_path.display()))
                })?
                .as_encoded_bytes();

            if !parent.exists() {
                return Err(GitError::InvalidInput(format!(
                    "Pattern parent directory does not exist: {}",
                    parent.display()
                )));
            }

            for entry in WalkDir::new(parent)
                .max_depth(1)
                .min_depth(1)
                .into_iter()
                .filter_entry(|e| e.file_type().is_file())
            {
                let entry = entry.map_err(|e| GitError::Io(e.into()))?;
                let filename_bytes = entry.file_name().as_encoded_bytes();
                if simple_glob_match(pattern_bytes, filename_bytes) {
                    result.push(entry.path().to_path_buf());
                }
            }
        } else if full_path.is_dir() {
            // Directory: recursively collect all files (skip .git directory)
            for entry in WalkDir::new(&full_path).into_iter().filter_entry(|e| {
                // Skip .git directory to avoid walking internal git files
                if e.file_type().is_dir() {
                    e.file_name() != ".git"
                } else {
                    true
                }
            }) {
                let entry = entry.map_err(|e| GitError::Io(e.into()))?;
                if entry.file_type().is_file() {
                    result.push(entry.path().to_path_buf());
                }
            }
        } else {
            // Regular file or symlink
            result.push(full_path);
        }
    }

    Ok(result)
}

/// Process a single file: handle symlinks, read content, create blob, add to index.
#[inline]
fn process_single_file(
    repo: &gix::Repository,
    index: &mut gix::index::File,
    file_path: &Path,
    relative_path: &Path,
    symlinks_enabled: bool,
) -> GitResult<()> {
    use gix::index::entry::{Flags, Mode, Stat};

    // Get file metadata once (lstat - doesn't follow symlinks)
    let fs_metadata = gix::index::fs::Metadata::from_path_no_follow(file_path)?;

    let (blob_data, mode) = if fs_metadata.is_symlink() {
        if symlinks_enabled {
            // Store symlink target
            let target = std::fs::read_link(file_path)?;
            let target_bytes = target.as_os_str().as_encoded_bytes().to_vec();
            (target_bytes, Mode::SYMLINK)
        } else {
            // Follow symlink and store content from target file
            let content = std::fs::read(file_path)?;
            // Check executable bit from target file's metadata (Unix only)
            #[cfg(unix)]
            let is_executable = {
                use std::os::unix::fs::PermissionsExt;
                let target_metadata = std::fs::metadata(file_path)?;
                target_metadata.permissions().mode() & 0o111 != 0
            };
            #[cfg(not(unix))]
            let is_executable = false; // Windows doesn't have Unix executable bits

            let mode = if is_executable {
                Mode::FILE_EXECUTABLE
            } else {
                Mode::FILE
            };
            (content, mode)
        }
    } else {
        // Regular file - reuse metadata for executable check
        let content = std::fs::read(file_path)?;
        let mode = if fs_metadata.is_executable() {
            Mode::FILE_EXECUTABLE
        } else {
            Mode::FILE
        };
        (content, mode)
    };

    // Write blob to ODB
    let blob_id = repo
        .write_blob(&blob_data)
        .map_err(|e| GitError::Gix(e.into()))?
        .detach();

    // Reuse metadata for stat (already have it from line 211)
    let stat = Stat::from_fs(&fs_metadata).map_err(|e| {
        GitError::InvalidInput(format!(
            "Failed to create stat for {}: {}",
            file_path.display(),
            e
        ))
    })?;

    // Convert path to BStr for gix API
    let path_bstr = relative_path.as_os_str().as_encoded_bytes().as_bstr();

    // Add to index
    index.dangerously_push_entry(stat, blob_id, Flags::empty(), mode, path_bstr);

    Ok(())
}

/// Execute add operation with the given options.
pub async fn add(repo: RepoHandle, opts: AddOpts) -> GitResult<()> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let AddOpts {
            paths,
            update_only,
            force,
        } = opts;

        if paths.is_empty() {
            return Err(GitError::InvalidInput(
                "No paths specified for add".to_string(),
            ));
        }

        // Get repo workdir
        let repo_path = repo_clone.workdir().ok_or_else(|| {
            GitError::InvalidInput("Cannot add files in bare repository".to_string())
        })?;

        // Check core.symlinks config
        let config = repo_clone.config_snapshot();
        let symlinks_enabled = config.boolean("core.symlinks").unwrap_or(true);

        // Expand input paths to concrete file paths
        let expanded_paths = expand_paths(&paths, repo_path)?;

        if expanded_paths.is_empty() {
            return Err(GitError::InvalidInput(
                "No files matched the given patterns".to_string(),
            ));
        }

        // Open mutable index (create empty one if it doesn't exist)
        let mut index = if let Ok(idx) = repo_clone.open_index() {
            idx
        } else {
            // Index doesn't exist yet (freshly initialized repo)
            // Create an empty index
            let index_path = repo_clone.index_path();
            let object_hash = repo_clone.object_hash();
            let mut new_index =
                gix::index::File::from_state(gix::index::State::new(object_hash), index_path);
            // Write the empty index to disk
            new_index
                .write(gix::index::write::Options::default())
                .map_err(|e| GitError::Gix(e.into()))?;
            // Re-open it
            repo_clone
                .open_index()
                .map_err(|e| GitError::Gix(e.into()))?
        };

        // Setup .gitignore checking if not forcing
        let mut excludes = if force {
            None
        } else {
            Some(
                repo_clone.excludes(
                    &index,
                    None,
                    gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
                )
                .map_err(|e| GitError::Gix(e.into()))?,
            )
        };

        // Process each file (all paths from expand_paths are absolute)
        for file_path in expanded_paths {
            // Convert to relative path (file_path is guaranteed absolute by expand_paths)
            let relative_path = file_path
                .strip_prefix(repo_path)
                .map_err(|_| {
                    GitError::InvalidInput(format!(
                        "Path {} is not within repository",
                        file_path.display()
                    ))
                })?
                .to_path_buf();

            // Convert path to BStr for gix operations
            let path_bstr = relative_path.as_os_str().as_encoded_bytes().as_bstr();

            // Check update_only mode
            if update_only && index.entry_by_path(path_bstr).is_none() {
                // Skip files not already in index
                continue;
            }

            // Check .gitignore (if not forcing) - reuse path_bstr
            if let Some(ref mut exc) = excludes {
                let platform = exc.at_entry(path_bstr, None)?;

                if platform.is_excluded() {
                    // Skip ignored files unless force=true
                    continue;
                }
            }

            // Process the file (file_path is already absolute per expand_paths contract)
            process_single_file(
                &repo_clone,
                &mut index,
                &file_path,
                &relative_path,
                symlinks_enabled,
            )?;
        }

        // CRITICAL: Sort entries to maintain invariants
        index.sort_entries();

        // Write index to disk with proper locking and checksum
        use gix::index::write::Options;
        index
            .write(Options::default())
            .map_err(|e| GitError::Gix(e.into()))?;

        Ok(())
    })
    .await
    .map_err(|e| GitError::InvalidInput(format!("Task join error: {e}")))?
}
