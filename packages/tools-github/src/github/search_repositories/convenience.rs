//! Convenience wrapper functions for `search_repositories` functionality

use super::{
    GithubSearch, Output, SearchConfig, SearchError, SearchProvider, SearchQuery, SearchResult,
};
use futures::StreamExt;

/// Convenience function to search GitHub repositories with a simple function call.
///
/// This wraps the more verbose `GithubSearch` API with a simple async function
/// that returns results directly instead of requiring stream handling.
///
/// # Arguments
///
/// * `token` - GitHub personal access token
/// * `query` - Search query parameters
///
/// # Returns
///
/// Returns a `SearchResult<Output>` containing repository results and metadata.
///
/// # Example
///
/// ```rust,no_run
/// use gitgix::{search_repositories, SearchQuery};
/// use chrono::Utc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let token = std::env::var("GITHUB_TOKEN")?;
///     
///     let query = SearchQuery {
///         terms: vec!["rust".to_string(), "web".to_string()],
///         language: Some("Rust".to_string()),
///         min_stars: 100,
///         license: None,
///         created_after: None,
///         pushed_after: None,
///         topic: None,
///         user: None,
///         org: None,
///         exclude_forks: true,
///         exclude_archived: true,
///     };
///     
///     let output = gitgix::search_repositories(&token, query).await?;
///     
///     println!("Found {} repositories", output.results.len());
///     for repo in output.results {
///         println!("- {}: {} stars", repo.name, repo.stars);
///         
///         if let Some(metrics) = repo.local_metrics {
///             println!("  Vulnerable deps: {}", metrics.dependency_metrics.vulnerable_dependencies);
///             println!("  Outdated deps: {}", metrics.dependency_metrics.outdated_dependencies);
///         }
///     }
///     
///     Ok(())
/// }
/// ```
pub async fn search_repositories(token: &str, query: SearchQuery) -> SearchResult<Output> {
    let search = GithubSearch::new(token.to_string())?;
    let mut session = search.search(query);

    // Get the single result from the stream
    session
        .next()
        .await
        .ok_or_else(|| SearchError::LocalAnalysisError("No results from search".to_string()))?
}

/// Convenience function to search GitHub repositories with custom configuration.
///
/// Like `search_repositories()` but allows specifying custom `SearchConfig` for
/// fine-tuning caching, concurrency, timeouts, etc.
///
/// # Arguments
///
/// * `token` - GitHub personal access token
/// * `query` - Search query parameters
/// * `config` - Custom search configuration
///
/// # Returns
///
/// Returns a `SearchResult<Output>` containing repository results and metadata.
///
/// # Example
///
/// ```rust,no_run
/// use gitgix::{search_repositories_with_config, SearchQuery, SearchConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let token = std::env::var("GITHUB_TOKEN")?;
///     
///     let query = SearchQuery {
///         terms: vec!["rust".to_string()],
///         language: Some("Rust".to_string()),
///         min_stars: 1000,
///         ..Default::default()
///     };
///     
///     let config = SearchConfig {
///         concurrency_limit: 5,
///         cache_capacity: 200,
///         ..Default::default()
///     };
///     
///     let output = gitgix::search_repositories_with_config(&token, query, config).await?;
///     
///     println!("Cache hit rate: {:.1}%", output.metadata.cache_hit_rate * 100.0);
///     
///     Ok(())
/// }
/// ```
pub async fn search_repositories_with_config(
    token: &str,
    query: SearchQuery,
    config: SearchConfig,
) -> SearchResult<Output> {
    let search = GithubSearch::with_config(token.to_string(), config)?;
    let mut session = search.search(query);

    // Get the single result from the stream
    session
        .next()
        .await
        .ok_or_else(|| SearchError::LocalAnalysisError("No results from search".to_string()))?
}
