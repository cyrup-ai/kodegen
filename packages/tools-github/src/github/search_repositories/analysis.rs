//! Repository analysis logic

use chrono::Utc;
use log::{info, warn};
use octocrab::{
    Octocrab,
    models::{Repository, repos::RepoCommit},
    params,
};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::{Mutex, RwLock};

use crate::github::search_repositories::cache::SearchCache;
use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::metrics::{
    MetricsCollectionContext, collect_local_metrics,
};
use crate::github::search_repositories::rate_limiter::RateLimiter;
use crate::github::search_repositories::types::{
    ActivityMetrics, LocalScores, QualityMetrics, RepositoryResult, SearchError, SearchResult,
    SecurityMetrics, WikiInfo,
};

/// Analyzes a single repository with caching
pub(crate) async fn analyze_repo(
    octocrab: Arc<Octocrab>,
    cache: Arc<Mutex<SearchCache>>,
    repo: Repository,
    _token: String,
    config: SearchConfig,
    _rate_limiter: Arc<RwLock<RateLimiter>>,
) -> SearchResult<RepositoryResult> {
    let repo_name = repo.full_name.as_deref().unwrap_or("unknown");
    let url = repo.clone_url.as_ref().map_or("", reqwest::Url::as_str);
    let stars = repo.stargazers_count.unwrap_or(0);

    // Extract owner safely - repository must have owner to fetch commits
    let owner_login = repo
        .owner
        .as_ref()
        .ok_or_else(|| SearchError::InvalidQuery {
            details: format!(
                "Repository '{repo_name}' has no owner information - cannot fetch commits"
            ),
        })?
        .login
        .as_str();

    // Get latest commit
    let commits_resp = octocrab
        .repos(owner_login, &repo.name)
        .list_commits()
        .per_page(1)
        .send()
        .await
        .map_err(|e| SearchError::ApiError(e.to_string()))?;

    let latest_sha = commits_resp
        .items
        .first()
        .map(|c| c.sha.clone())
        .unwrap_or_default();

    // Check cache
    {
        let mut c = cache.lock().await;
        if let Some(found) = c.get_if_valid(repo_name, &latest_sha) {
            info!("Cache hit for {repo_name}");
            return Ok(found);
        }
    }

    // Compute activity metrics
    let activity = compute_activity(&commits_resp.items, &octocrab, owner_login, &repo.name).await;

    // Compute API metrics
    let (api_score, ()) = compute_api_metrics(&repo, &activity);

    // Extract wiki information
    let wiki_info = WikiInfo {
        has_wiki: repo.has_wiki.unwrap_or(false),
        clone_url: url.to_string(),
    };

    // Perform local analysis
    let context = LocalAnalysisContext {
        repo_name,
        url,
        owner: owner_login,
        repo_name_str: &repo.name,
        wiki_info,
    };
    let local_scores = local_analysis(context, &repo, octocrab.clone(), &config).await?;

    // Combine scores
    let overall_score = 0.7 * api_score + 0.3 * local_scores.overall_local;

    let result = RepositoryResult {
        name: repo.name.clone(),
        full_name: repo_name.to_string(),
        url: repo
            .html_url
            .as_ref()
            .map_or("", reqwest::Url::as_str)
            .to_string(),
        clone_url: url.to_string(),
        description: repo.description,
        stars,
        forks: repo.forks_count.unwrap_or(0),
        watchers: repo.watchers_count.unwrap_or(0),
        language: repo
            .language
            .as_ref()
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string),
        topics: repo.topics.unwrap_or_default(),
        license: repo.license.map(|l| l.name),
        created_at: repo.created_at.unwrap_or_else(Utc::now),
        updated_at: repo.updated_at.unwrap_or_else(Utc::now),
        pushed_at: repo.pushed_at.unwrap_or_else(Utc::now),
        size_kb: repo.size.unwrap_or(0),
        quality_metrics: QualityMetrics {
            overall_score,
            api_score,
            local_score: local_scores.overall_local,
            popularity_score: local_scores.readme_score,
            maintenance_score: local_scores.coverage_score,
            documentation_score: local_scores.readme_score,
            security_score: local_scores
                .metrics
                .as_ref()
                .map_or(0.5, |m| calculate_security_score(&m.security_metrics)),
        },
        activity_metrics: activity,
        local_metrics: local_scores.metrics,
        errors: vec![],
    };

    Ok(result)
}

