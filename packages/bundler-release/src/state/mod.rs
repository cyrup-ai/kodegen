//! State management for release operations.
//!
//! This module provides comprehensive state tracking and persistence for release operations,
//! enabling resume capabilities and ensuring atomic operations.

mod manager;
mod release_state;

pub use manager::{
    FileInfo, LoadStateResult, SaveStateResult, StateConfig, StateFileInfo, StateManager,
};
pub use release_state::{
    FileBackup, GitCommitInfo, GitPushInfo, GitState, GitTagInfo, PublishPackageInfo, PublishState,
    ReleaseCheckpoint, ReleaseConfig, ReleaseError, ReleasePhase, ReleaseState,
    STATE_FORMAT_VERSION, VersionState, VersionUpdateInfo,
};

use crate::error::Result;
use std::path::Path;

/// Create a state manager for the default state file location
pub fn create_state_manager() -> Result<StateManager> {
    StateManager::new(".cyrup_release_state.json")
}

/// Create a state manager for a custom state file location
pub fn create_state_manager_at<P: AsRef<Path>>(path: P) -> Result<StateManager> {
    StateManager::new(path)
}

/// Create a state manager with custom configuration
pub fn create_state_manager_with_config<P: AsRef<Path>>(
    path: P,
    config: StateConfig,
) -> Result<StateManager> {
    StateManager::with_config(path, config)
}

/// Quick check if release state exists at default location
pub fn has_active_release() -> bool {
    std::path::Path::new(".cyrup_release_state.json").exists()
}

/// Quick check if release state exists at custom location
pub fn has_active_release_at<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

/// Load release state from default location
pub async fn load_release_state() -> Result<LoadStateResult> {
    let mut manager = create_state_manager()?;
    manager.load_state().await
}

/// Load release state from custom location
pub async fn load_release_state_from<P: AsRef<Path>>(path: P) -> Result<LoadStateResult> {
    let mut manager = StateManager::new(path)?;
    manager.load_state().await
}

/// Save release state to default location
pub async fn save_release_state(state: &ReleaseState) -> Result<SaveStateResult> {
    let mut manager = create_state_manager()?;
    manager.save_state(state).await
}

/// Save release state to custom location
pub async fn save_release_state_to<P: AsRef<Path>>(
    path: P,
    state: &ReleaseState,
) -> Result<SaveStateResult> {
    let mut manager = StateManager::new(path)?;
    manager.save_state(state).await
}

/// Cleanup release state at default location
pub fn cleanup_release_state() -> Result<()> {
    let manager = create_state_manager()?;
    manager.cleanup_state()
}

/// Cleanup release state at custom location  
pub fn cleanup_release_state_at<P: AsRef<Path>>(path: P) -> Result<()> {
    let manager = StateManager::new(path)?;
    manager.cleanup_state()
}
