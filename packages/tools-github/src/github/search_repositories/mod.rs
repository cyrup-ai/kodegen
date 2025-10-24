//! GitHub Repository Search Operation
//!
//! This module provides comprehensive GitHub repository search functionality
//! with local analysis, caching, and quality metrics.

mod analysis;
mod cache;
mod config;
mod convenience;
mod fetch;
mod helpers;
mod metrics;
mod rate_limiter;
mod types;

// Re-export public types
pub use config::SearchConfig;
pub use convenience::{search_repositories, search_repositories_with_config};
pub use types::{
    ActivityMetrics, CiCdMetrics, CodeQualityMetrics, DependencyMetrics, DocumentationMetrics,
    LocalMetrics, MetadataInfo, Output, QualityMetrics, ReadmeMetrics, RepositoryResult,
    SearchError, SearchQuery, SearchResult, SecurityMetrics, StructureMetrics, TestMetrics,
};

use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::Stream;
use futures::stream::{self, StreamExt};
use octocrab::{Octocrab, models::Repository};
use tokio::sync::mpsc::Receiver;
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio_stream::wrappers::ReceiverStream;

use analysis::analyze_repo;
use cache::SearchCache;
use fetch::fetch_repos;
use rate_limiter::RateLimiter;

/// Streaming search session
pub struct SearchSession {
    inner: ReceiverStream<SearchResult<Output>>,
}

impl SearchSession {
    fn new(rx: Receiver<SearchResult<Output>>) -> Self {
        Self {
            inner: ReceiverStream::new(rx),
        }
    }
}

impl Stream for SearchSession {
    type Item = SearchResult<Output>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

/// Search provider trait
pub trait SearchProvider: Send + Sync + 'static {
    fn search(&self, query: SearchQuery) -> SearchSession;
    fn search_with_config(&self, query: SearchQuery, config: SearchConfig) -> SearchSession;
}

/// Main GitHub search implementation
pub struct GithubSearch {
    octocrab: Arc<Octocrab>,
    cache: Arc<Mutex<SearchCache>>,
    concurrency: Arc<Semaphore>,
    token: String,
    config: SearchConfig,
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

impl GithubSearch {
    /// Creates a new `GithubSearch` instance with the given configuration
    pub fn new(token: String) -> SearchResult<Self> {
        Self::with_config(token, SearchConfig::default())
    }

