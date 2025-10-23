//! Upload assets to GitHub releases
//!
//! Pattern follows `create_release.rs` - direct async functions without `spawn_task`

use bytes::Bytes;
use octocrab::{models::repos::Asset, Octocrab};
use std::sync::Arc;

/// Options for uploading a release asset
#[derive(Debug, Clone)]
pub struct UploadAssetOptions {
    /// Release ID from `create_release`
    pub release_id: u64,
    /// Asset filename (e.g., "KodegenHelper.app-macos-aarch64.zip")
    pub asset_name: String,
    /// Optional label for the asset
    pub label: Option<String>,
    /// File content as bytes
    pub content: Bytes,
    /// If true, delete existing asset with same name before upload.
    /// Default: false (safer - fails if asset exists)
    pub replace_existing: bool,
}

/// Upload an asset to a GitHub release using octocrab
///
/// Uses the `release_id` from `create_release` and uploads binary content.
/// Returns the uploaded asset with download URL.
pub async fn upload_release_asset(
    client: Arc<Octocrab>,
    owner: &str,
    repo: &str,
    options: UploadAssetOptions,
) -> Result<Asset, octocrab::Error> {
    // Step 1: If replace_existing, find and delete existing asset
    if options.replace_existing {
        // List assets for this release
        let assets_page = client
            .repos(owner, repo)
            .releases()
            .assets(options.release_id)
            .per_page(100)
            .send()
            .await?;
        
        // Find asset with matching name
        if let Some(existing) = assets_page.items.iter().find(|a| a.name == options.asset_name) {
            // Delete the existing asset
            delete_release_asset(
                client.clone(),
                owner,
                repo,
                existing.id.0,  // AssetId is a newtype wrapper around u64
            ).await?;
        }
        // If no match found, that's fine - proceed with upload
    }
    
    // Step 2: Upload the new asset
    let repos = client.repos(owner, repo);
    let releases = repos.releases();
    
    if let Some(label) = options.label {
        releases
            .upload_asset(options.release_id, &options.asset_name, options.content)
            .label(&label)
            .send()
            .await
    } else {
        releases
            .upload_asset(options.release_id, &options.asset_name, options.content)
            .send()
            .await
    }
}

/// Delete a release asset
pub async fn delete_release_asset(
    client: Arc<Octocrab>,
    owner: &str,
    repo: &str,
    asset_id: u64,
) -> Result<(), octocrab::Error> {
    client
        .repos(owner, repo)
        .release_assets()
        .delete(asset_id)
        .await
}