/// Computes activity metrics from commit history
pub(crate) async fn compute_activity(
    commits: &[RepoCommit],
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
) -> Option<ActivityMetrics> {
    if commits.is_empty() {
        return None;
    }

    let last_commit = &commits[0];
    let now = Utc::now();
    let thirty_days_ago = now - chrono::Duration::days(30);
    let ninety_days_ago = now - chrono::Duration::days(90);
    let six_months_ago = now - chrono::Duration::days(180);
    let one_year_ago = now - chrono::Duration::days(365);

    // Time-based commit counts
    let commits_last_month = commits
        .iter()
        .filter(|c| {
            c.commit
                .author
                .as_ref()
                .and_then(|a| a.date)
                .is_some_and(|date| date > thirty_days_ago)
        })
        .count() as u32;

    let commits_last_6_months = commits
        .iter()
        .filter(|c| {
            c.commit
                .author
                .as_ref()
                .and_then(|a| a.date)
                .is_some_and(|date| date > six_months_ago)
        })
        .count() as u32;

    let commits_last_year = commits
        .iter()
        .filter(|c| {
            c.commit
                .author
                .as_ref()
                .and_then(|a| a.date)
                .is_some_and(|date| date > one_year_ago)
        })
        .count() as u32;

    // Active contributors (unique authors in last 3 months)
    let active_authors: HashSet<String> = commits
        .iter()
        .filter(|c| {
            c.commit
                .author
                .as_ref()
                .and_then(|a| a.date)
                .is_some_and(|date| date > ninety_days_ago)
        })
        .filter_map(|c| c.commit.author.as_ref().and_then(|a| a.email.clone()))
        .collect();

    let active_contributors_last_3_months = active_authors.len() as u32;

    // Fetch contributors count
    let contributors_count = match octocrab
        .repos(owner, repo)
        .list_contributors()
        .per_page(100)
        .send()
        .await
    {
        Ok(contributors_page) => contributors_page.items.len() as u32,
        Err(e) => {
            warn!("Failed to fetch contributors for {owner}/{repo}: {e}");
            1
        }
    };

    // Fetch merged PRs in last month
    let pull_requests_merged_last_month = match octocrab
        .pulls(owner, repo)
        .list()
        .state(params::State::Closed)
        .per_page(100)
        .send()
        .await
    {
        Ok(prs_page) => prs_page
            .items
            .iter()
            .filter(|pr| {
                pr.merged_at
                    .is_some_and(|merged_at| merged_at > thirty_days_ago)
            })
            .count() as u32,
        Err(e) => {
            warn!("Failed to fetch pull requests for {owner}/{repo}: {e}");
            0
        }
    };

    // Fetch closed issues in last month (excluding PRs)
    let issues_closed_last_month = match octocrab
        .issues(owner, repo)
        .list()
        .state(params::State::Closed)
        .per_page(100)
        .send()
        .await
    {
        Ok(issues_page) => issues_page
            .items
            .iter()
            .filter(|issue| {
                issue.pull_request.is_none()
                    && issue
                        .closed_at
                        .is_some_and(|closed| closed > thirty_days_ago)
            })
            .count() as u32,
        Err(e) => {
            warn!("Failed to fetch issues for {owner}/{repo}: {e}");
            0
        }
    };

    // Fetch releases and calculate frequency
    let (release_frequency, latest_release) = match octocrab
        .repos(owner, repo)
        .releases()
        .list()
        .per_page(20)
        .send()
        .await
    {
        Ok(releases_page) => {
            let releases = releases_page.items;

            let latest = releases.first().map(|r| r.tag_name.clone());

            let frequency = if releases.len() >= 2 {
                let newest = releases[0]
                    .created_at
                    .or(releases[0].published_at)
                    .unwrap_or_else(Utc::now);

                let oldest = releases[releases.len() - 1]
                    .created_at
                    .or(releases[releases.len() - 1].published_at)
                    .unwrap_or_else(Utc::now);

                let days_between = (newest - oldest).num_days();
                let avg_days = days_between / (releases.len() as i64 - 1);

                if avg_days < 30 {
                    "monthly"
                } else if avg_days < 90 {
                    "quarterly"
                } else if avg_days < 180 {
                    "biannual"
                } else {
                    "annual"
                }
            } else if releases.len() == 1 {
                "single"
            } else {
                "none"
            };

            (frequency.to_string(), latest)
        }
        Err(e) => {
            warn!("Failed to fetch releases for {owner}/{repo}: {e}");
            ("unknown".to_string(), None)
        }
    };

    Some(ActivityMetrics {
        commits_last_month,
        commits_last_6_months,
        commits_last_year,
        last_commit: last_commit.sha.clone(),
        last_commit_date: last_commit
            .commit
            .author
            .as_ref()
            .and_then(|a| a.date)
            .unwrap_or_else(Utc::now),
        contributors_count,
        active_contributors_last_3_months,
        pull_requests_merged_last_month,
        issues_closed_last_month,
        release_frequency,
        latest_release,
    })
}

