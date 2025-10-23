//! Git client for local repository operations
//!
//! Provides a cohesive client API for git operations without exposing
//! internal repository details.
//!
//! # Examples
//!
//! ```rust,no_run
//! use gitgix::{GitClient, CommitOpts};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Open existing repository
//!     let client = GitClient::open(".").await?;
//!
//!     // Perform operations
//!     let commit_id = client.commit(
//!         CommitOpts::message("Initial commit")
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

use std::path::Path;

use crate::runtime::{AsyncStream, AsyncTask};
use crate::{CommitId, CommitInfo, GitResult, MergeOutcome, RepoHandle};

use super::{AddOpts, BranchOpts, CheckoutOpts, CloneOpts, CommitOpts, FetchOpts, LogOpts, MergeOpts};

/// Git client for local repository operations.
///
/// Provides clean API for git operations. Cloning is cheap (RepoHandle is cheap to clone).
#[derive(Clone, Debug)]
pub struct GitClient {
    repo: RepoHandle,
}

impl GitClient {
    // ========================================================================
    // Constructors
    // ========================================================================

    /// Open an existing repository at the given path
    pub fn open(path: impl AsRef<Path>) -> AsyncTask<GitResult<Self>> {
        let repo_task = crate::git::open::open_repo(path);
        AsyncTask::spawn_async(async move {
            match repo_task.await {
                Ok(repo) => Ok(Self { repo }),
                Err(e) => Err(e),
            }
        })
    }

    /// Discover a repository by searching upward from the given path
    pub fn discover(path: impl AsRef<Path>) -> AsyncTask<GitResult<Self>> {
        let repo_task = crate::git::open::discover_repo(path);
        AsyncTask::spawn_async(async move {
            match repo_task.await {
                Ok(repo) => Ok(Self { repo }),
                Err(e) => Err(e),
            }
        })
    }

    /// Initialize a new repository at the given path
    pub fn init(path: impl AsRef<Path>) -> AsyncTask<GitResult<Self>> {
        let repo_task = crate::git::open::init_repo(path);
        AsyncTask::spawn_async(async move {
            match repo_task.await {
                Ok(repo) => Ok(Self { repo }),
                Err(e) => Err(e),
            }
        })
    }

    /// Initialize a bare repository at the given path
    pub fn init_bare(path: impl AsRef<Path>) -> AsyncTask<GitResult<Self>> {
        let repo_task = crate::git::open::init_bare_repo(path);
        AsyncTask::spawn_async(async move {
            match repo_task.await {
                Ok(repo) => Ok(Self { repo }),
                Err(e) => Err(e),
            }
        })
    }

    /// Clone a repository with the given options
    pub fn clone_repo(opts: CloneOpts) -> AsyncTask<GitResult<Self>> {
        let repo_task = crate::git::clone::clone_repo(opts);
        AsyncTask::spawn_async(async move {
            match repo_task.await {
                Ok(repo) => Ok(Self { repo }),
                Err(e) => Err(e),
            }
        })
    }

    // ========================================================================
    // Operations
    // ========================================================================

    /// Stage files for commit
    pub fn add(&self, opts: AddOpts) -> AsyncTask<GitResult<()>> {
        crate::git::add::add(self.repo.clone(), opts)
    }

    /// Create a commit
    pub fn commit(&self, opts: CommitOpts) -> AsyncTask<GitResult<CommitId>> {
        crate::git::commit::commit(self.repo.clone(), opts)
    }

    /// Fetch from a remote
    pub fn fetch(&self, opts: FetchOpts) -> AsyncTask<GitResult<()>> {
        crate::git::fetch::fetch(self.repo.clone(), opts)
    }

    /// Perform branch operations
    pub fn branch(&self, opts: BranchOpts) -> AsyncTask<GitResult<()>> {
        crate::git::branch::branch(self.repo.clone(), opts)
    }

    /// Checkout a branch or commit
    pub fn checkout(&self, opts: CheckoutOpts) -> AsyncTask<GitResult<()>> {
        crate::git::checkout::checkout(self.repo.clone(), opts)
    }

    /// Merge branches
    pub fn merge(&self, opts: MergeOpts) -> AsyncTask<GitResult<MergeOutcome>> {
        crate::git::merge::merge(self.repo.clone(), opts)
    }

    /// Stream commit history
    pub fn log(&self, opts: LogOpts) -> AsyncStream<GitResult<CommitInfo>> {
        crate::git::log::log(self.repo.clone(), opts)
    }
}
