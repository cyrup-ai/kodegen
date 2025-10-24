//! GitHub Issues search operation.

use crate::github::error::GitHubError;
use crate::runtime::{AsyncStream, EmitterBuilder};
use octocrab::{Octocrab, Page, models::issues::Issue};
use std::sync::Arc;

/// GitHub search API for issues and PRs.
pub(crate) fn search_issues(
    inner: Arc<Octocrab>,
    query: impl Into<String>,
    sort: Option<String>,
    order: Option<String>,
    page: Option<u32>,
    per_page: Option<u8>,
) -> AsyncStream<Result<Issue, GitHubError>> {
    let q = query.into();
    let builder = EmitterBuilder::new(Box::new(move || {
        Box::pin(async move {
            let mut results = Vec::new();
            let mut req = inner
                .search()
                .issues_and_pull_requests(&q)
                .per_page(per_page.unwrap_or(100))
                .page(page.unwrap_or(1));

            if let Some(s) = &sort {
                req = req.sort(s);
            }
            if let Some(o) = &order {
                req = req.order(o);
            }

            let mut page_res: Page<Issue> = req.send().await.map_err(GitHubError::from)?;
            results.extend(page_res.items);

            while let Some(next_page) = inner.get_page::<Issue>(&page_res.next).await? {
                page_res = next_page;
                results.extend(page_res.items);
            }
            Ok(results)
        })
    }));
    builder.emit(|v| v, |_| {})
}