/// Queries the latest GitHub Actions workflow run status
async fn query_build_status(octocrab: &Octocrab, owner: &str, repo: &str) -> String {
    match octocrab
        .workflows(owner, repo)
        .list_all_runs()
        .per_page(1)
        .send()
        .await
    {
        Ok(runs_page) => {
            if let Some(run) = runs_page.items.first() {
                // If workflow completed, return conclusion
                if let Some(conclusion) = &run.conclusion {
                    return conclusion.to_lowercase();
                }
                // Still running/queued
                return "pending".to_string();
            }
            // Repository has GitHub Actions but no runs yet
            "no_runs".to_string()
        }
        Err(e) => {
            warn!("Failed to query build status for {owner}/{repo}: {e}");
            "unknown".to_string()
        }
    }
}

/// Computes API-based quality metrics from repository metadata
pub(crate) fn compute_api_metrics(
    repo: &Repository,
    activity: &Option<ActivityMetrics>,
) -> (f32, ()) {
    let mut score: f32 = 0.0;

    // Stars contribute to score
    let stars = repo.stargazers_count.unwrap_or(0);
    if stars > 1000 {
        score += 0.3;
    } else if stars > 100 {
        score += 0.2;
    } else if stars > 10 {
        score += 0.1;
    }

    // Forks contribute to score
    let forks = repo.forks_count.unwrap_or(0);
    if forks > 100 {
        score += 0.2;
    } else if forks > 10 {
        score += 0.1;
    }

    // Recent activity contributes to score
    if let Some(act) = activity
        && act.commits_last_month > 0
    {
        score += 0.2;
    }

    // License presence
    if repo.license.is_some() {
        score += 0.1;
    }

    // Description presence
    if repo.description.is_some() {
        score += 0.1;
    }

    // Topics presence
    if let Some(topics) = &repo.topics
        && !topics.is_empty()
    {
        score += 0.1;
    }

    (score.min(1.0), ())
}

/// Context for local repository analysis.
pub(crate) struct LocalAnalysisContext<'a> {
    pub repo_name: &'a str,
    pub url: &'a str,
    pub owner: &'a str,
    pub repo_name_str: &'a str,
    pub wiki_info: WikiInfo,
}

