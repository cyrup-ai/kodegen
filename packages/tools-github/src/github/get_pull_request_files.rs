//! GitHub Pull Request files listing operation.

use crate::github::error::GitHubError;
use crate::runtime::{AsyncStream, EmitterBuilder};
use octocrab::{Octocrab, Page, models::repos::DiffEntry as PrFile};
use std::sync::Arc;

/// Stream files changed in a PR.
pub(crate) fn get_pull_request_files(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    pr_number: u64,
) -> AsyncStream<Result<PrFile, GitHubError>> {
    let (owner, repo) = (owner.into(), repo.into());

    let builder = EmitterBuilder::new(Box::new(move || {
        Box::pin(async move {
            let mut files = Vec::new();
            let mut page: Page<PrFile> = inner
                .pulls(&owner, &repo)
                .list_files(pr_number)
                .await
                .map_err(GitHubError::from)?;

            files.extend(page.items);

            while let Some(next) = inner.get_page::<PrFile>(&page.next).await? {
                page = next;
                files.extend(page.items);
            }
            Ok(files)
        })
    }));
    builder.emit(|v| v, |_| {})
}
