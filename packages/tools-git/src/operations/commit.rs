//! Git commit operation with comprehensive options.
//!
//! This module provides the `CommitOpts` builder pattern and commit operation
//! implementation for the `GitGix` service.

use chrono::{DateTime, Utc};

use crate::{CommitId, GitError, GitResult, RepoHandle};

/// Git signature (author/committer) information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub name: String,
    pub email: String,
    pub time: DateTime<Utc>,
}

impl Signature {
    /// Create a new signature with current time.
    pub fn new<N: Into<String>, E: Into<String>>(name: N, email: E) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            time: Utc::now(),
        }
    }

    /// Create a signature with a specific time.
    pub fn with_time<N: Into<String>, E: Into<String>>(
        name: N,
        email: E,
        time: DateTime<Utc>,
    ) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            time,
        }
    }
}

impl From<gix::actor::Signature> for Signature {
    fn from(sig: gix::actor::Signature) -> Self {
        use chrono::TimeZone;
        // Convert gix::date::Time to DateTime<Utc>
        let timestamp = sig.time.seconds;
        let time = Utc
            .timestamp_opt(timestamp, 0)
            .single()
            .unwrap_or_else(Utc::now);

        Self {
            name: sig.name.to_string(),
            email: sig.email.to_string(),
            time,
        }
    }
}

/// Options for `commit` operation with builder pattern.
#[derive(Debug, Clone)]
pub struct CommitOpts {
    pub message: String,
    pub amend: bool,
    pub all: bool,
    pub author: Option<Signature>,
    pub committer: Option<Signature>,
}

impl CommitOpts {
    /// Create new commit options with the given message.
    #[inline]
    pub fn message<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            amend: false,
            all: false,
            author: None,
            committer: None,
        }
    }

    /// Enable amend mode (modify the last commit).
    #[must_use]
    pub fn amend(mut self, yes: bool) -> Self {
        self.amend = yes;
        self
    }

    /// Enable all mode (commit all tracked files automatically).
    #[must_use]
    pub fn all(mut self, yes: bool) -> Self {
        self.all = yes;
        self
    }

    /// Set the author signature.
    #[must_use]
    pub fn author(mut self, sig: Signature) -> Self {
        self.author = Some(sig);
        self
    }

    /// Set the committer signature.
    #[must_use]
    pub fn committer(mut self, sig: Signature) -> Self {
        self.committer = Some(sig);
        self
    }
}

