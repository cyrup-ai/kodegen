//! Dependency management and freshness metrics collection

use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::types::DependencyMetrics;
use futures::stream::{self, StreamExt};
use log::warn;
use octocrab::models::repos::dependabot::State;
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::path::Path;
use std::time::{Duration, SystemTime};
use toml::Value as TomlValue;

// API response structures
#[derive(Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    crate_data: CrateData,
}

#[derive(Deserialize)]
struct CrateData {
    max_version: String,
}

#[derive(Deserialize)]
struct NpmPackageInfo {
    #[serde(rename = "dist-tags")]
    dist_tags: DistTags,
}

#[derive(Deserialize)]
struct DistTags {
    latest: String,
}

#[derive(Deserialize)]
struct PyPIPackageInfo {
    info: PyPIInfo,
}

#[derive(Deserialize)]
struct PyPIInfo {
    version: String,
}

/// User-Agent header for registry API requests
const USER_AGENT: &str = concat!("gitgix/", env!("CARGO_PKG_VERSION"));

/// Compare two semantic versions, returns true if current < latest
fn is_outdated(current: &str, latest: &str) -> bool {
    // Clean version strings (remove ^, ~, >=, etc.)
    let clean_current = current
        .trim_start_matches(&['<', '>', '=', '^', '~', 'v'][..])
        .split_whitespace()
        .next()
        .unwrap_or(current);

    let clean_latest = latest
        .trim_start_matches('v')
        .split_whitespace()
        .next()
        .unwrap_or(latest);

    match (Version::parse(clean_current), Version::parse(clean_latest)) {
        (Ok(curr_ver), Ok(latest_ver)) => curr_ver < latest_ver,
        _ => false, // Can't determine, assume not outdated
    }
}

/// Check outdated dependencies for Cargo (Rust) projects
async fn check_cargo_outdated(dependencies: &toml::Table, client: &Client) -> u32 {
    let deps: Vec<_> = dependencies
        .iter()
        .filter_map(|(name, version_spec)| {
            let version = match version_spec {
                TomlValue::String(v) => v.clone(),
                TomlValue::Table(t) => {
                    if let Some(TomlValue::String(v)) = t.get("version") {
                        v.clone()
                    } else {
                        return None; // Skip path/git dependencies
                    }
                }
                _ => return None,
            };
            Some((name.clone(), version))
        })
        .collect();

    let client = client.clone();
    let results = stream::iter(deps)
        .map(|(name, version)| {
            let client = client.clone();
            async move {
                let url = format!("https://crates.io/api/v1/crates/{name}");
                if let Ok(Ok(response)) = tokio::time::timeout(
                    Duration::from_secs(5),
                    client.get(&url).header("User-Agent", USER_AGENT).send(),
                )
                .await
                    && let Ok(data) = response.json::<CratesIoResponse>().await
                    && is_outdated(&version, &data.crate_data.max_version)
                {
                    return 1u32;
                }
                0u32
            }
        })
        .buffer_unordered(5)
        .collect::<Vec<_>>()
        .await;

    results.iter().sum()
}

/// Check outdated dependencies for npm (JavaScript/Node) projects
async fn check_npm_outdated(
    dependencies: &serde_json::Map<String, JsonValue>,
    client: &Client,
) -> u32 {
    let deps: Vec<_> = dependencies
        .iter()
        .filter_map(|(name, version_spec)| {
            let version = version_spec.as_str()?.to_string();

            // Filter out special versions and non-semver dependencies
            if version.is_empty()
                || version == "latest"
                || version == "*"
                || version.starts_with("http://")
                || version.starts_with("https://")
                || version.starts_with("git+")
                || version.starts_with("file:")
                || version.starts_with("github:")
                || version.starts_with("gitlab:")
                || version.starts_with("bitbucket:")
            {
                return None;
            }

            Some((name.clone(), version))
        })
        .collect();

    let client = client.clone();
    let results = stream::iter(deps)
        .map(|(name, version)| {
            let client = client.clone();
            async move {
                let url = format!("https://registry.npmjs.org/{name}");
                if let Ok(Ok(response)) =
                    tokio::time::timeout(Duration::from_secs(5), client.get(&url).send()).await
                    && let Ok(data) = response.json::<NpmPackageInfo>().await
                    && is_outdated(&version, &data.dist_tags.latest)
                {
                    return 1u32;
                }
                0u32
            }
        })
        .buffer_unordered(5)
        .collect::<Vec<_>>()
        .await;

    results.iter().sum()
}

