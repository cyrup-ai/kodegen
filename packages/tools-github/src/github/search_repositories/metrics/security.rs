//! Security practices and vulnerability metrics collection

use super::check_file_size;
use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::helpers::{is_git_dir, is_hidden, is_vendor_dir};
use crate::github::search_repositories::types::SecurityMetrics;
use lazy_static::lazy_static;
use log::warn;
use octocrab::models::repos::dependabot::State;
use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

/// Collects security metrics
pub(crate) async fn collect_security_metrics(
    repo_path: &Path,
    signed_commits_ratio: f32,
    config: &SearchConfig,
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
) -> Option<SecurityMetrics> {
    let security_policy = repo_path.join("SECURITY.md").exists();
    let vulnerability_disclosure =
        security_policy || repo_path.join(".github/SECURITY.md").exists();

    let dependency_scanning = repo_path.join(".github/dependabot.yml").exists()
        || repo_path.join(".github/renovate.json").exists();

    // Scan for common secret patterns in code files
    let secrets_scanning = detect_secrets(repo_path, config);

    // Detect CVE references in documentation and security files
    let cve_references = count_cve_references(repo_path, config);

    // Fetch security advisories from Dependabot API
    let security_advisories = match octocrab
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
            warn!("Failed to fetch security advisories for {owner}/{repo}: {e} - defaulting to 0");
            0
        }
    };

    Some(SecurityMetrics {
        security_policy,
        vulnerability_disclosure,
        dependency_scanning,
        secrets_scanning,
        signed_commits_ratio,
        security_advisories,
        cve_references,
        license_compatibility: repo_path.join("LICENSE").exists()
            || repo_path.join("LICENSE.md").exists(),
    })
}

/// Counts CVE references in documentation and security files
fn count_cve_references(repo_path: &Path, config: &SearchConfig) -> u32 {
    lazy_static! {
        static ref CVE_RE: Result<Regex, regex::Error> = Regex::new(r"CVE-\d{4}-\d{4,}");
    }

    // Validate regex compiled successfully
    let cve_re = match CVE_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return 0, // Return 0 if regex compilation fails
    };

    let mut cve_count = 0u32;
    let check_files = [
        "SECURITY.md",
        "CHANGELOG.md",
        "HISTORY.md",
        "README.md",
        ".github/SECURITY.md",
    ];

    for file_name in &check_files {
        let file_path = repo_path.join(file_name);

        // Check file size before reading
        if let Err(e) = check_file_size(&file_path, config.max_file_size) {
            warn!("Security file skipped: {e}");
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(&file_path) {
            cve_count += cve_re.find_iter(&content).count() as u32;
        }
    }

    // Also check docs directory
    let docs_dir = repo_path.join("docs");
    if docs_dir.exists() {
        for entry in WalkDir::new(&docs_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if entry.file_type().is_file() {
                // Check file size before reading
                if let Err(e) = check_file_size(entry.path(), config.max_file_size) {
                    warn!("Docs file skipped: {e}");
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    cve_count += cve_re.find_iter(&content).count() as u32;
                }
            }
        }
    }

    cve_count
}

/// Detects potential secrets in code files
fn detect_secrets(repo_path: &Path, config: &SearchConfig) -> bool {
    lazy_static! {
        static ref API_KEY_RE: Result<Regex, regex::Error> =
            Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"][a-zA-Z0-9_-]{20,}['"]"#);
        static ref AWS_KEY_RE: Result<Regex, regex::Error> =
            Regex::new(r"(?i)(aws_access_key_id|aws_secret_access_key)\s*[:=]");
        static ref PRIVATE_KEY_RE: Result<Regex, regex::Error> =
            Regex::new(r"-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----");
        static ref PASSWORD_RE: Result<Regex, regex::Error> =
            Regex::new(r#"(?i)(password|passwd|pwd)\s*[:=]\s*['"][^'"]{8,}['"]"#);
        static ref TOKEN_RE: Result<Regex, regex::Error> =
            Regex::new(r#"(?i)(token|secret|auth)\s*[:=]\s*['"][a-zA-Z0-9_-]{20,}['"]"#);
    }

    // Validate all regexes compiled successfully
    let api_key_re = match API_KEY_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return false,
    };
    let aws_key_re = match AWS_KEY_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return false,
    };
    let private_key_re = match PRIVATE_KEY_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return false,
    };
    let password_re = match PASSWORD_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return false,
    };
    let token_re = match TOKEN_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return false,
    };

    for entry in WalkDir::new(repo_path)
        .max_depth(3)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_git_dir(e) && !is_vendor_dir(e))
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Check file size before reading
        if let Err(e) = check_file_size(path, config.max_file_size) {
            warn!("Secret scanning file skipped: {e}");
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(path)
            && (api_key_re.is_match(&content)
                || aws_key_re.is_match(&content)
                || private_key_re.is_match(&content)
                || password_re.is_match(&content)
                || token_re.is_match(&content))
        {
            return true;
        }
    }

    false
}
