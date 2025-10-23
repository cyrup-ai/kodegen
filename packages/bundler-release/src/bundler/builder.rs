//! Bundle orchestration and coordination.
//!
//! This module provides the main [`Bundler`] orchestrator that coordinates
//! platform-specific bundling operations to create native installers.
//!
//! # Overview
//!
//! The bundler:
//! 1. Reads configuration from [`Settings`]
//! 2. Determines which package types to create
//! 3. Delegates to platform-specific modules
//! 4. Calculates checksums and metadata
//! 5. Returns [`BundledArtifact`] results
//!
//! # Example
//!
//! ```no_run
//! use kodegen_bundler_release::bundler::{Bundler, SettingsBuilder, PackageSettings};
//!
//! # fn example() -> kodegen_bundler_release::bundler::Result<()> {
//! let settings = SettingsBuilder::new()
//!     .project_out_directory("target/release")
//!     .package_settings(PackageSettings {
//!         product_name: "MyApp".into(),
//!         version: "1.0.0".into(),
//!         description: "My application".into(),
//!         ..Default::default()
//!     })
//!     .build()?;
//!
//! let bundler = Bundler::new(settings)?;
//! let artifacts = bundler.bundle()?;
//!
//! for artifact in artifacts {
//!     println!("Created: {:?} ({} bytes)", artifact.package_type, artifact.size);
//!     println!("SHA256: {}", artifact.checksum);
//! }
//! # Ok(())
//! # }
//! ```

use crate::bundler::{Settings, BundledArtifact, Result, PackageType};
use crate::bail;

/// Main bundler orchestrator.
///
/// Coordinates the creation of platform-specific installers by delegating to
/// platform modules and collecting results.
///
/// # Platform Support
///
/// - **Linux**: Creates .deb, .rpm, and AppImage packages
/// - **macOS**: Creates .app bundles and .dmg disk images
/// - **Windows**: Creates .msi and .exe (NSIS) installers
///
/// # Examples
///
/// ```no_run
/// use kodegen_bundler_release::bundler::{Bundler, Settings, PackageType};
///
/// # fn example(settings: Settings) -> kodegen_bundler_release::bundler::Result<()> {
/// // Create bundler
/// let bundler = Bundler::new(settings)?;
///
/// // Bundle with platform defaults
/// let artifacts = bundler.bundle()?;
///
/// // Or bundle specific types
/// let artifacts = bundler.bundle_types(&[
///     PackageType::Deb,
///     PackageType::AppImage,
/// ])?;
/// # Ok(())
/// # }
/// ```
pub struct Bundler {
    settings: Settings,
    #[cfg(target_os = "macos")]
    _temp_keychain: Option<kodegen_bundler_sign::macos::TempKeychain>,
}

impl std::fmt::Debug for Bundler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("Bundler");
        debug_struct.field("settings", &self.settings);
        #[cfg(target_os = "macos")]
        debug_struct.field("_temp_keychain", &self._temp_keychain.as_ref().map(|_| "<TempKeychain>"));
        debug_struct.finish()
    }
}

impl Bundler {
    /// Creates a new bundler with the given settings.
    ///
    /// # Arguments
    ///
    /// * `settings` - Bundler configuration from `SettingsBuilder`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kodegen_bundler_release::bundler::{Bundler, Settings};
    ///
    /// # fn example(settings: Settings) -> kodegen_bundler_release::bundler::Result<()> {
    /// let bundler = Bundler::new(settings)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(settings: Settings) -> Result<Self> {
        #[cfg(target_os = "macos")]
        let _temp_keychain = Self::setup_macos_signing()?;

