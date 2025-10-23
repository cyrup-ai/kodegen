//! Workspace analysis and dependency management.
//!
//! This module provides functionality to analyze Cargo workspaces, build dependency graphs,
//! and validate workspace structure for release operations.

mod analyzer;
mod dependency;
mod validator;

pub use analyzer::{WorkspaceInfo, SharedWorkspaceInfo, PackageInfo, WorkspaceConfig, PackageConfig, DependencySpec};
pub use dependency::{DependencyGraph, PublishOrder, PublishTier};
pub use validator::{WorkspaceValidator, ValidationResult, ValidationCheck};