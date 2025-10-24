//! README quality metrics collection

use super::check_file_size;
use crate::github::search_repositories::config::SearchConfig;
use crate::github::search_repositories::types::ReadmeMetrics;
use lazy_static::lazy_static;
use log::warn;
use regex::Regex;
use std::path::Path;

/// Collects README quality metrics
pub(crate) async fn collect_readme_metrics(
    repo_path: &Path,
    config: &SearchConfig,
) -> Option<ReadmeMetrics> {
    lazy_static! {
        static ref HEADER_RE: Result<Regex, regex::Error> = Regex::new(r"(?m)^#{1,6}\s+(.+)$");
        static ref CODE_BLOCK_RE: Result<Regex, regex::Error> = Regex::new(r"```[\s\S]*?```");
        static ref LINK_RE: Result<Regex, regex::Error> = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)");
        static ref IMAGE_RE: Result<Regex, regex::Error> = Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)");
        static ref BADGE_RE: Result<Regex, regex::Error> =
            Regex::new(r"!\[([^\]]*)\]\(https://img\.shields\.io/[^)]+\)");
    }

    // Validate all regexes compiled successfully
    let header_re = match HEADER_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return None,
    };
    let code_block_re = match CODE_BLOCK_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return None,
    };
    let link_re = match LINK_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return None,
    };
    let image_re = match IMAGE_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return None,
    };
    let badge_re = match BADGE_RE.as_ref() {
        Ok(re) => re,
        Err(_) => return None,
    };

    // Find README file (case-insensitive)
    let readme_variants = [
        "README.md",
        "README.txt",
        "README.rst",
        "README",
        "Readme.md",
        "readme.md",
    ];

    let mut readme_path = None;
    for variant in &readme_variants {
        let path = repo_path.join(variant);
        if path.exists() {
            readme_path = Some(path);
            break;
        }
    }

    let readme_path = readme_path?;

    // Check file size before reading
    if let Err(e) = check_file_size(&readme_path, config.max_file_size) {
        warn!("README file skipped: {e}");
        return None;
    }

    let content = std::fs::read_to_string(&readme_path).ok()?;

    let length = content.len() as u32;
    let sections_count = header_re.find_iter(&content).count() as u32;
    let code_blocks_count = code_block_re.find_iter(&content).count() as u32;
    let links_count = link_re.find_iter(&content).count() as u32;
    let images_count = image_re.find_iter(&content).count() as u32;
    let badges_count = badge_re.find_iter(&content).count() as u32;

    let content_lower = content.to_lowercase();
    let table_of_contents =
        content_lower.contains("table of contents") || content_lower.contains("## contents");
    let installation_instructions =
        content_lower.contains("install") || content_lower.contains("setup");
    let usage_examples = content_lower.contains("usage")
        || content_lower.contains("example")
        || code_blocks_count > 0;
    let api_documentation = content_lower.contains("api") || content_lower.contains("reference");
    let license_mentioned = content_lower.contains("license")
        || content_lower.contains("mit")
        || content_lower.contains("apache");
    let contributing_guidelines = content_lower.contains("contribut");

    // Calculate quality score
    let mut quality_score: f32 = 0.0;
    if length > 1000 {
        quality_score += 20.0;
    } else if length > 500 {
        quality_score += 10.0;
    }
    if sections_count >= 5 {
        quality_score += 20.0;
    } else if sections_count >= 3 {
        quality_score += 10.0;
    }
    if code_blocks_count > 0 {
        quality_score += 15.0;
    }
    if links_count > 3 {
        quality_score += 10.0;
    }
    if images_count > 0 {
        quality_score += 5.0;
    }
    if badges_count > 0 {
        quality_score += 5.0;
    }
    if table_of_contents {
        quality_score += 5.0;
    }
    if installation_instructions {
        quality_score += 10.0;
    }
    if usage_examples {
        quality_score += 10.0;
    }
    if api_documentation {
        quality_score += 5.0;
    }
    if license_mentioned {
        quality_score += 5.0;
    }
    if contributing_guidelines {
        quality_score += 5.0;
    }

    Some(ReadmeMetrics {
        exists: true,
        length,
        sections_count,
        code_blocks_count,
        links_count,
        images_count,
        badges_count,
        table_of_contents,
        installation_instructions,
        usage_examples,
        api_documentation,
        license_mentioned,
        contributing_guidelines,
        quality_score: quality_score.min(100.0),
    })
}
