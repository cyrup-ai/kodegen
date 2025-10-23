//! GitHub Release creation and management
//!
//! Uses octocrab's releases API: client.repos(owner, `repo).releases()`

use octocrab::models::repos::Release;
use octocrab::Octocrab;
use std::sync::Arc;

/// Options for creating a GitHub release
#[derive(Debug, Clone, Default)]
pub struct CreateReleaseOptions {
    /// Release tag name (e.g., "v1.0.0")
    pub tag_name: String,
    /// Target commit SHA or branch (defaults to main branch)
    pub target_commitish: Option<String>,
    /// Release name/title
    pub name: Option<String>,
    /// Release notes body (markdown)
    pub body: Option<String>,
    /// Whether this is a draft release
    pub draft: bool,
    /// Whether this is a pre-release
    pub prerelease: bool,
}

/// Result of release creation
#[derive(Debug, Clone)]
pub struct ReleaseResult {
    /// Release ID
    pub id: u64,
    /// Release tag name
    pub tag_name: String,
    /// Release name
    pub name: String,
    /// Release HTML URL
    pub html_url: String,
    /// Upload URL for assets
    pub upload_url: String,
    /// Whether this is a draft
    pub draft: bool,
    /// Whether this is a prerelease
    pub prerelease: bool,
}

/// Create a GitHub release using octocrab
pub async fn create_release(
    client: Arc<Octocrab>,
    owner: &str,
    repo: &str,
    options: CreateReleaseOptions,
) -> Result<ReleaseResult, octocrab::Error> {
    let release = client
        .repos(owner, repo)
        .releases()
        .create(&options.tag_name)
        .target_commitish(options.target_commitish.as_deref().unwrap_or(""))
        .name(options.name.as_deref().unwrap_or(&options.tag_name))
        .body(options.body.as_deref().unwrap_or(""))
        .draft(options.draft)
        .prerelease(options.prerelease)
        .send()
        .await?;
    
    Ok(ReleaseResult {
        id: release.id.0,
        tag_name: release.tag_name,
        name: release.name.unwrap_or_default(),
        html_url: release.html_url.to_string(),
        upload_url: release.upload_url,
        draft: release.draft,
        prerelease: release.prerelease,
    })
}

/// Get a release by tag
pub async fn get_release_by_tag(
    client: Arc<Octocrab>,
    owner: &str,
    repo: &str,
    tag: &str,
) -> Result<Option<Release>, octocrab::Error> {
    client
        .repos(owner, repo)
        .releases()
        .get_by_tag(tag)
        .await
        .map(Some)
        .or_else(|e| {
            // Return None for 404, propagate other errors
            if matches!(e, octocrab::Error::GitHub { .. }) {
                Ok(None)
            } else {
                Err(e)
            }
        })
}

/// Delete a release
pub async fn delete_release(
    client: Arc<Octocrab>,
    owner: &str,
    repo: &str,
    release_id: u64,
) -> Result<(), octocrab::Error> {
    client
        .repos(owner, repo)
        .releases()
        .delete(release_id)
        .await
}