/// Performs local repository analysis by cloning and scanning
pub(crate) async fn local_analysis(
    context: LocalAnalysisContext<'_>,
    repo: &Repository,
    octocrab: Arc<Octocrab>,
    config: &SearchConfig,
) -> SearchResult<LocalScores> {
    // Check repository size before cloning
    let repo_size_kb = u64::from(repo.size.unwrap_or(0));
    let repo_size_bytes = repo_size_kb * 1024;

    if repo_size_bytes > config.max_repo_size {
        warn!(
            "Repository {} too large: {} bytes (max: {} bytes) - skipping clone",
            context.repo_name, repo_size_bytes, config.max_repo_size
        );

        // Return default metrics without local analysis
        return Ok(LocalScores {
            overall_local: 0.3, // Low score for oversized repos
            readme_score: 0.0,
            coverage_score: 0.0,
            metrics: None,
        });
    }

    // Create temp directory for cloning
    let temp_dir =
        TempDir::new().map_err(|e| SearchError::LocalAnalysisError(format!("Temp dir: {e}")))?;

    let repo_path = temp_dir.path();

    // Clone repository using gix with timeout protection
    let url_owned = context.url.to_string();
    let repo_path_owned = repo_path.to_path_buf();

    let clone_result = tokio::time::timeout(
        config.fetch_timeout,
        tokio::task::spawn_blocking(move || {
            // Parse URL to gix::Url type
            let parsed_url = gix::url::parse(url_owned.as_str().into())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let mut prep = gix::prepare_clone(parsed_url, &repo_path_owned)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let (checkout, outcome) = prep
                .fetch_then_checkout(gix::progress::Discard, &AtomicBool::new(false))
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((checkout, outcome))
        }),
    )
    .await;

    let gix_repo = match clone_result {
        // Timeout occurred
        Err(_) => {
            warn!(
                "Clone timeout for {} after {:?}",
                context.repo_name, config.fetch_timeout
            );
            return Err(SearchError::TimeoutError {
                operation: format!("git_clone_{}", context.repo_name),
                duration: config.fetch_timeout,
            });
        }
        // spawn_blocking panicked or was cancelled
        Ok(Err(e)) => {
            warn!("Clone task failed for {}: {:?}", context.repo_name, e);
            return Ok(LocalScores {
                overall_local: 0.3,
                readme_score: 0.0,
                coverage_score: 0.0,
                metrics: None,
            });
        }
        // Clone operation failed
        Ok(Ok(Err(e))) => {
            warn!("Clone failed for {}: {}", context.repo_name, e);
            return Ok(LocalScores {
                overall_local: 0.3,
                readme_score: 0.0,
                coverage_score: 0.0,
                metrics: None,
            });
        }
        // Success
        Ok(Ok(Ok((checkout, _)))) => Some(checkout.persist()),
    };

    // Calculate signed commits ratio synchronously to avoid Send issues
    let signed_commits_ratio = if let Some(repo) = gix_repo.as_ref() {
        calculate_signed_commits_ratio(repo)
    } else {
        0.0
    };

    // Query build status if GitHub Actions is configured
    let build_status = if repo_path.join(".github/workflows").exists() {
        query_build_status(&octocrab, context.owner, context.repo_name_str).await
    } else {
        "no_ci".to_string()
    };

    // Collect metrics with timeout protection (pass ratio instead of repo reference)
    let metrics_context = MetricsCollectionContext {
        signed_commits_ratio,
        build_status,
        owner: context.owner,
        repo: context.repo_name_str,
        wiki_info: context.wiki_info,
    };
    let local_metrics = match tokio::time::timeout(
        Duration::from_secs(10),
        collect_local_metrics(repo_path, metrics_context, config, &octocrab),
    )
    .await
    {
        Ok(Some(metrics)) => metrics,
        Ok(None) => {
            warn!("Metrics collection failed for {}", context.repo_name);
            return Ok(LocalScores {
                overall_local: 0.3,
                readme_score: 0.0,
                coverage_score: 0.0,
                metrics: None,
            });
        }
        Err(_) => {
            warn!("Timeout collecting metrics for {}", context.repo_name);
            return Ok(LocalScores {
                overall_local: 0.3,
                readme_score: 0.0,
                coverage_score: 0.0,
                metrics: None,
            });
        }
    };

    // Calculate scores from metrics
    let readme_score = local_metrics.readme_quality.quality_score / 100.0;
    let coverage_score = local_metrics.test_metrics.test_coverage_estimate;
    let overall_local = f32::midpoint(readme_score, coverage_score);

    Ok(LocalScores {
        overall_local,
        readme_score,
        coverage_score,
        metrics: Some(local_metrics),
    })
}

/// Calculate signed commits ratio
fn calculate_signed_commits_ratio(repo: &gix::Repository) -> f32 {
    let mut total_commits = 0u32;
    let mut signed_commits = 0u32;

    if let Ok(head) = repo.head()
        && let Some(mut head_ref) = head.try_into_referent()
        && let Ok(peeled) = head_ref.peel_to_id()
    {
        let walk = peeled
            .ancestors()
            .sorting(gix::revision::walk::Sorting::ByCommitTime(
                Default::default(),
            ))
            .all();

        if let Ok(iter) = walk {
            for commit_info in iter.take(100).flatten() {
                if let Ok(commit_obj) = repo.find_object(commit_info.id)
                    && let Ok(commit) = commit_obj.try_into_commit()
                {
                    total_commits += 1;
                    if commit.signature().is_ok() {
                        signed_commits += 1;
                    }
                }
            }
        }
    }

    if total_commits > 0 {
        signed_commits as f32 / total_commits as f32
    } else {
        0.0
    }
}

/// Calculate security score from `SecurityMetrics` components
fn calculate_security_score(metrics: &SecurityMetrics) -> f32 {
    let mut score = 0.0;

    // Security policy presence (15%)
    if metrics.security_policy {
        score += 0.15;
    }

    // Vulnerability disclosure process (10%)
    if metrics.vulnerability_disclosure {
        score += 0.10;
    }

    // Dependency scanning enabled (20%)
    if metrics.dependency_scanning {
        score += 0.20;
    }

    // No secrets detected (20%)
    // NOTE: secrets_scanning=true means secrets FOUND (bad)
    if !metrics.secrets_scanning {
        score += 0.20;
    }

    // Signed commits ratio (20%)
    score += metrics.signed_commits_ratio * 0.20;

    // No security advisories (10%)
    if metrics.security_advisories == 0 {
        score += 0.10;
    }

    // Low CVE references (5%)
    if metrics.cve_references == 0 {
        score += 0.05;
    } else if metrics.cve_references <= 2 {
        score += 0.025;
    }

    score.min(1.0)
}
