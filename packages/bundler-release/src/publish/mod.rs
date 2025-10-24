//! Package publishing operations for Cargo workspaces.
//!
//! This module provides comprehensive publishing capabilities including
//! dependency-ordered publishing, retry logic, rate limiting, and rollback support.

mod cargo_ops;
mod publisher;

pub use cargo_ops::{CargoPublisher, PublishConfig, PublishResult, YankResult};
pub use publisher::{
    PublishProgress, Publisher, PublisherConfig, PublishingResult, RollbackResult,
};

use crate::error::Result;
use crate::workspace::WorkspaceInfo;
use semver::Version;

/// Create a publisher for the current workspace
pub fn create_publisher(workspace: &WorkspaceInfo) -> Result<Publisher> {
    use std::sync::Arc;
    Publisher::new(Arc::new(workspace.clone()))
}

/// Create a publisher with custom configuration
pub fn create_publisher_with_config(
    workspace: &WorkspaceInfo,
    config: PublisherConfig,
) -> Result<Publisher> {
    use std::sync::Arc;
    Publisher::with_config(Arc::new(workspace.clone()), config)
}

/// Quick check if a package version is already published
pub async fn is_package_published(package_name: &str, version: &Version) -> Result<bool> {
    let publisher = CargoPublisher::new();
    publisher.is_package_published(package_name, version).await
}

/// Quick yank of a published package
pub async fn yank_package(package_name: &str, version: &Version) -> Result<YankResult> {
    let publisher = CargoPublisher::new();
    let config = PublishConfig::default();
    publisher.yank_package(package_name, version, &config).await
}

/// Get published versions of a package
pub async fn get_published_versions(package_name: &str) -> Result<Vec<Version>> {
    let publisher = CargoPublisher::new();
    publisher.get_published_versions(package_name).await
}
