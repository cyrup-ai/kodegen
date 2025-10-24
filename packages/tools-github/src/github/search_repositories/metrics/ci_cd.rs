//! CI/CD pipeline and automation metrics collection

use super::check_file_size;
use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::types::CiCdMetrics;
use log::warn;
use std::path::Path;
use walkdir::WalkDir;

/// Collects CI/CD metrics
pub(crate) async fn collect_ci_cd_metrics(
    repo_path: &Path,
    build_status: String,
    config: &SearchConfig,
) -> Option<CiCdMetrics> {
    let mut ci_providers = Vec::new();
    let mut workflow_files = 0u32;
    let mut has_ci = false;
    let mut test_automation = false;
    let mut deployment_automation = false;
    let mut code_quality_checks = false;
    let mut security_scanning = false;
    let mut dependency_updates = false;
    let mut release_automation = false;

    // Check for GitHub Actions
    let gh_actions = repo_path.join(".github/workflows");
    if gh_actions.exists() {
        has_ci = true;
        ci_providers.push("GitHub Actions".to_string());
        if let Ok(entries) = std::fs::read_dir(&gh_actions) {
            workflow_files = entries.count() as u32;
        }

        // Scan workflow files
        for entry in WalkDir::new(&gh_actions)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if entry.file_type().is_file() {
                // Check file size before reading
                if let Err(e) = check_file_size(entry.path(), config.max_file_size) {
                    warn!("Workflow file skipped: {e}");
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if content.contains("test")
                        || content.contains("cargo test")
                        || content.contains("npm test")
                    {
                        test_automation = true;
                    }
                    if content.contains("deploy") {
                        deployment_automation = true;
                    }
                    if content.contains("lint") || content.contains("clippy") {
                        code_quality_checks = true;
                    }
                    if content.contains("security") || content.contains("audit") {
                        security_scanning = true;
                    }
                    if content.contains("dependabot") || content.contains("renovate") {
                        dependency_updates = true;
                    }
                    if content.contains("release") {
                        release_automation = true;
                    }
                }
            }
        }
    }

    // Check for other CI systems
    if repo_path.join(".travis.yml").exists() {
        has_ci = true;
        ci_providers.push("Travis CI".to_string());
    }
    if repo_path.join(".circleci").exists() {
        has_ci = true;
        ci_providers.push("CircleCI".to_string());
    }
    if repo_path.join("Jenkinsfile").exists() {
        has_ci = true;
        ci_providers.push("Jenkins".to_string());
    }
    if repo_path.join(".gitlab-ci.yml").exists() {
        has_ci = true;
        ci_providers.push("GitLab CI".to_string());
    }

    Some(CiCdMetrics {
        has_ci,
        ci_providers,
        workflow_files,
        build_status,
        test_automation,
        deployment_automation,
        code_quality_checks,
        security_scanning,
        dependency_updates,
        release_automation,
    })
}
