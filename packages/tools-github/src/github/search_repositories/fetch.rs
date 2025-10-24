//! Repository fetching logic

use chrono::DateTime;
use octocrab::{Octocrab, models::Repository};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::rate_limiter::RateLimiter;
use crate::github::search_repositories::types::{SearchError, SearchQuery, SearchResult};

/// Fetches repositories from GitHub based on query parameters
pub(crate) async fn fetch_repos(
    oc: &Octocrab,
    query: &SearchQuery,
    config: &SearchConfig,
    rate_limiter: &Arc<RwLock<RateLimiter>>,
) -> SearchResult<(Vec<Repository>, u32, u32)> {
    // Check rate limit before making request
    {
        // First, check if rate limit needs reset (requires write lock)
        let mut limiter = rate_limiter.write().await;
        limiter.check_and_reset_if_expired();

        // Check if we can make the request
        if !limiter.can_make_request() {
            return Err(SearchError::RateLimitExceeded {
                remaining: limiter.remaining,
                reset_time: limiter.reset_time,
            });
        }

        // Wait if we're approaching the limit (downgrade to read lock for waiting)
        drop(limiter);
        let limiter = rate_limiter.read().await;
        limiter.wait_if_needed(config.rate_limit_buffer).await?;
    }

    let mut search_terms = query.terms.join(" ");

    // Add filters to search query
    if let Some(lang) = &query.language {
        search_terms.push_str(&format!(" language:{lang}"));
    }

    if let Some(license) = &query.license {
        search_terms.push_str(&format!(" license:{license}"));
    }

    if query.min_stars > 0 {
        search_terms.push_str(&format!(" stars:>={}", query.min_stars));
    }

    if let Some(created_after) = &query.created_after {
        search_terms.push_str(&format!(" created:>{}", created_after.format("%Y-%m-%d")));
    }

    if let Some(pushed_after) = &query.pushed_after {
        search_terms.push_str(&format!(" pushed:>{}", pushed_after.format("%Y-%m-%d")));
    }

    if let Some(topic) = &query.topic {
        search_terms.push_str(&format!(" topic:{topic}"));
    }

    if let Some(user) = &query.user {
        search_terms.push_str(&format!(" user:{user}"));
    }

    if let Some(org) = &query.org {
        search_terms.push_str(&format!(" org:{org}"));
    }

    if query.exclude_forks {
        search_terms.push_str(" fork:false");
    }

    if query.exclude_archived {
        search_terms.push_str(" archived:false");
    }

    let search_future = oc
        .search()
        .repositories(&search_terms)
        .sort("stars")
        .order("desc")
        .per_page(100)
        .send();

    let search_resp = tokio::time::timeout(config.api_timeout, search_future)
        .await
        .map_err(|_| SearchError::TimeoutError {
            operation: "repository_search".to_string(),
            duration: config.api_timeout,
        })?
        .map_err(|e| SearchError::ApiError(e.to_string()))?;

    // Get rate limit info from the rate limit API and update the limiter
    let rate_limit_remaining = match oc.ratelimit().get().await {
        Ok(rate_limit) => {
            let remaining = rate_limit.resources.search.remaining as u32;
            let reset_timestamp = rate_limit.resources.search.reset;
            let reset_time = DateTime::from_timestamp(reset_timestamp as i64, 0)
                .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(1));

            // Update the rate limiter with fresh data
            {
                let mut limiter = rate_limiter.write().await;
                limiter.update(remaining, reset_time);
            }

            remaining
        }
        Err(_) => 5000, // Default fallback if rate limit check fails
    };

    let repos = search_resp.items;
    let total = search_resp.total_count.unwrap_or(0) as u32;

    Ok((repos, total, rate_limit_remaining))
}
