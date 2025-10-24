//! GitHub Pull Request reviews listing operation.

use crate::github::error::GitHubError;
use crate::runtime::{AsyncStream, EmitterBuilder};
use octocrab::{Octocrab, Page, models::pulls::Review};
use std::sync::Arc;

/// Stream PR reviews.
pub(crate) fn get_pull_request_reviews(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    pr_number: u64,
) -> AsyncStream<Result<Review, GitHubError>> {
    let (owner, repo) = (owner.into(), repo.into());

    let builder = EmitterBuilder::new(Box::new(move || {
        Box::pin(async move {
            let mut reviews = Vec::new();
            let mut page: Page<Review> = inner
                .pulls(&owner, &repo)
                .list_reviews(pr_number)
                .per_page(100)
                .send()
                .await
                .map_err(GitHubError::from)?;

            reviews.extend(page.items);

            while let Some(next) = inner.get_page::<Review>(&page.next).await? {
                page = next;
                reviews.extend(page.items);
            }
            Ok(reviews)
        })
    }));
    builder.emit(|v| v, |_| {})
}
