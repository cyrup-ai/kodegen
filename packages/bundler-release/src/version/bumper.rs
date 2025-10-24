//! Version bumping logic using semantic versioning.
//!
//! This module provides type-safe version bump operations with proper semver compliance.

use crate::error::{Result, VersionError};
use semver::{BuildMetadata, Prerelease, Version};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Type of version bump to perform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionBump {
    /// Bump major version (breaking changes)
    Major,
    /// Bump minor version (new features)
    Minor,
    /// Bump patch version (bug fixes)
    Patch,
    /// Bump to specific version
    Exact(Version),
}

/// Version bumper with semver compliance
#[derive(Debug, Clone)]
pub struct VersionBumper {
    /// Current version being managed
    current_version: Version,
}

impl VersionBumper {
    /// Create a new version bumper with the given current version
    pub fn new(current_version: &str) -> Result<Self> {
        let version =
            Version::from_str(current_version).map_err(|e| VersionError::ParseFailed {
                version: current_version.to_string(),
                source: e,
            })?;

        Ok(Self {
            current_version: version,
        })
    }

    /// Create a version bumper from an existing semver Version
    pub fn from_version(version: Version) -> Self {
        Self {
            current_version: version,
        }
    }

    /// Perform the specified version bump
    pub fn bump(&self, bump_type: VersionBump) -> Result<Version> {
        match bump_type {
            VersionBump::Major => self.bump_major(),
            VersionBump::Minor => self.bump_minor(),
            VersionBump::Patch => self.bump_patch(),
            VersionBump::Exact(version) => self.bump_exact(version),
        }
    }

    /// Bump major version (X.y.z -> (X+1).0.0)
    fn bump_major(&self) -> Result<Version> {
        let mut new_version = self.current_version.clone();
        new_version.major += 1;
        new_version.minor = 0;
        new_version.patch = 0;
        new_version.pre = Prerelease::EMPTY;
        new_version.build = BuildMetadata::EMPTY;
        Ok(new_version)
    }

    /// Bump minor version (x.Y.z -> x.(Y+1).0)
    fn bump_minor(&self) -> Result<Version> {
        let mut new_version = self.current_version.clone();
        new_version.minor += 1;
        new_version.patch = 0;
        new_version.pre = Prerelease::EMPTY;
        new_version.build = BuildMetadata::EMPTY;
        Ok(new_version)
    }

    /// Bump patch version (x.y.Z -> x.y.(Z+1))
    fn bump_patch(&self) -> Result<Version> {
        let mut new_version = self.current_version.clone();
        new_version.patch += 1;
        new_version.pre = Prerelease::EMPTY;
        new_version.build = BuildMetadata::EMPTY;
        Ok(new_version)
    }

    /// Set exact version
    fn bump_exact(&self, version: Version) -> Result<Version> {
        self.validate_version_progression(&version)?;
        Ok(version)
    }

