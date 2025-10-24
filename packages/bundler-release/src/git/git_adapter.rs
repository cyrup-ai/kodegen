//! Adapter layer between GitOperations trait and kodgen_git operations.
//!
//! This module provides an implementation of the GitOperations trait that
//! delegates to the kodgen_git package operations, eliminating code duplication.

use crate::error::{GitError, ReleaseError, Result};
use crate::git::{
    BranchInfo, CommitInfo, GitOperations, PushInfo, RemoteInfo, ResetType, TagInfo,
    ValidationResult,
};
use kodegen_tools_git::{
    self as git, AddOpts, CommitOpts, FetchOpts, MergeOpts, MergeOutcome, PushOpts, RepoHandle,
    ResetMode, ResetOpts, TagOpts,
};
use semver::Version;
use std::path::Path;

/// Git operations using kodgen_git backend
#[derive(Debug)]
pub struct KodegenGitOperations {
    repo: RepoHandle,
    _work_dir: std::path::PathBuf,
}

impl KodegenGitOperations {
    /// Open repository using kodgen_git
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo = git::open_repo(path.as_ref())
            .await
            .map_err(|_| GitError::NotRepository)?
            .map_err(|_| GitError::NotRepository)?;

        let _work_dir = repo
            .raw()
            .workdir()
            .ok_or(GitError::NotRepository)?
            .to_path_buf();

        Ok(Self { repo, _work_dir })
    }
}

impl GitOperations for KodegenGitOperations {
    async fn create_release_commit(
        &self,
        version: &Version,
        message: Option<String>,
    ) -> Result<CommitInfo> {
        // Stage all changes
        git::add(
            self.repo.clone(),
            AddOpts {
                paths: vec![std::path::PathBuf::from(".")],
                update_only: false,
                force: false,
            },
        )
        .await
        .map_err(|e| GitError::CommitFailed {
            reason: format!("Failed to add files: {}", e),
        })?;

        // Create commit
        let commit_message = message.unwrap_or_else(|| format!("release: v{}", version));

        let commit_id = git::commit(
            self.repo.clone(),
            CommitOpts {
                message: commit_message.clone(),
                amend: false,
                all: false,
                author: None,
                committer: None,
            },
        )
        .await
        .map_err(|e| GitError::CommitFailed {
            reason: format!("Failed to create commit: {}", e),
        })?;

        // Get commit info using gix directly
        let repo_clone = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let commit =
                repo_clone
                    .raw()
                    .find_commit(commit_id)
                    .map_err(|e| GitError::CommitFailed {
                        reason: format!("Failed to find commit: {}", e),
                    })?;

            let hash = commit.id().to_string();
            let short_hash = commit
                .id()
                .shorten()
                .map(|prefix| prefix.to_string())
                .unwrap_or_else(|_| hash.clone());
            let message = commit
                .message()
                .map(|m| m.summary().to_string())
                .unwrap_or_else(|_| "No commit message".to_string());
            let author = commit.author().map_err(|e| GitError::CommitFailed {
                reason: format!("Failed to get author: {}", e),
            })?;
            let author_name = author.name.to_string();
            let author_email = author.email.to_string();
            // Parse git time format: "<seconds> <timezone>"
            let timestamp = author
                .time
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<i64>().ok())
                .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
                .unwrap_or_else(chrono::Utc::now);
            let parents: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

