//! GitHub code search operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{models::Code, Octocrab};
use std::sync::Arc;

/// Search for code across GitHub repositories.
pub(crate) fn search_code(
    inner: Arc<Octocrab>,
    query: impl Into<String>,
    sort: Option<String>,
    order: Option<String>,
    page: Option<u32>,
    per_page: Option<u8>,
) -> AsyncTask<Result<octocrab::Page<Code>, GitHubError>> {
    let query = query.into();

    spawn_task(async move {
        let mut request = inner
            .search()
            .code(&query);
        
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
        
        let results = request
            .send()
            .await
            .map_err(GitHubError::from)?;
        
        Ok(results)
    })
}