/// Check outdated dependencies for pip (Python) projects
async fn check_pypi_outdated(requirements: &[String], client: &Client) -> u32 {
    let deps: Vec<_> = requirements
        .iter()
        .filter_map(|requirement| {
            let requirement = requirement.trim();

            // Skip comments and pip options
            if requirement.starts_with('#') || requirement.starts_with('-') {
                return None;
            }

            // Try operators in order of specificity
            for op in ["==", ">=", "<=", "~=", "!=", ">", "<"] {
                if let Some(idx) = requirement.find(op) {
                    let name = requirement[..idx].trim();
                    let version_part = requirement[idx + op.len()..].trim();
                    // Handle compound specs like ">=1.0,<2.0"
                    let version = version_part.split(',').next().unwrap_or("").trim();

                    if !name.is_empty() && !version.is_empty() {
                        return Some((name.to_string(), version.to_string()));
                    }
                    return None;
                }
            }
            None // No version specifier found
        })
        .collect();

    let client = client.clone();
    let results = stream::iter(deps)
        .map(|(name, version)| {
            let client = client.clone();
            async move {
                let url = format!("https://pypi.org/pypi/{name}/json");
                if let Ok(Ok(response)) =
                    tokio::time::timeout(Duration::from_secs(5), client.get(&url).send()).await
                    && let Ok(data) = response.json::<PyPIPackageInfo>().await
                    && is_outdated(&version, &data.info.version)
                {
                    return 1u32;
                }
                0u32
            }
        })
        .buffer_unordered(5)
        .collect::<Vec<_>>()
        .await;

    results.iter().sum()
}

/// Calculate dependency freshness based on lock file modification time
/// Returns score from 0.0 (very stale) to 1.0 (very fresh)
#[inline]
fn calculate_freshness_from_lock_age(lock_file_path: &Path) -> f32 {
    // Attempt to get file metadata
    let metadata = match std::fs::metadata(lock_file_path) {
        Ok(m) => m,
        Err(_) => return 0.5, // File doesn't exist or inaccessible, neutral score
    };

    // Get modification time
    let modified = match metadata.modified() {
        Ok(time) => time,
        Err(_) => return 0.5, // Can't read modification time, neutral score
    };

    // Calculate age
    let now = SystemTime::now();
    let age = match now.duration_since(modified) {
        Ok(duration) => duration,
        Err(_) => return 0.5, // Clock skew or future timestamp, neutral score
    };

    // Convert to days
    let days = age.as_secs() / 86400;

    // Exponential decay scoring based on age thresholds
    match days {
        0..=30 => 1.0,     // < 1 month: excellent
        31..=90 => 0.9,    // 1-3 months: very good
        91..=180 => 0.8,   // 3-6 months: good
        181..=365 => 0.6,  // 6-12 months: acceptable
        366..=730 => 0.4,  // 1-2 years: aging
        731..=1095 => 0.2, // 2-3 years: stale
        _ => 0.1,          // > 3 years: very stale
    }
}

/// Helper to calculate freshness from `SystemTime` timestamp
#[inline]
fn calculate_freshness_from_timestamp(modified: SystemTime) -> f32 {
    let now = SystemTime::now();
    let age = match now.duration_since(modified) {
        Ok(duration) => duration,
        Err(_) => return 0.5,
    };

    let days = age.as_secs() / 86400;

    match days {
        0..=30 => 1.0,
        31..=90 => 0.9,
        91..=180 => 0.8,
        181..=365 => 0.6,
        366..=730 => 0.4,
        731..=1095 => 0.2,
        _ => 0.1,
    }
}

/// Get the most recent modification time from multiple possible lock files
#[inline]
fn get_most_recent_lock_file(repo_path: &Path, lock_files: &[&str]) -> Option<SystemTime> {
    lock_files
        .iter()
        .filter_map(|&filename| {
            let path = repo_path.join(filename);
            std::fs::metadata(&path).ok()?.modified().ok()
        })
        .max() // Get the most recent timestamp
}

