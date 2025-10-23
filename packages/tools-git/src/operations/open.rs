//! Git repository opening and discovery operations.
//!
//! This module provides functions for opening existing repositories and
//! discovering repositories from subdirectories for the `GitGix` service.

use std::path::{Path, PathBuf};

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

/// Open an existing repository at the given path.
pub fn open_repo<P: AsRef<Path>>(path: P) -> AsyncTask<GitResult<RepoHandle>> {
    let path = path.as_ref().to_path_buf();

    AsyncTask::spawn(move || {
        // Check if path exists first
        if !path.exists() {
            return Err(GitError::InvalidInput(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        let repo = gix::open(&path).map_err(|e| {
            GitError::InvalidInput(format!(
                "Failed to open Git repository at {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(RepoHandle::new(repo))
    })
}

/// Discover a repository by searching upward from the given path.
///
/// This function will search from the given path upward through parent
/// directories until it finds a Git repository or reaches the filesystem root.
pub fn discover_repo<P: AsRef<Path>>(path: P) -> AsyncTask<GitResult<RepoHandle>> {
    let path = path.as_ref().to_path_buf();

    AsyncTask::spawn(move || {
        let repo = gix::discover(&path).map_err(|e| {
            GitError::InvalidInput(format!(
                "No Git repository found at {} or any parent directory: {}",
                path.display(),
                e
            ))
        })?;

        Ok(RepoHandle::new(repo))
    })
}

/// Initialize a new repository at the given path.
pub fn init_repo<P: AsRef<Path>>(path: P) -> AsyncTask<GitResult<RepoHandle>> {
    let path = path.as_ref().to_path_buf();

    AsyncTask::spawn(move || {
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(GitError::Io)?;
        }

        // Check if already a repository
        if gix::open(&path).is_ok() {
            return Err(GitError::InvalidInput(format!(
                "Path is already a Git repository: {}",
                path.display()
            )));
        }

        let repo = gix::init(&path).map_err(GitError::from)?;

        Ok(RepoHandle::new(repo))
    })
}

/// Initialize a bare repository at the given path.
pub fn init_bare_repo<P: AsRef<Path>>(path: P) -> AsyncTask<GitResult<RepoHandle>> {
    let path = path.as_ref().to_path_buf();

    AsyncTask::spawn(move || {
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(GitError::Io)?;
        }

        // Check if already a repository
        if gix::open(&path).is_ok() {
            return Err(GitError::InvalidInput(format!(
                "Path is already a Git repository: {}",
                path.display()
            )));
        }

        let repo = gix::init_bare(&path).map_err(GitError::from)?;

        Ok(RepoHandle::new(repo))
    })
}

/// Check if a path contains a valid Git repository.
pub fn is_repository<P: AsRef<Path>>(path: P) -> AsyncTask<bool> {
    let path = path.as_ref().to_path_buf();

    AsyncTask::spawn(move || gix::open(&path).is_ok())
}

/// Get repository information without opening the full repository.
pub fn probe_repository<P: AsRef<Path>>(path: P) -> AsyncTask<GitResult<RepositoryInfo>> {
    let path = path.as_ref().to_path_buf();

    AsyncTask::spawn(move || {
        let repo = gix::open(&path).map_err(|e| {
            GitError::InvalidInput(format!(
                "Failed to probe Git repository at {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(RepositoryInfo {
            path,
            is_bare: repo.is_bare(),
            git_dir: repo.git_dir().to_path_buf(),
            work_dir: repo.workdir().map(std::path::Path::to_path_buf),
        })
    })
}

/// Basic repository information that can be obtained without full initialization.
#[derive(Debug, Clone)]
pub struct RepositoryInfo {
    /// The path where the repository was found.
    pub path: PathBuf,
    /// Whether this is a bare repository.
    pub is_bare: bool,
    /// Path to the .git directory.
    pub git_dir: PathBuf,
    /// Path to the working directory (None for bare repositories).
    pub work_dir: Option<PathBuf>,
}
