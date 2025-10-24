//! GitHub Issue comments listing operation.

use crate::github::error::GitHubError;
use crate::runtime::{AsyncStream, EmitterBuilder};
use octocrab::{Octocrab, Page, models::issues::Comment};
use std::sync::Arc;

/// Fetch all comments for an issue as a stream.
pub(crate) fn get_issue_comments(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    issue_number: u64,
) -> AsyncStream<Result<Comment, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();
    let builder = EmitterBuilder::new(Box::new(move || {
        Box::pin(async move {
            let mut comments = Vec::new();
            let mut page: Page<Comment> = inner
                .issues(&owner, &repo)
                .list_comments(issue_number)
                .per_page(100)
                .send()
                .await
                .map_err(GitHubError::from)?;

            comments.extend(page.items);

            while let Some(next_page) = inner.get_page::<Comment>(&page.next).await? {
                page = next_page;
                comments.extend(page.items);
            }
            Ok(comments)
        })
    }));
    // Identity handlers – no transformation, no side-effect error handling
    builder.emit(|v| v, |_| {})
}