/// Calculate dependency freshness based on lock file ages
/// Returns score from 0.0 to 1.0
fn calculate_dependency_freshness(repo_path: &Path, package_managers: &[String]) -> f32 {
    if package_managers.is_empty() {
        return 0.0; // No dependency management
    }

    let mut first_score: Option<f32> = None;
    let mut additional_scores: Vec<f32> = Vec::new();

    // Check each detected package manager's lock files
    for pm in package_managers {
        let score = match pm.as_str() {
            "Cargo" => {
                let lock_path = repo_path.join("Cargo.lock");
                if lock_path.exists() {
                    calculate_freshness_from_lock_age(&lock_path)
                } else {
                    continue;
                }
            }
            "npm" => {
                let lock_files = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml"];
                if let Some(most_recent) = get_most_recent_lock_file(repo_path, &lock_files) {
                    calculate_freshness_from_timestamp(most_recent)
                } else {
                    0.4 // Has package.json but no lock file
                }
            }
            "pip" => {
                let lock_files = ["Pipfile.lock", "poetry.lock"];
                if let Some(most_recent) = get_most_recent_lock_file(repo_path, &lock_files) {
                    calculate_freshness_from_timestamp(most_recent)
                } else {
                    // Check requirements.txt as fallback
                    let req_path = repo_path.join("requirements.txt");
                    if req_path.exists() {
                        calculate_freshness_from_lock_age(&req_path)
                    } else {
                        0.4
                    }
                }
            }
            "Go modules" => {
                let lock_path = repo_path.join("go.sum");
                if lock_path.exists() {
                    calculate_freshness_from_lock_age(&lock_path)
                } else {
                    0.4
                }
            }
            _ => continue,
        };

        match first_score {
            None => first_score = Some(score),
            Some(_) => additional_scores.push(score),
        }
    }

    // Calculate final score
    match (first_score, additional_scores.is_empty()) {
        (None, _) => 0.4,             // No scores collected
        (Some(score), true) => score, // Single PM: zero allocation path
        (Some(first), false) => {
            // Multiple PMs: average all scores
            let sum = first + additional_scores.iter().sum::<f32>();
            sum / (1 + additional_scores.len()) as f32
        }
    }
}

