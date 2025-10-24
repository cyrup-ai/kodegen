//! Metrics collection orchestration

mod ci_cd;
mod code_quality;
mod dependencies;
mod documentation;
mod readme;
mod security;
mod structure;
mod tests;

pub(crate) use ci_cd::collect_ci_cd_metrics;
pub(crate) use code_quality::collect_code_quality_metrics;
pub(crate) use dependencies::collect_dependency_metrics;
pub(crate) use documentation::collect_documentation_metrics;
pub(crate) use readme::collect_readme_metrics;
pub(crate) use security::collect_security_metrics;
pub(crate) use structure::collect_structure_metrics;
pub(crate) use tests::collect_test_metrics;

use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::types::{LocalMetrics, WikiInfo};
use std::path::Path;

/// Helper function to check if a file's size is within the allowed limit
pub(crate) fn check_file_size(path: &Path, max_size: usize) -> Result<(), String> {
    let metadata =
        std::fs::metadata(path).map_err(|e| format!("Failed to get file metadata: {e}"))?;

    if metadata.len() > max_size as u64 {
        return Err(format!(
            "File too large: {} bytes (max: {} bytes)",
            metadata.len(),
            max_size
        ));
    }

    Ok(())
}

/// Context for metrics collection.
pub(crate) struct MetricsCollectionContext<'a> {
    pub signed_commits_ratio: f32,
    pub build_status: String,
    pub owner: &'a str,
    pub repo: &'a str,
    pub wiki_info: WikiInfo,
}

/// Orchestrates collection of all local metrics
pub(crate) async fn collect_local_metrics(
    repo_path: &Path,
    context: MetricsCollectionContext<'_>,
    config: &SearchConfig,
    octocrab: &octocrab::Octocrab,
) -> Option<LocalMetrics> {
    let readme_quality = collect_readme_metrics(repo_path, config).await?;
    let code_quality = collect_code_quality_metrics(repo_path, config).await?;
    let test_metrics = collect_test_metrics(repo_path, code_quality.code_lines, config).await?;
    let ci_cd_metrics = collect_ci_cd_metrics(repo_path, context.build_status, config).await?;
    let documentation_metrics = collect_documentation_metrics(repo_path, context.wiki_info).await?;
    let dependency_metrics =
        collect_dependency_metrics(repo_path, config, octocrab, context.owner, context.repo)
            .await?;
    let security_metrics = collect_security_metrics(
        repo_path,
        context.signed_commits_ratio,
        config,
        octocrab,
        context.owner,
        context.repo,
    )
    .await?;
    let structure_metrics = collect_structure_metrics(repo_path).await?;

    Some(LocalMetrics {
        readme_quality,
        code_quality,
        test_metrics,
        ci_cd_metrics,
        documentation_metrics,
        security_metrics,
        dependency_metrics,
        structure_metrics,
    })
}