/// Execute commit operation with the given options.
pub async fn commit(repo: RepoHandle, opts: CommitOpts) -> GitResult<CommitId> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let CommitOpts {
            message,
            amend,
            all,
            author,
            committer,
        } = opts;

        if message.trim().is_empty() {
            return Err(GitError::InvalidInput(
                "Commit message cannot be empty".to_string(),
            ));
        }

        // Get current index
        let index = repo_clone
            .open_index()
            .map_err(|e| GitError::Gix(e.into()))?;

        // Handle --all option: stage all modified tracked files
        let index = if all {
            // Get working directory (fail for bare repos)
            let workdir = repo_clone.workdir().ok_or_else(|| {
                GitError::InvalidInput("Cannot use --all in bare repository".to_string())
            })?;

            // Shadow index as mutable
            let mut index = index;
            let mut changed = false;

            // Collect entries to process to avoid borrow issues
            let entries_to_process: Vec<_> = (0..index.entries().len())
                .map(|idx| {
                    let entry = &index.entries()[idx];
                    let entry_path = entry.path(&index).to_owned();
                    let entry_id = entry.id;
                    (entry_path, entry_id)
                })
                .collect();

            // Process each tracked file
            for (entry_path, old_id) in entries_to_process {
                // Build full path
                use gix::bstr::ByteSlice;
                use std::path::Path;
                let path_str = std::str::from_utf8(entry_path.as_bytes())
                    .map_err(|_| GitError::InvalidInput("Invalid UTF-8 in path".to_string()))?;
                let full_path = workdir.join(Path::new(path_str));

                // Skip non-existent or non-file entries
                if !full_path.exists() || !full_path.is_file() {
                    continue;
                }

                // Read file contents
                let contents = std::fs::read(&full_path)?;

                // Write blob and get ID
                let blob_id = repo_clone
                    .write_blob(&contents)
                    .map_err(|e| GitError::Gix(e.into()))?
                    .detach();

                // Only update if content changed
                if blob_id != old_id {
                    // Get file metadata
                    let metadata = gix::index::fs::Metadata::from_path_no_follow(&full_path)?;

                    // Determine mode
                    use gix::index::entry::Mode;
                    let mode = if metadata.is_executable() {
                        Mode::FILE_EXECUTABLE
                    } else {
                        Mode::FILE
                    };

                    // Create stat
                    use gix::index::entry::Stat;
                    let stat = Stat::from_fs(&metadata).map_err(|e| {
                        GitError::InvalidInput(format!("Failed to create stat: {e}"))
                    })?;

                    // Add to index
                    use gix::index::entry::Flags;
                    index.dangerously_push_entry(
                        stat,
                        blob_id,
                        Flags::empty(),
                        mode,
                        entry_path.as_ref(),
                    );
                    changed = true;
                }
            }

            // If we modified the index, write it to disk
            if changed {
                // CRITICAL: Sort entries to maintain index invariants
                index.sort_entries();

                // Write index with proper locking and checksum
                use gix::index::write::Options;
                index
                    .write(Options::default())
                    .map_err(|e| GitError::Gix(e.into()))?;
            }

            // Re-open index for tree building
            repo_clone
                .open_index()
                .map_err(|e| GitError::Gix(e.into()))?
        } else {
            index
        };

        // Create tree editor to build hierarchical tree structure
        let mut editor = gix::objs::tree::Editor::new(
            gix::objs::Tree::empty(),
            &repo_clone.objects,
            repo_clone.object_hash(),
        );

        // Add each index entry with path components to build hierarchy
        for entry in index.entries() {
            if let Some(tree_mode) = entry.mode.to_tree_entry_mode() {
                let path = entry.path(&index);
                // Split path into components for hierarchical tree building
                let components: Vec<&gix::bstr::BStr> = path
                    .split(|&b| b == b'/')
                    .map(std::convert::AsRef::as_ref)
                    .collect();

                // Convert tree::EntryMode to EntryKind
                let kind = tree_mode.kind();

                editor
                    .upsert(components, kind, entry.id)
                    .map_err(|e| GitError::Gix(Box::new(e)))?;
            }
        }

        // Write all tree objects and get root tree ID
        let tree_id = editor
            .write(|tree| {
                repo_clone
                    .write_object(tree)
                    .map(gix::Id::detach)
                    .map_err(|e| GitError::Gix(Box::new(e)))
            })
            .map_err(|e| match e {
                GitError::Gix(inner) => GitError::Gix(inner),
                _ => GitError::Gix(Box::new(e)),
            })?;

        // Get current HEAD commit ID
        let head_commit_id = repo_clone.head_id().ok();

        // Get or create author signature
        let author_sig = if let Some(author) = author {
            gix::actor::Signature {
                name: author.name.as_str().into(),
                email: author.email.as_str().into(),
                time: gix::date::Time::new(author.time.timestamp(), 0),
            }
        } else {
            // Use config default - convert SignatureRef to owned Signature
            let sig_ref = repo_clone
                .author()
                .ok_or_else(|| GitError::InvalidInput("No author configured".to_string()))?
                .map_err(|e| GitError::Gix(Box::new(e)))?;
            sig_ref.to_owned().map_err(|e| GitError::Gix(Box::new(e)))?
        };

        // Get or create committer signature
        let committer_sig = if let Some(committer) = committer {
            gix::actor::Signature {
                name: committer.name.as_str().into(),
                email: committer.email.as_str().into(),
                time: gix::date::Time::new(committer.time.timestamp(), 0),
            }
        } else {
            // Use config default or author
            match repo_clone.committer() {
                Some(Ok(sig_ref)) => sig_ref.to_owned().map_err(|e| GitError::Gix(Box::new(e)))?,
                Some(Err(e)) => return Err(GitError::Gix(Box::new(e))),
                None => author_sig.clone(),
            }
        };

        // Determine parents based on amend flag
        let parents = if amend {
            // For amend: use HEAD's parents (replace HEAD)
            if let Some(head_id) = head_commit_id {
                // Get HEAD commit object using pattern from checkout.rs
                let head_commit = repo_clone
                    .find_object(head_id)
                    .map_err(|e| GitError::Gix(e.into()))?
                    .try_into_commit()
                    .map_err(|_| {
                        GitError::InvalidInput("HEAD does not point to a commit".to_string())
                    })?;

                // Get HEAD's parent IDs
                head_commit
                    .parent_ids()
                    .map(gix::Id::detach)
                    .collect::<Vec<_>>()
            } else {
                // No HEAD exists - cannot amend
                return Err(GitError::InvalidInput(
                    "Cannot amend: no commits exist yet".to_string(),
                ));
            }
        } else {
            // Normal commit: use HEAD as parent
            head_commit_id
                .into_iter()
                .map(gix::Id::detach)
                .collect::<Vec<_>>()
        };

        // Create time buffers for signature conversion
        use gix::date::parse::TimeBuf;
        let mut committer_time_buf = TimeBuf::default();
        let mut author_time_buf = TimeBuf::default();

        let commit_id = repo_clone
            .commit_as(
                committer_sig.to_ref(&mut committer_time_buf),
                author_sig.to_ref(&mut author_time_buf),
                "HEAD",
                &message,
                tree_id,
                parents,
            )
            .map_err(|e| GitError::Gix(e.into()))?;

        Ok(commit_id.detach())
    })
    .await
    .map_err(|e| GitError::InvalidInput(format!("Task join error: {e}")))?
}