/// Collects dependency metrics
pub(crate) async fn collect_dependency_metrics(
    repo_path: &Path,
    config: &SearchConfig,
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
) -> Option<DependencyMetrics> {
    let mut direct_dependencies = 0u32;
    let mut dev_dependencies = 0u32;
    let mut package_managers = Vec::new();
    let mut lock_files_present = false;

    // Store dependencies for version checking
    let mut cargo_deps: Option<toml::Table> = None;
    let mut npm_deps: Option<serde_json::Map<String, JsonValue>> = None;
    let mut python_reqs: Vec<String> = Vec::new();

    // Rust - Cargo.toml
    if let Ok(content) = std::fs::read_to_string(repo_path.join("Cargo.toml")) {
        package_managers.push("Cargo".to_string());
        if let Ok(toml_value) = content.parse::<TomlValue>() {
            if let Some(deps) = toml_value.get("dependencies").and_then(|v| v.as_table()) {
                direct_dependencies += deps.len() as u32;
                cargo_deps = Some(deps.clone());
            }
            if let Some(dev_deps) = toml_value
                .get("dev-dependencies")
                .and_then(|v| v.as_table())
            {
                dev_dependencies += dev_deps.len() as u32;
            }
        }
        if repo_path.join("Cargo.lock").exists() {
            lock_files_present = true;
        }
    }

    // JavaScript/Node - package.json
    if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json")) {
        package_managers.push("npm".to_string());
        if let Ok(json_value) = serde_json::from_str::<JsonValue>(&content) {
            if let Some(deps) = json_value.get("dependencies").and_then(|v| v.as_object()) {
                direct_dependencies += deps.len() as u32;
                npm_deps = Some(deps.clone());
            }
            if let Some(dev_deps) = json_value
                .get("devDependencies")
                .and_then(|v| v.as_object())
            {
                dev_dependencies += dev_deps.len() as u32;
            }
        }
        if repo_path.join("package-lock.json").exists() || repo_path.join("yarn.lock").exists() {
            lock_files_present = true;
        }
    }

    // Python - requirements.txt or pyproject.toml
    if repo_path.join("requirements.txt").exists() {
        package_managers.push("pip".to_string());
        if let Ok(content) = std::fs::read_to_string(repo_path.join("requirements.txt")) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    direct_dependencies += 1;
                    python_reqs.push(trimmed.to_string());
                }
            }
        }
    }
    if repo_path.join("Pipfile").exists() || repo_path.join("poetry.lock").exists() {
        lock_files_present = true;
    }

    // Go - go.mod
    if repo_path.join("go.mod").exists() {
        package_managers.push("Go modules".to_string());
        if let Ok(content) = std::fs::read_to_string(repo_path.join("go.mod")) {
            let mut in_require_block = false;
            for line in content.lines() {
                let trimmed = line.trim();

                // Check for start of require block
                if trimmed.starts_with("require (") || trimmed == "require (" {
                    in_require_block = true;
                    continue;
                }

                // Check for end of require block
                if in_require_block && trimmed.starts_with(')') {
                    in_require_block = false;
                    continue;
                }

                // Count dependencies inside require block
                if in_require_block && !trimmed.is_empty() && !trimmed.starts_with("//") {
                    direct_dependencies += 1;
                }

                // Handle single-line require statements
                if !in_require_block && trimmed.starts_with("require ") && !trimmed.contains('(') {
                    direct_dependencies += 1;
                }
            }
        }
        if repo_path.join("go.sum").exists() {
            lock_files_present = true;
        }
    }

    let total_dependencies = direct_dependencies + dev_dependencies;

    // Calculate freshness score based on lock file modification timestamps
    let dependency_freshness_score = calculate_dependency_freshness(repo_path, &package_managers);

    // Fetch vulnerable dependencies from Dependabot API
    let vulnerable_dependencies = match octocrab
        .repos(owner, repo)
        .dependabot()
        .per_page(config.api_page_size)
        .get_alerts()
        .await
    {
        Ok(page) => page
            .items
            .iter()
            .filter(|alert| matches!(alert.state, State::Open))
            .count() as u32,
        Err(e) => {
            warn!("Failed to fetch Dependabot alerts for {owner}/{repo}: {e} - defaulting to 0");
            0
        }
    };

    // Check for outdated dependencies using registry APIs
    // Performance guard: Only check if total deps <= 50
    let outdated_dependencies = if total_dependencies > 50 {
        0 // Too many to check efficiently
    } else {
        // Create HTTP client with timeout
        let client = match Client::builder().timeout(Duration::from_secs(5)).build() {
            Ok(c) => c,
            Err(_) => {
                return Some(DependencyMetrics {
                    total_dependencies,
                    direct_dependencies,
                    dev_dependencies,
                    outdated_dependencies: 0,
                    vulnerable_dependencies,
                    dependency_freshness_score,
                    package_managers,
                    lock_files_present,
                });
            }
        };

        // Set overall timeout of 10 seconds for all checks
        let check_future = async {
            // Run all registry checks in parallel for fair timeout distribution
            let (cargo_count, npm_count, pypi_count) = tokio::join!(
                async {
                    if let Some(ref deps) = cargo_deps {
                        check_cargo_outdated(deps, &client).await
                    } else {
                        0u32
                    }
                },
                async {
                    if let Some(ref deps) = npm_deps {
                        check_npm_outdated(deps, &client).await
                    } else {
                        0u32
                    }
                },
                async {
                    if python_reqs.is_empty() {
                        0u32
                    } else {
                        check_pypi_outdated(&python_reqs, &client).await
                    }
                }
            );

            cargo_count + npm_count + pypi_count
        };

        if let Ok(count) = tokio::time::timeout(Duration::from_secs(10), check_future).await {
            count
        } else {
            warn!("Timeout checking outdated dependencies for {owner}/{repo}");
            0
        }
    };

    Some(DependencyMetrics {
        total_dependencies,
        direct_dependencies,
        dev_dependencies,
        outdated_dependencies,
        vulnerable_dependencies,
        dependency_freshness_score,
        package_managers,
        lock_files_present,
    })
}
