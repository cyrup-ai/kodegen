//! Git tag operations
//!
//! Provides functionality for creating, deleting, and listing Git tags.

use crate::{GitError, GitResult, RepoHandle};
use chrono::{DateTime, Utc};
use gix::bstr::ByteSlice;

/// Options for creating a tag
#[derive(Debug, Clone)]
pub struct TagOpts {
    /// Tag name
    pub name: String,
    /// Optional message for annotated tags
    pub message: Option<String>,
    /// Target commit (defaults to HEAD if None)
    pub target: Option<String>,
    /// Force creation (overwrite if exists)
    pub force: bool,
}

/// Information about a Git tag
#[derive(Debug, Clone)]
pub struct TagInfo {
    /// Tag name
    pub name: String,
    /// Tag message (if annotated)
    pub message: Option<String>,
    /// Target commit hash
    pub target_commit: String,
    /// Tag timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether this is an annotated tag
    pub is_annotated: bool,
}

/// Create a Git tag
///
/// Creates either a lightweight or annotated tag depending on whether a message is provided.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `opts` - Tag creation options
///
/// # Returns
///
/// Returns `TagInfo` on success containing information about the created tag.
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, create_tag, TagOpts};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// let tag_info = create_tag(&repo, TagOpts {
///     name: "v1.0.0".to_string(),
///     message: Some("Release v1.0.0".to_string()),
///     target: None,
///     force: false,
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_tag(repo: &RepoHandle, opts: TagOpts) -> GitResult<TagInfo> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let tag_ref_name = format!("refs/tags/{}", opts.name);

        // Resolve target commit
        let target = if let Some(ref target_str) = opts.target {
            repo_clone
                .rev_parse_single(target_str.as_bytes().as_bstr())
                .map_err(|e| GitError::Parse(format!("Invalid target '{target_str}': {e}")))?
                .into()
        } else {
            // Default to HEAD
            let mut head = repo_clone.head().map_err(|e| GitError::Gix(Box::new(e)))?;
            head.try_peel_to_id()
                .map_err(|e| GitError::Gix(Box::new(e)))?
                .ok_or_else(|| {
                    GitError::InvalidInput("HEAD does not point to a commit".to_string())
                })?
                .detach()
        };

        // Check if tag exists
        if !opts.force
            && repo_clone
                .refs
                .find(tag_ref_name.as_bytes().as_bstr())
                .is_ok()
        {
            return Err(GitError::InvalidInput(format!(
                "Tag '{}' already exists",
                opts.name
            )));
        }

        // Create tag reference
        let is_annotated = opts.message.is_some();

        // For lightweight tags, use transaction
        if is_annotated {
            // For annotated tags, create tag object
            let message = opts.message.as_deref().unwrap_or("");
            let signature = get_signature(&repo_clone)?;

            use gix::bstr::ByteSlice;
            let time_str = signature.time.to_string();
            let sig_ref = gix::actor::SignatureRef {
                name: signature.name.as_bstr(),
                email: signature.email.as_bstr(),
                time: &time_str,
            };

            repo_clone
                .tag(
                    &opts.name,
                    target,
                    gix::objs::Kind::Commit,
                    Some(sig_ref),
                    message,
                    if opts.force {
                        gix::refs::transaction::PreviousValue::Any
                    } else {
                        gix::refs::transaction::PreviousValue::MustNotExist
                    },
                )
                .map_err(|e| GitError::Gix(Box::new(e)))?;
        } else {
            let ref_name = gix::refs::FullName::try_from(tag_ref_name.as_bytes().as_bstr())
                .map_err(|e| GitError::Gix(Box::new(e)))?;

            let edit = gix::refs::transaction::RefEdit {
                change: gix::refs::transaction::Change::Update {
                    log: gix::refs::transaction::LogChange::default(),
                    expected: if opts.force {
                        gix::refs::transaction::PreviousValue::Any
                    } else {
                        gix::refs::transaction::PreviousValue::MustNotExist
                    },
                    new: gix::refs::Target::Object(target),
                },
                name: ref_name,
                deref: false,
            };

            repo_clone
                .refs
                .transaction()
                .prepare(
                    vec![edit],
                    gix::lock::acquire::Fail::Immediately,
                    gix::lock::acquire::Fail::Immediately,
                )
                .map_err(|e| GitError::Gix(Box::new(e)))?
                .commit(None)
                .map_err(|e| GitError::Gix(Box::new(e)))?;
        }

        // Get tag info
        let commit = repo_clone
            .find_object(target)
            .map_err(|e| GitError::Gix(Box::new(e)))?
            .try_into_commit()
            .map_err(|_| GitError::Parse("Target is not a commit".to_string()))?;

        let commit_time = commit.time().map_err(|e| GitError::Gix(Box::new(e)))?;
        let timestamp = DateTime::from_timestamp(commit_time.seconds, 0).unwrap_or_else(Utc::now);

        Ok(TagInfo {
            name: opts.name,
            message: opts.message,
            target_commit: target.to_string(),
            timestamp,
            is_annotated,
        })
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Delete a Git tag
///
/// Deletes a local tag. Remote tag deletion is not supported by this function.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `tag_name` - Name of the tag to delete
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, delete_tag};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// delete_tag(&repo, "v1.0.0").await?;
/// # Ok(())
/// # }
/// ```
pub async fn delete_tag(repo: &RepoHandle, tag_name: &str) -> GitResult<()> {
    let repo_clone = repo.clone_inner();
    let tag_name = tag_name.to_string();

    tokio::task::spawn_blocking(move || {
        let tag_ref_name = format!("refs/tags/{tag_name}");

        // Check if tag exists
        repo_clone
            .refs
            .find(tag_ref_name.as_bytes().as_bstr())
            .map_err(|_| GitError::ReferenceNotFound(tag_name.clone()))?;

        // Delete the tag using transaction
        let ref_name = gix::refs::FullName::try_from(tag_ref_name.as_bytes().as_bstr())
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        let edit = gix::refs::transaction::RefEdit {
            change: gix::refs::transaction::Change::Delete {
                expected: gix::refs::transaction::PreviousValue::Any,
                log: gix::refs::transaction::RefLog::AndReference,
            },
            name: ref_name,
            deref: false,
        };

        repo_clone
            .refs
            .transaction()
            .prepare(
                vec![edit],
                gix::lock::acquire::Fail::Immediately,
                gix::lock::acquire::Fail::Immediately,
            )
            .map_err(|e| GitError::Gix(Box::new(e)))?
            .commit(None)
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        Ok(())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Check if a tag exists
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `tag_name` - Name of the tag to check
///
/// # Returns
///
/// Returns `true` if the tag exists, `false` otherwise.
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, tag_exists};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// if tag_exists(&repo, "v1.0.0").await? {
///     println!("Tag exists!");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn tag_exists(repo: &RepoHandle, tag_name: &str) -> GitResult<bool> {
    let repo_clone = repo.clone_inner();
    let tag_name = tag_name.to_string();

    tokio::task::spawn_blocking(move || {
        let tag_ref_name = format!("refs/tags/{tag_name}");
        Ok(repo_clone
            .refs
            .find(tag_ref_name.as_bytes().as_bstr())
            .is_ok())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// List all tags in the repository
///
/// # Arguments
///
/// * `repo` - Repository handle
///
/// # Returns
///
/// Returns a vector of `TagInfo` for all tags in the repository.
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, list_tags};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// let tags = list_tags(&repo).await?;
/// for tag in tags {
///     println!("Tag: {} -> {}", tag.name, tag.target_commit);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn list_tags(repo: &RepoHandle) -> GitResult<Vec<TagInfo>> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let mut tags = Vec::new();

        // Iterate over all tag references
        let refs_platform = repo_clone
            .references()
            .map_err(|e| GitError::Gix(Box::new(e)))?;
        let tag_refs = refs_platform
            .prefixed("refs/tags/")
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        for reference in tag_refs {
            let mut reference = reference.map_err(GitError::Gix)?;

            let name = reference.name().as_bstr();
            if !name.starts_with(b"refs/tags/") {
                continue;
            }

            let tag_name = name
                .strip_prefix(b"refs/tags/")
                .and_then(|n| std::str::from_utf8(n).ok())
                .ok_or_else(|| GitError::Parse("Invalid tag name".to_string()))?
                .to_string();

            // Get target
            let target_id = reference
                .peel_to_id()
                .map_err(|e| GitError::Gix(Box::new(e)))?;

            // Try to get tag object for annotated tags
            let (message, is_annotated, timestamp) = if let Ok(obj) =
                repo_clone.find_object(target_id)
            {
                if let Ok(tag_obj) = obj.try_into_tag() {
                    let tag_ref = tag_obj.decode().ok();
                    let msg = tag_ref.as_ref().map(|t| t.message.to_string());
                    let ts = if let Some(ref tag) = tag_ref {
                        if let Some(tagger) = &tag.tagger {
                            if let Ok(time) = tagger.time() {
                                DateTime::from_timestamp(time.seconds, 0).unwrap_or_else(Utc::now)
                            } else {
                                Utc::now()
                            }
                        } else {
                            Utc::now()
                        }
                    } else {
                        Utc::now()
                    };
                    (msg, true, ts)
                } else if let Ok(obj2) = repo_clone.find_object(target_id) {
                    if let Ok(commit) = obj2.try_into_commit() {
                        let ts = commit
                            .time()
                            .ok()
                            .unwrap_or_else(gix::date::Time::now_local_or_utc);
                        let ts = DateTime::from_timestamp(ts.seconds, 0).unwrap_or_else(Utc::now);
                        (None, false, ts)
                    } else {
                        (None, false, Utc::now())
                    }
                } else {
                    (None, false, Utc::now())
                }
            } else {
                (None, false, Utc::now())
            };

            tags.push(TagInfo {
                name: tag_name,
                message,
                target_commit: target_id.to_string(),
                timestamp,
                is_annotated,
            });
        }

        Ok(tags)
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Helper function to get signature from repository config
fn get_signature(repo: &gix::Repository) -> GitResult<gix::actor::Signature> {
    let config = repo.config_snapshot();

    let name = config
        .string("user.name")
        .ok_or_else(|| GitError::InvalidInput("Git user.name not configured".to_string()))?;

    let email = config
        .string("user.email")
        .ok_or_else(|| GitError::InvalidInput("Git user.email not configured".to_string()))?;

    Ok(gix::actor::Signature {
        name: name.into_owned(),
        email: email.into_owned(),
        time: gix::date::Time::now_local_or_utc(),
    })
}
