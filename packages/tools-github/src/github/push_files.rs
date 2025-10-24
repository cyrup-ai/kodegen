//! GitHub Multiple files push operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{
    Octocrab,
    models::repos::{Commit, Ref},
};
use std::collections::HashMap;
use std::sync::Arc;

/// Push multiple files in **one** commit (tree + commit + update-ref).
pub(crate) fn push_files(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    branch: impl Into<String>,
    files: HashMap<String, String>, // path -> base64-content
    message: impl Into<String>,
) -> AsyncTask<Result<Commit, GitHubError>> {
    let (owner, repo, branch, message) = (owner.into(), repo.into(), branch.into(), message.into());

    spawn_task(async move {
        // 1. Get latest commit SHA of branch
        let reference: Ref = inner
            .get(
                format!("repos/{owner}/{repo}/git/ref/heads/{branch}"),
                None::<&()>,
            )
            .await
            .map_err(GitHubError::from)?;

        let base_tree_sha = match reference.object {
            octocrab::models::repos::Object::Commit { sha, .. } => sha,
            octocrab::models::repos::Object::Tag { sha, .. } => sha,
            _ => return Err(GitHubError::Custom("Unexpected object type".into())),
        };

        // 2. Create a blob per file
        let mut tree_entries = Vec::new();
        for (path, content) in files {
            let blob: serde_json::Value = inner
                .post(
                    format!("repos/{owner}/{repo}/git/blobs"),
                    Some(&serde_json::json!({
                        "content": content,
                        "encoding": "base64"
                    })),
                )
                .await
                .map_err(GitHubError::from)?;

            tree_entries.push(serde_json::json!({
                "path": path,
                "mode": "100644",
                "type": "blob",
                "sha": blob["sha"]
            }));
        }

        // 3. Create tree
        let tree: serde_json::Value = inner
            .post(
                format!("repos/{owner}/{repo}/git/trees"),
                Some(&serde_json::json!({
                    "base_tree": base_tree_sha,
                    "tree": tree_entries
                })),
            )
            .await
            .map_err(GitHubError::from)?;

        // 4. Create commit
        let commit: Commit = inner
            .post(
                format!("repos/{owner}/{repo}/git/commits"),
                Some(&serde_json::json!({
                    "message": message,
                    "tree": tree["sha"],
                    "parents": [base_tree_sha]
                })),
            )
            .await
            .map_err(GitHubError::from)?;

        // 5. Update ref
        inner
            .patch::<(), _, _>(
                format!("repos/{owner}/{repo}/git/refs/heads/{branch}"),
                Some(&serde_json::json!({ "sha": commit.sha })),
            )
            .await
            .map_err(GitHubError::from)?;

        Ok(commit)
    })
}