        Ok(Self {
            settings,
            #[cfg(target_os = "macos")]
            _temp_keychain,
        })
    }

    /// Setup macOS code signing from environment variables
    ///
    /// This function handles certificate import from environment variables for CI/CD.
    /// - APPLE_CERTIFICATE: Base64-encoded .p12 certificate imported to temp keychain
    /// - APPLE_API_KEY, APPLE_API_ISSUER: Used directly by xcrun notarytool (no file needed)
    ///
    /// The TempKeychain is kept alive for the lifetime of the Bundler, ensuring
    /// the certificate remains available for all signing operations.
    #[cfg(target_os = "macos")]
    fn setup_macos_signing() -> Result<Option<kodegen_bundler_sign::macos::TempKeychain>> {
        // Note: API key env vars (APPLE_API_KEY, APPLE_API_ISSUER, APPLE_API_KEY_CONTENT)
        // are used directly by xcrun notarytool - no need to write .p8 files

        // Import certificate if APPLE_CERTIFICATE is set
        if let (Ok(cert_b64), Ok(password)) = (
            std::env::var("APPLE_CERTIFICATE"),
            std::env::var("APPLE_CERTIFICATE_PASSWORD").map(|p| p.trim().to_string())
        ) {
            use base64::Engine;
            let cert_bytes = base64::engine::general_purpose::STANDARD.decode(cert_b64)
                .map_err(|e| crate::bundler::Error::GenericError(format!(
                    "Invalid APPLE_CERTIFICATE (not valid base64): {}", e
                )))?;

            log::info!("Importing certificate from APPLE_CERTIFICATE environment variable");
            let keychain = kodegen_bundler_sign::macos::TempKeychain::from_certificate_bytes(&cert_bytes, &password)
                .map_err(|e| crate::bundler::Error::GenericError(format!(
                    "Failed to import certificate: {}", e
                )))?;

            log::info!("✓ Certificate imported to temporary keychain");
            return Ok(Some(keychain));
        }

        Ok(None)
    }

    /// Executes bundling operations for default platform types.
    ///
    /// Automatically determines which package types to create based on:
    /// 1. Explicit types from [`Settings::package_types()`] if set
    /// 2. Platform defaults otherwise (e.g., .deb + AppImage on Linux)
    ///
    /// # Returns
    ///
    /// Vector of [`BundledArtifact`] results, one per created package.
    ///
    /// # Platform Defaults
    ///
    /// - **Linux**: Deb, AppImage
    /// - **macOS**: MacOsBundle, Dmg
    /// - **Windows**: WindowsMsi, Nsis
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kodegen_bundler_release::bundler::Bundler;
    ///
    /// # fn example(bundler: Bundler) -> kodegen_bundler_release::bundler::Result<()> {
    /// let artifacts = bundler.bundle()?;
    /// println!("Created {} packages", artifacts.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn bundle(&self) -> Result<Vec<BundledArtifact>> {
        let package_types = self.determine_platform_types();
        self.bundle_types(&package_types)
    }

    /// Executes bundling operations for specific package types.
    ///
    /// Creates installers for the specified package types, regardless of platform
    /// defaults. Useful for creating only specific formats or cross-compiling.
    ///
    /// # Arguments
    ///
    /// * `types` - Slice of [`PackageType`] variants to create
    ///
    /// # Returns
    ///
    /// Vector of [`BundledArtifact`] results, one per created package.
    ///
    /// # Bundling Order
    ///
    /// Package types are created in the order provided, but some types have
    /// dependencies (e.g., DMG requires .app to exist first).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kodegen_bundler_release::bundler::{Bundler, PackageType};
    ///
    /// # fn example(bundler: Bundler) -> kodegen_bundler_release::bundler::Result<()> {
    /// // Create only Debian and AppImage packages
    /// let artifacts = bundler.bundle_types(&[
    ///     PackageType::Deb,
    ///     PackageType::AppImage,
    /// ])?;
    ///
    /// for artifact in artifacts {
    ///     println!("Created: {}", artifact.package_type);
    ///     for path in &artifact.paths {
    ///         println!("  {}", path.display());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Platform Compatibility
    ///
    /// Attempting to create a package type unsupported on the current platform
    /// will return an error.
    pub fn bundle_types(&self, types: &[PackageType]) -> Result<Vec<BundledArtifact>> {
        let mut artifacts = Vec::new();
        
        for package_type in types {
            let paths = match package_type {
                #[cfg(target_os = "linux")]
                PackageType::Deb => {
                    crate::bundler::platform::linux::debian::bundle_project(&self.settings)?
                }
                #[cfg(target_os = "linux")]
                PackageType::Rpm => {
                    crate::bundler::platform::linux::rpm::bundle_project(&self.settings)?
                }
                #[cfg(target_os = "linux")]
                PackageType::AppImage => {
                    crate::bundler::platform::linux::appimage::bundle_project(&self.settings)?
                }
                #[cfg(target_os = "macos")]
                PackageType::MacOsBundle => {
                    crate::bundler::platform::macos::app::bundle_project(&self.settings)?
                }
                #[cfg(target_os = "macos")]
                PackageType::Dmg => {
                    crate::bundler::platform::macos::dmg::bundle_project(&self.settings)?
                }
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                PackageType::WindowsMsi => {
                    crate::bundler::platform::windows::msi::bundle_project(&self.settings)?
                }
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                PackageType::Nsis => {
                    crate::bundler::platform::windows::nsis::bundle_project(&self.settings)?
                }
                #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
                _ => {
                    bail!("Package type {:?} not supported on this platform", package_type);
                }
                #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
                _ => {
                    bail!("Package type {:?} not supported on this platform", package_type);
                }
            };
            
            // Calculate artifact metadata
            let size = paths.iter().fold(0u64, |acc, p| {
                acc + std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
            });
            
            let checksum = if !paths.is_empty() {
                calculate_sha256(&paths[0])?
            } else {
                String::new()
            };
            
            artifacts.push(BundledArtifact {
                package_type: *package_type,
                paths,
                size,
                checksum,
            });
        }
        
        Ok(artifacts)
    }

    /// Returns a reference to the bundler settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    
    /// Determines which package types to build based on host platform.
    ///
    /// Returns explicit types from settings if specified, otherwise returns
    /// platform-appropriate defaults.
    fn determine_platform_types(&self) -> Vec<PackageType> {
        // If explicit types specified, use those
        if let Some(types) = self.settings.package_types() {
            return types.to_vec();
        }
        
        // Otherwise default to current platform
        if cfg!(target_os = "linux") {
            vec![PackageType::Deb, PackageType::AppImage]
        } else if cfg!(target_os = "macos") {
            vec![PackageType::MacOsBundle, PackageType::Dmg]
        } else if cfg!(target_os = "windows") {
            vec![PackageType::WindowsMsi, PackageType::Nsis]
        } else {
            vec![]
        }
    }
}