    /// Validate that the new version is a proper progression from current
    fn validate_version_progression(&self, new_version: &Version) -> Result<()> {
        if new_version <= &self.current_version {
            return Err(VersionError::InvalidVersion {
                version: new_version.to_string(),
                reason: format!(
                    "New version '{}' must be greater than current version '{}'",
                    new_version, self.current_version
                ),
            }
            .into());
        }

        // Check for valid semantic version progression
        let major_increased = new_version.major > self.current_version.major;
        let minor_increased = new_version.minor > self.current_version.minor;
        let patch_increased = new_version.patch > self.current_version.patch;

        // Validate progression rules
        if major_increased {
            // Major bump: minor and patch should be 0 for clean versioning
            if new_version.minor != 0 || new_version.patch != 0 {
                return Err(VersionError::InvalidVersion {
                    version: new_version.to_string(),
                    reason: "Major version bump should reset minor and patch to 0".to_string(),
                }
                .into());
            }
        } else if minor_increased {
            // Minor bump: patch should be 0 for clean versioning
            if new_version.patch != 0 {
                return Err(VersionError::InvalidVersion {
                    version: new_version.to_string(),
                    reason: "Minor version bump should reset patch to 0".to_string(),
                }
                .into());
            }
        } else if !patch_increased {
            return Err(VersionError::InvalidVersion {
                version: new_version.to_string(),
                reason: "Version must increment at least one component".to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Get the current version
    pub fn current_version(&self) -> &Version {
        &self.current_version
    }

    /// Check if a version string is valid semver
    pub fn validate_version_string(version: &str) -> Result<Version> {
        Version::from_str(version).map_err(|e| {
            VersionError::ParseFailed {
                version: version.to_string(),
                source: e,
            }
            .into()
        })
    }

    /// Calculate version bump type needed to reach target version
    pub fn calculate_bump_type(&self, target_version: &Version) -> Result<VersionBump> {
        if target_version <= &self.current_version {
            return Err(VersionError::InvalidVersion {
                version: target_version.to_string(),
                reason: "Target version must be greater than current version".to_string(),
            }
            .into());
        }

        // Check which component changed
        if target_version.major > self.current_version.major {
            // Validate it's a proper major bump
            if target_version.minor == 0 && target_version.patch == 0 {
                Ok(VersionBump::Major)
            } else {
                Ok(VersionBump::Exact(target_version.clone()))
            }
        } else if target_version.minor > self.current_version.minor {
            // Validate it's a proper minor bump
            if target_version.patch == 0 {
                Ok(VersionBump::Minor)
            } else {
                Ok(VersionBump::Exact(target_version.clone()))
            }
        } else if target_version.patch > self.current_version.patch {
            Ok(VersionBump::Patch)
        } else {
            Ok(VersionBump::Exact(target_version.clone()))
        }
    }

    /// Preview what the next version would be for each bump type
    pub fn preview_bumps(&self) -> Result<BumpPreview> {
        Ok(BumpPreview {
            current: self.current_version.clone(),
            major: self.bump_major()?,
            minor: self.bump_minor()?,
            patch: self.bump_patch()?,
        })
    }

    /// Check if version has pre-release or build metadata
    pub fn has_prerelease_or_build(&self) -> bool {
        !self.current_version.pre.is_empty() || !self.current_version.build.is_empty()
    }

    /// Get version components as tuple (major, minor, patch)
    pub fn version_components(&self) -> (u64, u64, u64) {
        (
            self.current_version.major,
            self.current_version.minor,
            self.current_version.patch,
        )
    }

    /// Create a release version (removes pre-release and build metadata)
    pub fn to_release_version(&self) -> Version {
        let mut release_version = self.current_version.clone();
        release_version.pre = Prerelease::EMPTY;
        release_version.build = BuildMetadata::EMPTY;
        release_version
    }
}

/// Preview of all possible version bumps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BumpPreview {
    /// Current version
    pub current: Version,
    /// Version after major bump
    pub major: Version,
    /// Version after minor bump
    pub minor: Version,
    /// Version after patch bump
    pub patch: Version,
}

impl BumpPreview {
    /// Get version for specific bump type
    pub fn get_version<'a>(&'a self, bump_type: &'a VersionBump) -> Option<&'a Version> {
        match bump_type {
            VersionBump::Major => Some(&self.major),
            VersionBump::Minor => Some(&self.minor),
            VersionBump::Patch => Some(&self.patch),
            VersionBump::Exact(version) => Some(version),
        }
    }

    /// Format preview as human-readable string
    pub fn format_preview(&self) -> String {
        format!(
            "Current: {} | Major: {} | Minor: {} | Patch: {}",
            self.current, self.major, self.minor, self.patch
        )
    }
}

impl FromStr for VersionBump {
    type Err = VersionError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "major" => Ok(VersionBump::Major),
            "minor" => Ok(VersionBump::Minor),
            "patch" => Ok(VersionBump::Patch),
            version_str => {
                let version =
                    Version::from_str(version_str).map_err(|e| VersionError::ParseFailed {
                        version: version_str.to_string(),
                        source: e,
                    })?;
                Ok(VersionBump::Exact(version))
            }
        }
    }
}

impl std::fmt::Display for VersionBump {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionBump::Major => write!(f, "major"),
            VersionBump::Minor => write!(f, "minor"),
            VersionBump::Patch => write!(f, "patch"),
            VersionBump::Exact(version) => write!(f, "{}", version),
        }
    }
}

#[cfg(test)]
mod tests {
    // Note: Tests will be written in ./tests/ directory by another agent
    // This is just to ensure the module compiles correctly
}
