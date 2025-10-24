//! GitHub code search operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::Code};
use std::sync::Arc;

/// Enrich code search results with star counts by fetching full repository details
async fn enrich_code_results_with_stars(
    octocrab: Arc<Octocrab>,
    mut page: octocrab::Page<Code>,
) -> Result<octocrab::Page<Code>, GitHubError> {
    use std::collections::HashMap;
    use tokio::task::JoinSet;

    if page.items.is_empty() {
        return Ok(page);
    }

    let mut join_set = JoinSet::new();

    for (idx, code_item) in page.items.iter().enumerate() {
        // Skip if already has stars
        if code_item.repository.stargazers_count.is_some() {
            continue;
        }

        let octocrab = Arc::clone(&octocrab);
        let owner = code_item
            .repository
            .owner
            .as_ref()
            .map(|o| o.login.clone())
            .unwrap_or_default();
        let repo_name = code_item.repository.name.clone();

        join_set.spawn(async move {
            match octocrab.repos(&owner, &repo_name).get().await {
                Ok(repo) => Some((
                    idx,
                    repo.stargazers_count,
                    repo.forks_count,
                    repo.open_issues_count,
                )),
                Err(_) => None,
            }
        });
    }

    let mut enrichments = HashMap::new();
    while let Some(result) = join_set.join_next().await {
        if let Ok(Some((idx, stars, forks, issues))) = result {
            enrichments.insert(idx, (stars, forks, issues));
        }
    }

    for (idx, (stars, forks, issues)) in enrichments {
        if let Some(item) = page.items.get_mut(idx) {
            item.repository.stargazers_count = stars;
            item.repository.forks_count = forks;
            item.repository.open_issues_count = issues;
        }
    }

    Ok(page)
}

/// Search for code across GitHub repositories.
pub(crate) fn search_code(
    inner: Arc<Octocrab>,
    query: impl Into<String>,
    sort: Option<String>,
    order: Option<String>,
    page: Option<u32>,
    per_page: Option<u8>,
    enrich_stars: bool,
) -> AsyncTask<Result<octocrab::Page<Code>, GitHubError>> {
    let query = query.into();

    spawn_task(async move {
        let mut request = inner.search().code(&query);

        if let Some(sort_val) = sort {
            // Valid values: "indexed"
            request = request.sort(&sort_val);
        }

        if let Some(order_val) = order {
            // Valid values: "asc", "desc"
            request = request.order(&order_val);
        }

        if let Some(p) = page {
            request = request.page(p);
        }

        if let Some(pp) = per_page {
            request = request.per_page(pp);
        }

        let mut results = request.send().await.map_err(GitHubError::from)?;

        if enrich_stars {
            results = enrich_code_results_with_stars(inner, results).await?;
        }

        Ok(results)
    })
}
