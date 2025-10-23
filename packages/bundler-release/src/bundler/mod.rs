//! Multi-platform binary bundler for creating native installers.
//!
//! This module provides bundling capabilities for Linux (.deb, .rpm, AppImage),
//! macOS (.app, .dmg), and Windows (.msi, .exe via NSIS).
//!
//! # Configuration
//!
//! Bundling is configured via `[package.metadata.bundle]` in `Cargo.toml`:
//!
//! ```toml
//! [package.metadata.bundle]
//! identifier = "com.example.app"
//! publisher = "Example Inc."
//! icon = ["assets/icon.png"]
//!
//! [package.metadata.bundle.linux.deb]
//! depends = ["libc6"]
//! ```
//!
//! # Supported Formats
//!
//! | Platform | Formats | Notes |
//! |----------|---------|-------|
//! | Linux | .deb, .rpm, AppImage | All major distributions |
//! | macOS | .app, .dmg | Code signing optional |
//! | Windows | .msi, .exe (NSIS) | Authenticode signing optional |
//!
//! # Integration
//!
//! The bundler integrates with the release workflow in `kodegen_bundler_release`:
//!
//! ```no_run
//! use kodegen_bundler_release::bundler::{Bundler, SettingsBuilder};
//!
//! let settings = SettingsBuilder::new()
//!     .project_out_directory("target/release")
//!     .build()?;
//!
//! let bundler = Bundler::new(settings)?;
//! let artifacts = bundler.bundle()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Code Signing
//!
//! See [`kodegen_sign`](../../sign/index.html) for code signing setup:
//! - macOS: Automated Developer ID provisioning
//! - Windows: Authenticode via osslsigncode
//! - Linux: GPG signing guidance

#![warn(missing_docs)]

mod builder;
mod error;
mod patch;
mod platform;
mod resources;
mod settings;
mod utils;

// Public re-exports
pub use builder::Bundler;
pub use error::{Error, Result};
pub use patch::patch_binary;
pub use platform::PackageType;
pub use settings::{
    // Main configuration types
    Settings,
    BundleSettings,
    PackageSettings,
    SettingsBuilder,
    // Binary configuration
    BundleBinary,
    // Architecture detection
    Arch,
    // Platform-specific settings
    DebianSettings,
    RpmSettings,
    AppImageSettings,
    MacOsSettings,
    DmgSettings,
    WindowsSettings,
    WixSettings,
    NsisSettings,
    // NSIS enums
    NSISInstallerMode,
    NsisCompression,
};

/// A bundled artifact result containing metadata about created installers.
///
/// This struct is returned after successful bundling operations and contains
/// information about the generated installer packages.
///
/// # Fields
///
/// - `package_type`: The format of the created package (deb, rpm, dmg, etc.)
/// - `paths`: All files created as part of this bundle (main package + metadata files)
/// - `size`: Total size of the main artifact in bytes
/// - `checksum`: SHA-256 checksum for integrity verification
///
/// # Examples
///
/// ```no_run
/// use kodegen_bundler_release::bundler::{Bundler, Settings, SettingsBuilder, PackageSettings};
///
/// # fn example() -> kodegen_bundler_release::bundler::Result<()> {
/// # let settings = SettingsBuilder::new()
/// #     .project_out_directory("target/release")
/// #     .package_settings(PackageSettings::default())
/// #     .build()?;
/// let bundler = Bundler::new(settings)?;
/// let artifacts = bundler.bundle()?;
///
/// for artifact in artifacts {
///     println!("Created {}: {} bytes", 
///         artifact.package_type, 
///         artifact.size
///     );
///     println!("SHA256: {}", artifact.checksum);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct BundledArtifact {
    /// The package type that was created (e.g., Deb, Rpm, Dmg, Msi).
    pub package_type: PackageType,
    
    /// Paths to all files created as part of this bundle.
    ///
    /// Typically includes the main installer package plus any metadata files
    /// (checksums, signatures, etc.).
    pub paths: Vec<std::path::PathBuf>,
    
    /// Total size of the main artifact in bytes.
    pub size: u64,
    
    /// SHA-256 checksum of the main artifact for integrity verification.
    ///
    /// This can be published alongside the artifact for users to verify downloads.
    pub checksum: String,
}