/// Calculates SHA256 checksum of a file or directory.
///
/// For files: Reads in 8KB chunks and computes the SHA-256 hash.
/// For directories: Recursively hashes all files in deterministic order.
///
/// # Arguments
///
/// * `path` - Path to file or directory to hash
///
/// # Returns
///
/// * `Ok(String)` - Hex-encoded SHA-256 hash (64 characters)
/// * `Err` - If path cannot be read or is neither file nor directory
fn calculate_sha256(path: &std::path::Path) -> Result<String> {
    use sha2::{Sha256, Digest};
    use std::io::Read;
    
    let metadata = std::fs::metadata(path)
        .map_err(crate::bundler::Error::IoError)?;
    
    if metadata.is_file() {
        // Hash a single file
        let mut file = std::fs::File::open(path)
            .map_err(crate::bundler::Error::IoError)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        
        loop {
            let n = file.read(&mut buffer)
                .map_err(crate::bundler::Error::IoError)?;
            if n == 0 { break; }
            hasher.update(&buffer[..n]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    } else if metadata.is_dir() {
        // Hash directory tree (e.g., macOS .app bundles)
        calculate_directory_sha256(path)
    } else {
        bail!("Path is neither file nor directory: {}", path.display())
    }
}

/// Calculates SHA256 checksum of a directory tree.
///
/// Recursively traverses the directory, hashing each file's path and content
/// in sorted order to ensure deterministic results. This is used for macOS
/// .app bundles which are directories, not single files.
///
/// # Algorithm
///
/// 1. Recursively collect all files using walkdir
/// 2. Sort paths lexicographically for deterministic order
/// 3. For each file: hash(relative_path + file_content)
/// 4. Return final combined hash
///
/// # Arguments
///
/// * `dir_path` - Path to directory to hash
///
/// # Returns
///
/// * `Ok(String)` - Hex-encoded SHA-256 hash of entire directory tree
/// * `Err` - If directory cannot be traversed
fn calculate_directory_sha256(dir_path: &std::path::Path) -> Result<String> {
    use sha2::{Sha256, Digest};
    use std::io::Read;
    
    // Collect all files recursively
    let mut entries: Vec<_> = walkdir::WalkDir::new(dir_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
    
    // Sort by path for deterministic ordering
    entries.sort_by_key(|e| e.path().to_path_buf());
    
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    
    for entry in entries {
        // Include relative path in hash (preserves directory structure)
        if let Ok(rel_path) = entry.path().strip_prefix(dir_path) {
            hasher.update(rel_path.to_string_lossy().as_bytes());
        }
        
        // Hash file content
        if let Ok(mut file) = std::fs::File::open(entry.path()) {
            loop {
                match file.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => hasher.update(&buffer[..n]),
                    Err(_) => break, // Skip unreadable files
                }
            }
        }
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}
