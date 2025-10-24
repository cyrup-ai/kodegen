//! Documentation completeness metrics collection

use crate::github::search_repositories::types::{DocumentationMetrics, WikiInfo};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tempfile::TempDir;
use walkdir::WalkDir;

/// Collects documentation metrics
pub(crate) async fn collect_documentation_metrics(
    repo_path: &Path,
    wiki_info: WikiInfo,
) -> Option<DocumentationMetrics> {
    let docs_dir = repo_path.join("docs");
    let has_docs_folder = docs_dir.exists();

    let mut docs_files_count = 0u32;
    if has_docs_folder {
        for entry in WalkDir::new(&docs_dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension()
                    && ["md", "rst", "txt", "adoc"].contains(&ext.to_str().unwrap_or(""))
                {
                    docs_files_count += 1;
                }
            }
        }
    }

    let api_docs_generated = repo_path.join("target/doc").exists()
        || repo_path.join("docs/api").exists()
        || repo_path.join("build/docs").exists();

    let changelog_exists = repo_path.join("CHANGELOG.md").exists()
        || repo_path.join("CHANGELOG").exists()
        || repo_path.join("HISTORY.md").exists();

    let contributing_guide =
        repo_path.join("CONTRIBUTING.md").exists() || repo_path.join("CONTRIBUTING").exists();

    let code_of_conduct = repo_path.join("CODE_OF_CONDUCT.md").exists();

    let issue_templates = repo_path.join(".github/ISSUE_TEMPLATE").exists();
    let pr_templates = repo_path.join(".github/PULL_REQUEST_TEMPLATE.md").exists()
        || repo_path.join(".github/pull_request_template.md").exists();

    Some(DocumentationMetrics {
        has_docs_folder,
        docs_files_count,
        api_docs_generated,
        changelog_exists,
        contributing_guide,
        code_of_conduct,
        issue_templates,
        pr_templates,
        wiki_pages: count_wiki_pages(wiki_info).await,
    })
}

/// Count wiki pages by cloning wiki repository
async fn count_wiki_pages(wiki_info: WikiInfo) -> u32 {
    // Early return if wiki is not enabled
    if !wiki_info.has_wiki {
        return 0;
    }

    // Construct wiki URL: replace .git with .wiki.git
    let wiki_url = if wiki_info.clone_url.ends_with(".git") {
        wiki_info.clone_url.replace(".git", ".wiki.git")
    } else {
        format!("{}.wiki.git", wiki_info.clone_url)
    };

    // Parse wiki URL
    let parsed_url = match gix::url::parse(wiki_url.as_str().into()) {
        Ok(url) => url,
        Err(_) => return 0, // Invalid URL format
    };

    // Create temporary directory for wiki clone
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(_) => return 0, // Cannot create temp directory
    };

    let wiki_path = temp_dir.path().to_path_buf();

    // Clone wiki repository using spawn_blocking for sync gix operations
    let clone_result = tokio::task::spawn_blocking(move || {
        let mut prep = gix::prepare_clone(parsed_url, &wiki_path)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let (checkout, _) = prep
            .fetch_then_checkout(gix::progress::Discard, &AtomicBool::new(false))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(checkout.persist())
    })
    .await;

    // Handle clone result
    match clone_result {
        Ok(Ok(_repo)) => {
            // Count markdown files in wiki
            count_markdown_files(temp_dir.path())
        }
        _ => 0, // Clone failed or task panicked
    }
}

/// Count markdown files in a directory, excluding .git folder
fn count_markdown_files(path: &Path) -> u32 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(std::result::Result::ok) // Skip errors
        .filter(|e| e.file_type().is_file()) // Only files
        .filter(|e| {
            // Exclude .git directory
            !e.path().components().any(|c| c.as_os_str() == ".git")
        })
        .filter(|e| {
            // Only markdown files (.md extension)
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .count() as u32
}