            Ok(CommitInfo {
                hash,
                short_hash,
                message,
                author_name,
                author_email,
                timestamp,
                parents,
            })
        })
        .await
        .map_err(|e| GitError::CommitFailed {
            reason: format!("Task join error: {}", e),
        })?
    }

    async fn create_version_tag(
        &self,
        version: &Version,
        message: Option<String>,
    ) -> Result<TagInfo> {
        let tag_name = format!("v{}", version);
        let tag_message = message.unwrap_or_else(|| format!("Release v{}", version));

        let tag_info = git::create_tag(
            &self.repo,
            TagOpts {
                name: tag_name.clone(),
                message: Some(tag_message.clone()),
                target: None,
                force: false,
            },
        )
        .await
        .map_err(|_| GitError::TagExists {
            tag: tag_name.clone(),
        })?;

        Ok(TagInfo {
            name: tag_info.name,
            message: tag_info.message,
            target_commit: tag_info.target_commit,
            timestamp: tag_info.timestamp,
            is_annotated: tag_info.is_annotated,
        })
    }

    async fn push_to_remote(&self, remote_name: Option<&str>, push_tags: bool) -> Result<PushInfo> {
        let remote = remote_name.unwrap_or("origin");

        // Fetch from remote to sync refs
        git::fetch(self.repo.clone(), FetchOpts::from_remote(remote))
            .await
            .map_err(|e| {
                ReleaseError::Git(GitError::BranchOperationFailed {
                    reason: format!("Failed to fetch from remote '{}': {}", remote, e),
                })
            })?;

        // Get current branch to determine merge target
        let current_branch = self.get_current_branch().await?;
        let remote_branch = format!("{}/{}", remote, current_branch.name);

        // Try to merge remote changes
        match git::merge(self.repo.clone(), MergeOpts::new(&remote_branch)).await {
            Ok(MergeOutcome::AlreadyUpToDate) => {
                // No remote changes - continue to push
            }
            Ok(MergeOutcome::FastForward(_)) => {
                // Fast-forwarded to remote - continue to push
            }
            Ok(MergeOutcome::MergeCommit(_)) => {
                // Merged divergent changes - continue to push
            }
            Err(kodegen_tools_git::GitError::InvalidInput(ref msg))
                if msg.contains("Invalid merge target") =>
            {
                // Remote branch doesn't exist (first push) - continue to push
            }
            Err(kodegen_tools_git::GitError::MergeConflict(_)) => {
                // FAIL: conflicts must be resolved manually
                return Err(ReleaseError::Git(GitError::BranchOperationFailed {
                    reason: format!(
                        "Cannot push: remote '{}' has conflicting changes. \
                         Resolve conflicts manually then retry release.",
                        remote
                    ),
                }));
            }
            Err(e) => {
                return Err(ReleaseError::Git(GitError::BranchOperationFailed {
                    reason: format!("Failed to merge remote changes: {}", e),
                }));
            }
        }

        let result = git::push(
            &self.repo,
            PushOpts {
                remote: remote.to_string(),
                refspecs: Vec::new(),
                force: false,
                tags: push_tags,
                timeout_secs: None,
            },
        )
        .await
        .map_err(|_| GitError::PushFailed {
            reason: "Channel error".to_string(),
        })?;

        Ok(PushInfo {
            remote_name: remote.to_string(),
            commits_pushed: result.commits_pushed,
            tags_pushed: result.tags_pushed,
            warnings: result.warnings,
        })
    }

    async fn is_working_directory_clean(&self) -> Result<bool> {
        Ok(git::is_clean(&self.repo)
            .await
            .map_err(|_| GitError::BranchOperationFailed {
                reason: "Channel error".to_string(),
            })?)
    }

    async fn get_current_branch(&self) -> Result<BranchInfo> {
        let branch_info =
            git::current_branch(&self.repo)
                .await
                .map_err(|_| GitError::BranchOperationFailed {
                    reason: "Channel error".to_string(),
                })?;

        Ok(BranchInfo {
            name: branch_info.name,
            is_current: branch_info.is_current,
            commit_hash: branch_info.commit_hash,
            upstream: branch_info.upstream,
            ahead_count: branch_info.ahead_count,
            behind_count: branch_info.behind_count,
        })
    }

    async fn reset_to_commit(&self, commit_id: &str, reset_type: ResetType) -> Result<()> {
        let mode = match reset_type {
            ResetType::Soft => ResetMode::Soft,
            ResetType::Mixed => ResetMode::Mixed,
            ResetType::Hard => ResetMode::Hard,
        };

        Ok(git::reset(
            &self.repo,
            ResetOpts {
                target: commit_id.to_string(),
                mode,
                cancel_token: None,
            },
        )
        .await
        .map_err(|_| GitError::BranchOperationFailed {
            reason: "Channel error".to_string(),
        })?)
    }

    async fn delete_tag(&self, tag_name: &str, delete_remote: bool) -> Result<()> {
        // Delete local tag
        git::delete_tag(&self.repo, tag_name).await.map_err(|_| {
            GitError::BranchOperationFailed {
                reason: "Channel error".to_string(),
            }
        })?;

        // Delete remote tag if requested
        if delete_remote {
            git::delete_remote_tag(&self.repo, "origin", tag_name)
                .await
                .map_err(|_| GitError::PushFailed {
                    reason: "Channel error".to_string(),
                })?;
        }

        Ok(())
    }

    async fn get_recent_commits(&self, count: usize) -> Result<Vec<CommitInfo>> {
        let repo_clone = self.repo.clone();

        tokio::task::spawn_blocking(move || {
            let head = repo_clone
                .raw()
                .head()
                .map_err(|e| GitError::BranchOperationFailed {
                    reason: format!("Failed to get HEAD: {}", e),
                })?;

            let mut commits = Vec::new();
            let mut walker = head
                .into_peeled_id()
                .map_err(|e| GitError::BranchOperationFailed {
                    reason: format!("Failed to peel HEAD: {}", e),
                })?
                .ancestors()
                .all()
                .map_err(|e| GitError::BranchOperationFailed {
                    reason: format!("Failed to create commit walker: {}", e),
                })?;

            for _ in 0..count {
                if let Some(commit_result) = walker.next() {
                    let commit_info =
                        commit_result.map_err(|e| GitError::BranchOperationFailed {
                            reason: format!("Failed to get commit: {}", e),
                        })?;

                    let commit = repo_clone
                        .raw()
                        .find_commit(commit_info.id())
                        .map_err(|e| GitError::BranchOperationFailed {
                            reason: format!("Failed to find commit: {}", e),
                        })?;

                    let hash = commit.id().to_string();
                    let short_hash = commit
                        .id()
                        .shorten()
                        .map(|prefix| prefix.to_string())
                        .unwrap_or_else(|_| hash.clone());
                    let message = commit
                        .message()
                        .map(|m| m.summary().to_string())
                        .unwrap_or_else(|_| "No commit message".to_string());
                    let author = commit
                        .author()
                        .map_err(|e| GitError::BranchOperationFailed {
                            reason: format!("Failed to get author: {}", e),
                        })?;
                    let author_name = author.name.to_string();
                    let author_email = author.email.to_string();
                    // Parse git time format: "<seconds> <timezone>"
                    let timestamp = author
                        .time
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse::<i64>().ok())
                        .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
                        .unwrap_or_else(chrono::Utc::now);
                    let parents: Vec<String> =
                        commit.parent_ids().map(|id| id.to_string()).collect();

                    commits.push(CommitInfo {
                        hash,
                        short_hash,
                        message,
                        author_name,
                        author_email,
                        timestamp,
                        parents,
                    });
                } else {
                    break;
                }
            }

            Ok(commits)
        })
        .await
        .map_err(|e| GitError::BranchOperationFailed {
            reason: format!("Task join error: {}", e),
        })?
    }

    async fn tag_exists(&self, tag_name: &str) -> Result<bool> {
        Ok(git::tag_exists(&self.repo, tag_name).await.map_err(|_| {
            GitError::BranchOperationFailed {
                reason: "Channel error".to_string(),
            }
        })?)
    }

    async fn get_remotes(&self) -> Result<Vec<RemoteInfo>> {
        let remotes =
            git::list_remotes(&self.repo)
                .await
                .map_err(|_| GitError::RemoteOperationFailed {
                    operation: "list remotes".to_string(),
                    reason: "Channel error".to_string(),
                })?;

        Ok(remotes
            .into_iter()
            .map(|r| RemoteInfo {
                name: r.name,
                fetch_url: r.fetch_url,
                push_url: r.push_url,
                is_reachable: true, // kodgen_git doesn't provide reachability info
            })
            .collect())
    }

    async fn validate_release_readiness(&self) -> Result<ValidationResult> {
        let mut blocking_issues = Vec::new();
        let mut warnings = Vec::new();

        // Check if we're on a branch (not detached HEAD)
        let is_detached =
            git::is_detached(&self.repo)
                .await
                .map_err(|_| GitError::BranchOperationFailed {
                    reason: "Channel error".to_string(),
                })?;

        if is_detached {
            blocking_issues.push("Repository is in detached HEAD state".to_string());
        }

        // Check if origin remote exists
        let has_origin = git::remote_exists(&self.repo, "origin")
            .await
            .map_err(|_| GitError::RemoteOperationFailed {
                operation: "check origin".to_string(),
                reason: "Channel error".to_string(),
            })?;

        if !has_origin {
            warnings.push("No 'origin' remote configured".to_string());
        }

        let is_ready = blocking_issues.is_empty();
        let status_summary = if is_ready {
            "Repository is ready for release".to_string()
        } else {
            format!("{} issue(s) found", blocking_issues.len())
        };

        Ok(ValidationResult {
            is_ready,
            blocking_issues,
            warnings,
            status_summary,
        })
    }

    async fn create_release_branch(&self, version: &Version) -> Result<BranchInfo> {
        let branch_name = format!("v{}", version);

        // Create branch and checkout using kodgen_git
        // This is equivalent to: git checkout -b vX.X.X
        use kodegen_tools_git::operations::branch::BranchOpts;

        kodegen_tools_git::operations::branch::branch(
            self.repo.clone(),
            BranchOpts::new(&branch_name).checkout(true), // Checkout after creating
        )
        .await
        .map_err(|_| GitError::BranchOperationFailed {
            reason: format!("Failed to create and checkout branch '{}'", branch_name),
        })?
        .map_err(|_| GitError::BranchOperationFailed {
            reason: format!("Failed to create and checkout branch '{}'", branch_name),
        })?;

        // Get branch info after creation
        self.get_current_branch().await
    }

    async fn push_branch_with_tags(
        &self,
        branch_name: &str,
        remote_name: Option<&str>,
    ) -> Result<PushInfo> {
        let remote = remote_name.unwrap_or("origin");

        // Push the specific branch with tags
        // This is equivalent to: git push origin vX.X.X --tags
        use kodegen_tools_git::operations::push::{PushOpts, push};

        let result = push(
            &self.repo,
            PushOpts {
                remote: remote.to_string(),
                refspecs: vec![format!("refs/heads/{}", branch_name)], // Use full ref to avoid ambiguity with tags
                force: false,
                tags: true, // Push tags with the branch
                timeout_secs: None,
            },
        )
        .await
        .map_err(|e| GitError::PushFailed {
            reason: format!(
                "Failed to push branch '{}' to '{}': {}",
                branch_name, remote, e
            ),
        })?;

        Ok(PushInfo {
            remote_name: remote.to_string(),
            commits_pushed: result.commits_pushed,
            tags_pushed: result.tags_pushed,
            warnings: result.warnings,
        })
    }

    async fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        use kodegen_tools_git::operations::checkout::{CheckoutOpts, checkout};

        checkout(self.repo.clone(), CheckoutOpts::new(branch_name))
            .await
            .map_err(|e| GitError::BranchOperationFailed {
                reason: format!("Failed to checkout branch '{}': {}", branch_name, e),
            })?;

        Ok(())
    }

    async fn commit_all_changes(&self, message: &str) -> Result<CommitInfo> {
        use kodegen_tools_git::operations::add::{AddOpts, add};
        use kodegen_tools_git::operations::commit::{CommitOpts, commit};

        // Stage all changes (tracked and untracked)
        add(self.repo.clone(), AddOpts::new(["."]))
            .await
            .map_err(|e| GitError::CommitFailed {
                reason: format!("Failed to stage changes: {}", e),
            })?;

        // Create commit with all staged changes
        let commit_opts = CommitOpts::message(message).all(false); // Already staged via add
        let commit_id =
            commit(self.repo.clone(), commit_opts)
                .await
                .map_err(|e| GitError::CommitFailed {
                    reason: format!("Failed to create commit: {}", e),
                })?;

        // Return minimal commit info with the commit ID
        let commit_hash = commit_id.to_string();
        let short_hash = commit_hash.chars().take(7).collect();

        Ok(CommitInfo {
            hash: commit_hash,
            short_hash,
            message: message.to_string(),
            author_name: "Release Tool".to_string(),
            author_email: "release@kodegen.dev".to_string(),
            timestamp: chrono::Utc::now(),
            parents: Vec::new(),
        })
    }

    async fn merge_branch(&self, branch_name: &str) -> Result<()> {
        use kodegen_tools_git::operations::merge::{MergeOpts, MergeOutcome, merge};

        let outcome = merge(self.repo.clone(), MergeOpts::new(branch_name))
            .await
            .map_err(|e| GitError::BranchOperationFailed {
                reason: format!("Failed to merge branch '{}': {}", branch_name, e),
            })?;

        match outcome {
            MergeOutcome::FastForward(_) | MergeOutcome::MergeCommit(_) => Ok(()),
            MergeOutcome::AlreadyUpToDate => Ok(()),
        }
    }
}