    pub fn with_config(token: String, config: SearchConfig) -> SearchResult<Self> {
        let oc = Octocrab::builder()
            .personal_token(token.clone())
            .build()
            .map_err(|e| SearchError::ApiError(e.to_string()))?;

        Ok(Self {
            octocrab: Arc::new(oc),
            cache: Arc::new(Mutex::new(SearchCache::new(
                config.cache_capacity,
                config.cache_ttl,
            ))),
            concurrency: Arc::new(Semaphore::new(config.concurrency_limit)),
            token,
            config,
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new())),
        })
    }

    /// Orchestrates the entire search operation
    async fn run_search(
        query: SearchQuery,
        octocrab: Arc<Octocrab>,
        cache: Arc<Mutex<SearchCache>>,
        concurrency: Arc<Semaphore>,
        token: String,
        config: SearchConfig,
        rate_limiter: Arc<RwLock<RateLimiter>>,
    ) -> SearchResult<Output> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();

        // Cleanup expired cache entries
        {
            let mut c = cache.lock().await;
            c.cleanup_expired();
        }

        let (repos, total_results, rate_limit_remaining) =
            fetch_repos(&octocrab, &query, &config, &rate_limiter).await?;

        if repos.is_empty() {
            return Err(SearchError::NoResults {
                query: query.terms.join(" "),
            });
        }

        // Limit to top 10
        let top_repos = repos.into_iter().take(10).collect::<Vec<_>>();

        // Analyze repositories
        let (results, analysis_errors) = Self::analyze_all(
            top_repos,
            octocrab.clone(),
            cache.clone(),
            concurrency,
            token.clone(),
            config.clone(),
            rate_limiter.clone(),
        )
        .await?;

        errors.extend(analysis_errors);

        // Get cache statistics
        let (cache_hits, cache_misses) = {
            let c = cache.lock().await;
            c.cache_stats()
        };
        let total_cache_ops = cache_hits + cache_misses;
        let cache_hit_rate = if total_cache_ops > 0 {
            cache_hits as f32 / total_cache_ops as f32
        } else {
            0.0
        };

        // Update cache with new results
        {
            let mut c = cache.lock().await;
            for r in &results {
                let last_sha = r
                    .activity_metrics
                    .as_ref()
                    .map(|a| a.last_commit.clone())
                    .unwrap_or_default();
                c.put(r.full_name.clone(), r.clone(), last_sha);
            }
        }

        let processing_time = start_time.elapsed().as_millis();

        Ok(Output {
            status: if errors.is_empty() {
                "success".to_string()
            } else {
                "partial".to_string()
            },
            results,
            metadata: MetadataInfo {
                total_results,
                cache_hit_rate,
                cache_hits,
                cache_misses,
                processing_time_ms: processing_time,
                api_rate_limit_remaining: rate_limit_remaining,
                partial_results: !errors.is_empty(),
            },
            errors,
        })
    }

    /// Analyzes all repositories in parallel
    async fn analyze_all(
        repos: Vec<Repository>,
        octocrab: Arc<Octocrab>,
        cache: Arc<Mutex<SearchCache>>,
        concurrency: Arc<Semaphore>,
        token: String,
        config: SearchConfig,
        rate_limiter: Arc<RwLock<RateLimiter>>,
    ) -> SearchResult<(Vec<RepositoryResult>, Vec<String>)> {
        // Create futures for parallel repository analysis
        let futures = repos.into_iter().map(|repo| {
            // Clone all Arc references for move into async closure
            let octocrab = octocrab.clone();
            let cache = cache.clone();
            let concurrency = concurrency.clone();
            let token = token.clone();
            let config = config.clone();
            let rate_limiter = rate_limiter.clone();

            async move {
                // Acquire semaphore permit for concurrency control
                let permit = match concurrency.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        return Err(SearchError::LocalAnalysisError(
                            "Concurrency limit reached".to_string(),
                        ));
                    }
                };

                // Analyze repository
                let result = analyze_repo(octocrab, cache, repo, token, config, rate_limiter).await;

                // Release permit via RAII
                drop(permit);

                result
            }
        });

        // Execute futures concurrently with bounded parallelism
        let all_results = stream::iter(futures)
            .buffer_unordered(config.concurrency_limit)
            .collect::<Vec<SearchResult<RepositoryResult>>>()
            .await;

        // Partition results into successes and errors
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for result in all_results {
            match result {
                Ok(repo_result) => results.push(repo_result),
                Err(e) => errors.push(e.to_string()),
            }
        }

        Ok((results, errors))
    }
}

impl SearchProvider for GithubSearch {
    /// Executes a repository search with the instance's default configuration.
    fn search(&self, query: SearchQuery) -> SearchSession {
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        // Clone all necessary data for the spawned task
        let octocrab = self.octocrab.clone();
        let cache = self.cache.clone();
        let concurrency = self.concurrency.clone();
        let token = self.token.clone();
        let config = self.config.clone();
        let rate_limiter = self.rate_limiter.clone();

        // Spawn async task to perform the search
        tokio::spawn(async move {
            let result = Self::run_search(
                query,
                octocrab,
                cache,
                concurrency,
                token,
                config,
                rate_limiter,
            )
            .await;

            // Send the result through the channel (ignore send errors if receiver dropped)
            let _ = tx.send(result).await;
        });

        SearchSession::new(rx)
    }

    /// Executes a repository search with a custom configuration.
    fn search_with_config(&self, query: SearchQuery, config: SearchConfig) -> SearchSession {
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        // Clone all necessary data for the spawned task
        let octocrab = self.octocrab.clone();
        let cache = self.cache.clone();
        let concurrency = self.concurrency.clone();
        let token = self.token.clone();
        let rate_limiter = self.rate_limiter.clone();

        // Spawn async task to perform the search with custom config
        tokio::spawn(async move {
            let result = Self::run_search(
                query,
                octocrab,
                cache,
                concurrency,
                token,
                config, // Use the provided config instead of self.config
                rate_limiter,
            )
            .await;

            // Send the result through the channel (ignore send errors if receiver dropped)
            let _ = tx.send(result).await;
        });

        SearchSession::new(rx)
    }
}
