//! GitHub Pull Request comments listing operation.

use crate::github::error::GitHubError;
use crate::runtime::{AsyncStream, EmitterBuilder};
use octocrab::{Octocrab, Page};
use std::sync::Arc;

/// Stream PR review comments.
pub(crate) fn get_pull_request_comments(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    pr_number: u64,
) -> AsyncStream<Result<octocrab::models::pulls::Comment, GitHubError>> {
    let (owner, repo) = (owner.into(), repo.into());

    let builder = EmitterBuilder::new(Box::new(move || {
        Box::pin(async move {
            let mut comments = Vec::new();
            let mut page: Page<octocrab::models::pulls::Comment> = inner
                .pulls(&owner, &repo)
                .list_comments(Some(pr_number))
                .per_page(100)
                .send()
                .await
                .map_err(GitHubError::from)?;

            comments.extend(page.items);
            while let Some(next) = inner
                .get_page::<octocrab::models::pulls::Comment>(&page.next)
                .await?
            {
                page = next;
                comments.extend(page.items);
            }
            Ok(comments)
        })
    }));
    builder.emit(|v| v, |_| {})
}
