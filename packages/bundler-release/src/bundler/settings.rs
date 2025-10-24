//! Configuration structures for bundling operations.
//!
//! This module provides comprehensive configuration types for multi-platform
//! bundling, including package metadata, platform-specific settings, and
//! builder patterns for constructing settings.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// CPU architecture for target binaries.
///
/// Represents the target architecture for bundled binaries. The architecture is
/// automatically detected from the Rust target triple during bundling.
///
/// # Platform Support
///
/// - ✅ Linux: All architectures supported
/// - ✅ macOS: X86_64, AArch64, Universal
/// - ✅ Windows: X86_64, X86
///
/// # Examples
///
/// ```no_run
/// use kodegen_bundler_release::bundler::Arch;
///
/// let arch = Arch::X86_64;
/// println!("Target architecture: {:?}", arch);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arch {
    /// x86_64 / AMD64 (64-bit) - Most common desktop/server architecture
    X86_64,
    /// x86 / i686 (32-bit) - Legacy 32-bit Intel
    X86,
    /// AArch64 / ARM64 (64-bit) - Apple Silicon, modern ARM devices
    AArch64,
    /// ARM with hard-float (32-bit) - Raspberry Pi and embedded ARM
    Armhf,
    /// ARM with soft-float (32-bit) - Older embedded ARM devices
    Armel,
    /// RISC-V (64-bit) - Emerging open architecture
    Riscv64,
    /// macOS universal binary - Contains both x86_64 and AArch64
    Universal,
}

/// Package metadata and configuration.
///
/// Contains core package information used across all bundling platforms.
/// This typically maps from `Cargo.toml` `[package]` section.
///
/// # Examples
///
/// ```no_run
/// use kodegen_bundler_release::bundler::PackageSettings;
///
/// let settings = PackageSettings {
///     product_name: "MyApp".into(),
///     version: "1.0.0".into(),
///     description: "An awesome application".into(),
///     homepage: Some("https://example.com".into()),
///     authors: Some(vec!["Author Name <email@example.com>".into()]),
///     default_run: Some("myapp".into()),
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct PackageSettings {
    /// Product name displayed to users.
    ///
    /// This is the human-readable name shown in installers and system menus.
    /// Usually derived from `Cargo.toml` `package.name`.
    pub product_name: String,

    /// Version string in semantic versioning format.
    ///
    /// Example: "1.0.0", "0.2.3-beta.1"
    pub version: String,

    /// Brief description of the application.
    ///
    /// Used in package managers and installer descriptions.
    pub description: String,

    /// Homepage URL for the application.
    ///
    /// Default: None
    pub homepage: Option<String>,

    /// List of package authors.
    ///
    /// Format: "Name <email@example.com>"
    ///
    /// Default: None
    pub authors: Option<Vec<String>>,

    /// Default binary to run when multiple binaries exist.
    ///
    /// If the package contains multiple binaries, this specifies which one
    /// should be the primary executable.
    ///
    /// Default: None (uses first binary)
    pub default_run: Option<String>,
}

/// Debian package (.deb) configuration.
///
/// Configures the creation of Debian packages for Ubuntu, Debian, and derivatives.
///
/// # Configuration
///
/// Add to `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bundle.linux.deb]
/// depends = ["libc6 (>= 2.31)", "libssl3"]
/// section = "devel"
/// priority = "optional"
/// ```
///
/// # Dependency Format
///
/// Dependencies follow Debian package syntax:
/// - `package-name` - Any version
/// - `package-name (>= 1.0)` - Minimum version
/// - `package-name (<< 2.0)` - Maximum version
///
/// # Desktop Integration
///
/// If a desktop file template is provided via `desktop_template`, it will be
/// installed to `/usr/share/applications/`.
///
/// # Maintainer Scripts
///
/// Lifecycle scripts are executed during installation/removal:
/// - `pre_install_script` - Before installation
/// - `post_install_script` - After installation  
/// - `pre_remove_script` - Before removal
/// - `post_remove_script` - After removal
///
/// # See Also
///
/// - [`RpmSettings`] - RPM package configuration
/// - [`AppImageSettings`] - AppImage configuration
#[derive(Clone, Debug, Default)]
pub struct DebianSettings {
    /// Package dependencies in Debian syntax.
    ///
    /// Example: `["libc6 (>= 2.31)", "libssl3"]`
    ///
    /// Default: None
    pub depends: Option<Vec<String>>,

    /// Package recommendations (optional dependencies).
    ///
    /// These packages enhance functionality but aren't required.
    ///
    /// Default: None
    pub recommends: Option<Vec<String>>,

    /// Virtual packages this package provides.
    ///
    /// Used for package alternatives and virtual package names.
    ///
    /// Default: None
    pub provides: Option<Vec<String>>,

    /// Packages that cannot be installed alongside this one.
    ///
    /// Default: None
    pub conflicts: Option<Vec<String>>,

    /// Packages this one replaces (for upgrades).
    ///
    /// Default: None
    pub replaces: Option<Vec<String>>,

    /// Custom files to add to package (destination -> source).
    ///
    /// Maps installation paths to source files.
    ///
    /// Default: Empty
    pub files: HashMap<PathBuf, PathBuf>,

    /// Path to custom `.desktop` file template.
    ///
    /// Will be installed to `/usr/share/applications/`.
    ///
    /// Default: None (auto-generated if not provided)
    pub desktop_template: Option<PathBuf>,

    /// Debian control file section.
    ///
    /// Common values: "utils", "devel", "admin", "net"
    ///
    /// Default: None (uses "utils")
    pub section: Option<String>,

    /// Package priority in Debian repository.
    ///
    /// Values: "required", "important", "standard", "optional", "extra"
    ///
    /// Default: None (uses "optional")
    pub priority: Option<String>,

    /// Path to Debian changelog file.
    ///
    /// Default: None (auto-generated)
    pub changelog: Option<PathBuf>,

    /// Pre-install script path (preinst).
    ///
    /// Executed before package installation.
    ///
    /// Default: None
    pub pre_install_script: Option<PathBuf>,

    /// Post-install script path (postinst).
    ///
    /// Executed after package installation.
    ///
    /// Default: None
    pub post_install_script: Option<PathBuf>,

    /// Pre-remove script path (prerm).
    ///
    /// Executed before package removal.
    ///
    /// Default: None
    pub pre_remove_script: Option<PathBuf>,

    /// Post-remove script path (postrm).
    ///
    /// Executed after package removal.
    ///
    /// Default: None
    pub post_remove_script: Option<PathBuf>,
}

/// RPM package (.rpm) configuration.
///
/// Configures the creation of RPM packages for Fedora, RHEL, CentOS, and derivatives.
///
/// # Configuration
///
/// Add to `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bundle.linux.rpm]
/// depends = ["glibc >= 2.31"]
/// release = "1"
/// compression = "zstd"
/// ```
///
/// # Compression Algorithms
///
/// Supported values for `compression`:
/// - `"gzip"` - Standard gzip compression
/// - `"xz"` - Better compression, slower
/// - `"zstd"` - Modern, balanced compression (recommended)
/// - `"bzip2"` - Legacy compression
///
/// # See Also
///
/// - [`DebianSettings`] - Debian package configuration
/// - [`AppImageSettings`] - AppImage configuration
#[derive(Clone, Debug)]
pub struct RpmSettings {
    /// Package dependencies in RPM syntax.
    ///
    /// Example: `["glibc >= 2.31", "openssl-libs"]`
    ///
    /// Default: None
    pub depends: Option<Vec<String>>,

    /// Package recommendations (weak dependencies).
    ///
    /// Default: None
    pub recommends: Option<Vec<String>>,

    /// Virtual packages this package provides.
    ///
    /// Default: None
    pub provides: Option<Vec<String>>,

    /// Packages that cannot be installed alongside this one.
    ///
    /// Default: None
    pub conflicts: Option<Vec<String>>,

    /// Packages this one obsoletes (supersedes).
    ///
    /// Default: None
    pub obsoletes: Option<Vec<String>>,

    /// Release number appended to version.
    ///
    /// Incremented for packaging changes without version bumps.
    ///
    /// Default: "1"
    pub release: String,

    /// Epoch number for version ordering.
    ///
    /// Used to force version ordering when normal comparison fails.
    /// Rarely needed.
    ///
    /// Default: 0
    pub epoch: u32,

    /// Custom files to add to package (destination -> source).
    ///
    /// Default: Empty
    pub files: HashMap<PathBuf, PathBuf>,

    /// Path to custom `.desktop` file template.
    ///
    /// Default: None (auto-generated)
    pub desktop_template: Option<PathBuf>,

    /// Pre-install script path (%pre).
    ///
    /// Default: None
    pub pre_install_script: Option<PathBuf>,

    /// Post-install script path (%post).
    ///
    /// Default: None
    pub post_install_script: Option<PathBuf>,

    /// Pre-remove script path (%preun).
    ///
    /// Default: None
    pub pre_remove_script: Option<PathBuf>,

    /// Post-remove script path (%postun).
    ///
    /// Default: None
    pub post_remove_script: Option<PathBuf>,

    /// Compression algorithm: "gzip", "xz", "zstd", "bzip2".
    ///
    /// Default: None (uses RPM default, typically "gzip")
    pub compression: Option<String>,
}

impl Default for RpmSettings {
    fn default() -> Self {
        Self {
            depends: None,
            recommends: None,
            provides: None,
            conflicts: None,
            obsoletes: None,
            release: "1".to_string(),
            epoch: 0,
            files: HashMap::new(),
            desktop_template: None,
            pre_install_script: None,
            post_install_script: None,
            pre_remove_script: None,
            post_remove_script: None,
            compression: None,
        }
    }
}

/// AppImage portable application configuration.
///
/// AppImage creates self-contained, portable executables for Linux that work
/// across distributions without installation.
///
/// # Configuration
///
/// Add to `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bundle.linux.appimage]
/// bundle_media_framework = true
/// ```
///
/// # Features
///
/// AppImages are portable executables that:
/// - Run on any Linux distribution
/// - Don't require installation or root privileges
/// - Bundle all dependencies internally
///
/// # See Also
///
/// - [`DebianSettings`] - Debian package configuration
/// - [`RpmSettings`] - RPM package configuration
#[derive(Clone, Debug, Default)]
pub struct AppImageSettings {
    /// Custom files to include (destination -> source).
    ///
    /// Default: Empty
    pub files: HashMap<PathBuf, PathBuf>,

    /// Bundle GStreamer media framework.
    ///
    /// Enable this if your application uses audio/video playback.
    ///
    /// Default: false
    pub bundle_media_framework: bool,

    /// Bundle xdg-open binary for opening URLs/files.
    ///
    /// Enable this if your application needs to open web browsers or files.
    ///
    /// Default: false
    pub bundle_xdg_open: bool,
}

/// macOS application bundle (.app) configuration.
///
/// Configures the creation of macOS `.app` bundles with optional code signing
/// and notarization.
///
/// # Configuration
///
/// Add to `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bundle.macos]
/// minimum_system_version = "10.15"
/// signing_identity = "Developer ID Application: Your Name (TEAMID)"
/// entitlements = "entitlements.plist"
/// ```
///
/// # Code Signing
///
/// See [`kodegen_sign`](../../sign/index.html) for automated certificate provisioning
/// and code signing setup.
///
/// # See Also
///
/// - [`DmgSettings`] - DMG disk image configuration
/// - [`WindowsSettings`] - Windows installer configuration
#[derive(Clone, Debug, Default)]
pub struct MacOsSettings {
    /// System frameworks to bundle with the application.
    ///
    /// Example: `["WebKit.framework", "Security.framework"]`
    ///
    /// Default: None (no additional frameworks)
    pub frameworks: Option<Vec<String>>,

    /// Minimum macOS version required (LSMinimumSystemVersion).
    ///
    /// Example: "10.15", "11.0", "12.0"
    ///
    /// Default: None (uses current SDK version)
    pub minimum_system_version: Option<String>,

    /// Code signing identity name.
    ///
    /// Example: "Developer ID Application: Your Name (TEAMID)"
    ///
    /// Use "-" for ad-hoc signing (development only).
    ///
    /// Default: None (unsigned)
    pub signing_identity: Option<String>,

    /// Path to entitlements.plist for code signing.
    ///
    /// Required for certain macOS features (network, camera, etc.).
    ///
    /// Default: None
    pub entitlements: Option<PathBuf>,

    /// Custom files to include (destination -> source).
    ///
    /// Default: Empty
    pub files: HashMap<PathBuf, PathBuf>,

    /// Skip notarization with Apple.
    ///
    /// Notarization is required for distribution outside the Mac App Store.
    /// Only skip for development/testing.
    ///
    /// Default: false (notarization enabled)
    pub skip_notarization: bool,

    /// Skip stapling the notarization ticket.
    ///
    /// Stapling attaches the notarization ticket to the bundle for offline verification.
    ///
    /// Default: false (stapling enabled)
    pub skip_stapling: bool,
}

/// macOS DMG disk image configuration.
///
/// Configures the appearance and layout of macOS disk image installers.
///
/// # Configuration
///
/// Add to `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bundle.dmg]
/// background = "assets/dmg-background.png"
/// window_size = [540, 380]
/// ```
///
/// # See Also
///
/// - [`MacOsSettings`] - macOS app bundle configuration
#[derive(Clone, Debug, Default)]
pub struct DmgSettings {
    /// Path to background image for DMG window.
    ///
    /// Should be PNG format. Recommended size: 540x380 pixels.
    ///
    /// Default: None (plain background)
    pub background: Option<PathBuf>,

    /// DMG window size (width, height) in pixels.
    ///
    /// Default: None (uses default size)
    pub window_size: Option<(u32, u32)>,
}

/// Windows installer configuration.
///
/// Configures Windows installers (MSI via WiX, EXE via NSIS) with optional
/// Authenticode code signing.
///
/// # Configuration
///
/// Add to `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bundle.windows]
/// cert_path = "cert.pem"
/// key_path = "key.pem"
/// timestamp_url = "http://timestamp.digicert.com"
/// ```
///
/// # Code Signing
///
/// See [`kodegen_sign`](../../sign/index.html) for Authenticode signing setup
/// using osslsigncode.
///
/// # See Also
///
/// - [`WixSettings`] - WiX MSI installer configuration
/// - [`NsisSettings`] - NSIS installer configuration
#[derive(Clone, Debug, Default)]
pub struct WindowsSettings {
    // === Signing Configuration ===
    /// Path to certificate file (.pem, .crt, .pfx).
    ///
    /// For PKCS#12 (.pfx), also set `password`.
    ///
    /// Default: None (unsigned)
    pub cert_path: Option<PathBuf>,

    /// Path to private key file (.pem, .key).
    ///
    /// Not needed for PKCS#12 (.pfx) files which contain both cert and key.
    ///
    /// Default: None
    pub key_path: Option<PathBuf>,

    /// Password for encrypted key or PKCS#12 file.
    ///
    /// Default: None
    pub password: Option<String>,

    /// Timestamp server URL for signature timestamping.
    ///
    /// Recommended: "http://timestamp.digicert.com"
    ///
    /// Default: None (uses default timestamp server)
    pub timestamp_url: Option<String>,

    // === Legacy/Alternative Fields ===
    /// Custom sign command for alternative signing tools.
    ///
    /// Example: "signtool sign /sha1 ABC123... %1"
    ///
    /// Default: None (uses osslsigncode)
    pub sign_command: Option<String>,

    // === Installer Settings ===
    /// WiX MSI installer settings.
    ///
    /// See [`WixSettings`] for details.
    pub wix: WixSettings,

    /// NSIS EXE installer settings.
    ///
    /// See [`NsisSettings`] for details.
    pub nsis: NsisSettings,
}

/// WiX MSI installer configuration.
///
/// WiX creates professional Windows Installer (.msi) packages.
///
/// # Configuration
///
/// Add to `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bundle.windows.wix]
/// language = ["en-US"]
/// license = "LICENSE.rtf"
/// ```
///
/// # See Also
///
/// - [`WindowsSettings`] - Windows installer configuration
/// - [`NsisSettings`] - NSIS installer configuration
#[derive(Clone, Debug, Default)]
pub struct WixSettings {
    /// Supported installer languages.
    ///
    /// Example: `["en-US", "de-DE", "fr-FR"]`
    ///
    /// Default: Empty (uses "en-US")
    pub language: Vec<String>,

    /// Path to custom WiX template (.wxs file).
    ///
    /// Default: None (uses built-in template)
    pub template: Option<PathBuf>,

    /// Paths to WiX fragment files to include.
    ///
    /// Default: Empty
    pub fragment_paths: Vec<PathBuf>,

    /// Component group references to include.
    ///
    /// Default: Empty
    pub component_group_refs: Vec<String>,

    /// Component references to include.
    ///
    /// Default: Empty
    pub component_refs: Vec<String>,

    /// Feature group references to include.
    ///
    /// Default: Empty
    pub feature_group_refs: Vec<String>,

    /// Feature references to include.
    ///
    /// Default: Empty
    pub feature_refs: Vec<String>,

    /// Merge module (.msm) references to include.
    ///
    /// Default: Empty
    pub merge_refs: Vec<String>,

    /// Skip WebView2 runtime installation.
    ///
    /// Set to true if your app doesn't use WebView2.
    ///
    /// Default: false
    pub skip_webview_install: bool,

    /// Path to license file (.rtf format required).
    ///
    /// Shown during installation.
    ///
    /// Default: None
    pub license: Option<PathBuf>,

    /// Enable elevated update task for automatic updates.
    ///
    /// Default: false
    pub enable_elevated_update_task: bool,

    /// Path to banner image (493×58 pixels).
    ///
    /// Shown at top of installer dialogs.
    ///
    /// Default: None
    pub banner_path: Option<PathBuf>,

    /// Path to dialog image (493×312 pixels).
    ///
    /// Shown on installer welcome screen.
    ///
    /// Default: None
    pub dialog_image_path: Option<PathBuf>,
}

/// NSIS installer mode (installation scope).
///
/// Determines whether the installer installs for the current user only,
/// all users (requires admin), or lets the user choose.
///
/// # Configuration
///
/// ```toml
/// [package.metadata.bundle.windows.nsis]
/// installer_mode = "perMachine"  # or "currentUser" or "both"
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NSISInstallerMode {
    /// Per-user installation (no admin rights required).
    ///
    /// Installs to `%LOCALAPPDATA%`.
    #[default]
    CurrentUser,

    /// Per-machine installation (requires admin rights).
    ///
    /// Installs to `%PROGRAMFILES%`.
    PerMachine,

    /// Let user choose during installation.
    Both,
}

/// NSIS compression algorithm.
///
/// Controls the compression method used for the NSIS installer executable.
///
/// # Comparison
///
/// | Algorithm | Speed | Size | Notes |
/// |-----------|-------|------|-------|
/// | None | Fastest | Largest | Development only |
/// | Zlib | Fast | Medium | Default, good balance |
/// | Bzip2 | Medium | Small | Better compression |
/// | LZMA | Slowest | Smallest | Best compression |
///
/// # Configuration
///
/// ```toml
/// [package.metadata.bundle.windows.nsis]
/// compression = "lzma"
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NsisCompression {
    /// No compression - fastest, largest size.
    None,

    /// zlib compression - good balance (default).
    #[default]
    Zlib,

    /// bzip2 compression - smaller than zlib.
    Bzip2,

    /// LZMA compression - smallest size, slowest.
    Lzma,
}

/// NSIS installer (.exe) configuration.
///
/// NSIS creates lightweight, customizable Windows installer executables.
///
/// # Configuration
///
/// Add to `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bundle.windows.nsis]
/// installer_mode = "perMachine"
/// compression = "lzma"
/// languages = ["en-US", "de-DE"]
/// ```
///
/// # See Also
///
/// - [`WindowsSettings`] - Windows installer configuration
/// - [`WixSettings`] - WiX MSI installer configuration
/// - [`NSISInstallerMode`] - Installation scope
/// - [`NsisCompression`] - Compression algorithms
#[derive(Clone, Debug, Default)]
pub struct NsisSettings {
    /// Path to custom NSIS template (.nsi file).
    ///
    /// Default: None (uses built-in template)
    pub template: Option<PathBuf>,

    /// Path to header image (150×57 pixels).
    ///
    /// Shown at top of installer window.
    ///
    /// Default: None
    pub header_image: Option<PathBuf>,

    /// Path to sidebar image (164×314 pixels).
    ///
    /// Shown on left side of installer window.
    ///
    /// Default: None
    pub sidebar_image: Option<PathBuf>,

    /// Path to installer icon (.ico file).
    ///
    /// Icon for the installer executable itself.
    ///
    /// Default: None (uses application icon)
    pub installer_icon: Option<PathBuf>,

    /// Installation mode (per-user, per-machine, or both).
    ///
    /// Default: [`NSISInstallerMode::CurrentUser`]
    pub install_mode: NSISInstallerMode,

    /// Supported installer languages.
    ///
    /// Example: `["en-US", "de-DE"]`
    ///
    /// Default: None (uses English)
    pub languages: Option<Vec<String>>,

    /// Compression algorithm for installer.
    ///
    /// Default: None (uses [`NsisCompression::Zlib`])
    pub compression: Option<NsisCompression>,
}

/// Bundle configuration for all platforms.
///
/// Central configuration structure containing metadata and platform-specific settings.
///
/// # Configuration
///
/// Add to `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bundle]
/// identifier = "com.example.app"
/// publisher = "Example Inc."
/// icon = ["assets/icon.png"]
/// resources = ["config/**/*"]
/// category = "Utility"
/// ```
///
/// # See Also
///
/// - [`DebianSettings`] - Debian package configuration
/// - [`RpmSettings`] - RPM package configuration
/// - [`MacOsSettings`] - macOS app bundle configuration
/// - [`WindowsSettings`] - Windows installer configuration
#[derive(Debug, Clone, Default)]
pub struct BundleSettings {
    /// Bundle identifier in reverse domain notation.
    ///
    /// Example: "com.example.app", "ai.kodegen.app"
    ///
    /// Required for macOS and some Linux desktop integrations.
    ///
    /// Default: None
    pub identifier: Option<String>,

    /// Publisher/company name.
    ///
    /// Default: None
    pub publisher: Option<String>,

    /// Icon file paths (PNG recommended).
    ///
    /// Provide multiple sizes for best quality:
    /// `["icon-32.png", "icon-128.png", "icon-512.png"]`
    ///
    /// Auto-converted to platform-specific formats (ICNS, ICO).
    ///
    /// Default: None
    pub icon: Option<Vec<PathBuf>>,

    /// Resource glob patterns to bundle.
    ///
    /// Example: `["config/**/*", "templates/**/*"]`
    ///
    /// Default: None
    pub resources: Option<Vec<String>>,

    /// Copyright notice string.
    ///
    /// Example: "Copyright © 2024 Example Inc."
    ///
    /// Default: None
    pub copyright: Option<String>,

    /// Application category.
    ///
    /// Common values: "Utility", "Developer Tools", "Graphics", "Productivity"
    ///
    /// Default: None
    pub category: Option<String>,

    /// Short description (one line).
    ///
    /// Used in package managers and installer summaries.
    ///
    /// Default: None
    pub short_description: Option<String>,

    /// Long description (multiple paragraphs).
    ///
    /// Used in package details and documentation.
    ///
    /// Default: None
    pub long_description: Option<String>,

    /// External binaries to bundle.
    ///
    /// List of binary names (without path). Each must have a platform-specific
    /// variant: `binary-{target}` or `binary-{target}.exe`
    ///
    /// Example: `["helper"]` expects `helper-x86_64-unknown-linux-gnu`, etc.
    ///
    /// Default: None
    pub external_bin: Option<Vec<String>>,

    /// Debian-specific settings.
    ///
    /// See [`DebianSettings`] for details.
    pub deb: DebianSettings,

    /// RPM-specific settings.
    ///
    /// See [`RpmSettings`] for details.
    pub rpm: RpmSettings,

    /// AppImage-specific settings.
    ///
    /// See [`AppImageSettings`] for details.
    pub appimage: AppImageSettings,

    /// macOS-specific settings.
    ///
    /// See [`MacOsSettings`] for details.
    pub macos: MacOsSettings,

    /// DMG-specific settings.
    ///
    /// See [`DmgSettings`] for details.
    pub dmg: DmgSettings,

    /// Windows-specific settings.
    ///
    /// See [`WindowsSettings`] for details.
    pub windows: WindowsSettings,
}

/// A binary to bundle into the installer.
///
/// Represents an executable to include in the bundle. Multiple binaries can be
/// bundled, but typically one is marked as the main executable.
///
/// # Examples
///
/// ```no_run
/// use kodegen_bundler_release::bundler::BundleBinary;
///
/// let main_binary = BundleBinary::new("myapp".into(), true);
/// let helper = BundleBinary::new("myapp-helper".into(), false);
/// ```
#[derive(Clone, Debug)]
pub struct BundleBinary {
    name: String,
    main: bool,
    src_path: Option<String>,
}

impl BundleBinary {
    /// Creates a new bundle binary.
    ///
    /// # Arguments
    ///
    /// * `name` - Binary name (without extension)
    /// * `main` - Whether this is the main executable
    pub fn new(name: String, main: bool) -> Self {
        Self {
            name,
            main,
            src_path: None,
        }
    }

    /// Creates a new bundle binary with source path.
    ///
    /// # Arguments
    ///
    /// * `name` - Binary name (without extension)
    /// * `main` - Whether this is the main executable
    /// * `src_path` - Optional path to binary source
    pub fn with_path(name: String, main: bool, src_path: Option<String>) -> Self {
        Self {
            name,
            src_path,
            main,
        }
    }

    /// Mark the binary as the main executable.
    ///
    /// The main executable is used for desktop shortcuts and start menu entries.
    pub fn set_main(&mut self, main: bool) {
        self.main = main;
    }

    /// Sets the binary name.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Sets the source path of the binary.
    ///
    /// Returns self for method chaining.
    pub fn set_src_path(mut self, src_path: Option<String>) -> Self {
        self.src_path = src_path;
        self
    }

    /// Returns whether this is the main executable.
    pub fn main(&self) -> bool {
        self.main
    }

    /// Returns the binary name (without extension).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the binary source path if set.
    pub fn src_path(&self) -> Option<&String> {
        self.src_path.as_ref()
    }
}

/// Main settings for bundler operations.
///
/// Central configuration for the bundler, constructed via [`SettingsBuilder`].
/// Contains package metadata, bundle settings, and platform-specific configuration.
///
/// # Examples
///
/// ```no_run
/// use kodegen_bundler_release::bundler::{Settings, SettingsBuilder, PackageSettings};
///
/// # fn example() -> kodegen_bundler_release::bundler::Result<()> {
/// let settings = SettingsBuilder::new()
///     .project_out_directory("target/release")
///     .package_settings(PackageSettings {
///         product_name: "MyApp".into(),
///         version: "1.0.0".into(),
///         description: "My application".into(),
///         ..Default::default()
///     })
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// # See Also
///
/// - [`SettingsBuilder`] - Builder for constructing Settings
/// - [`PackageSettings`] - Package metadata
/// - [`BundleSettings`] - Bundle configuration
#[derive(Clone, Debug)]
pub struct Settings {
    /// Package metadata.
    package: PackageSettings,

    /// Bundle configuration.
    bundle_settings: BundleSettings,

    /// Output directory for bundles.
    ///
    /// Typically `target/release` or `target/debug`.
    project_out_directory: PathBuf,

    /// Package types to create.
    ///
    /// None means use platform defaults (.deb on Debian, .rpm on Fedora, etc.).
    package_types: Option<Vec<crate::bundler::platform::PackageType>>,

    /// Binaries to bundle.
    binaries: Vec<BundleBinary>,

    /// Target triple (e.g., "x86_64-unknown-linux-gnu").
    ///
    /// Used for architecture detection.
    target: String,
}

impl Settings {
    /// Returns the product name.
    pub fn product_name(&self) -> &str {
        &self.package.product_name
    }

    /// Returns the version string.
    pub fn version_string(&self) -> &str {
        &self.package.version
    }

    /// Returns the package description.
    pub fn description(&self) -> &str {
        &self.package.description
    }

    /// Returns the project output directory.
    ///
    /// This is where compiled binaries are located.
    pub fn project_out_directory(&self) -> &Path {
        &self.project_out_directory
    }

    /// Detects the binary architecture from the target triple.
    ///
    /// Automatically determines the target architecture based on the Rust
    /// target triple (e.g., "x86_64-unknown-linux-gnu" → `Arch::X86_64`).
    pub fn binary_arch(&self) -> Arch {
        if self.target.starts_with("x86_64") {
            Arch::X86_64
        } else if self.target.starts_with('i') {
            Arch::X86
        } else if self.target.starts_with("aarch64") {
            Arch::AArch64
        } else if self.target.starts_with("arm") && self.target.ends_with("hf") {
            Arch::Armhf
        } else if self.target.starts_with("arm") {
            Arch::Armel
        } else if self.target.starts_with("riscv64") {
            Arch::Riscv64
        } else {
            Arch::X86_64 // fallback
        }
    }

    /// Returns the binaries to bundle.
    pub fn binaries(&self) -> &[BundleBinary] {
        &self.binaries
    }

    /// Returns the full path to a binary.
    ///
    /// Automatically appends `.exe` extension on Windows.
    pub fn binary_path(&self, binary: &BundleBinary) -> PathBuf {
        let mut path = self.project_out_directory.join(binary.name());

        if cfg!(target_os = "windows") {
            path.set_extension("exe");
        }

        path
    }

    /// Returns the bundle settings.
    pub fn bundle_settings(&self) -> &BundleSettings {
        &self.bundle_settings
    }

    /// Returns the package types to create.
    ///
    /// None means use platform defaults.
    pub fn package_types(&self) -> Option<&[crate::bundler::platform::PackageType]> {
        self.package_types.as_deref()
    }

    /// Loads and returns icon files with metadata.
    ///
    /// Reads icon files from paths specified in bundle settings and returns
    /// icon information including dimensions and format.
    ///
    /// # Errors
    ///
    /// Returns `IconPathError` if no icon paths are configured.
    pub fn icon_files(
        &self,
    ) -> crate::bundler::Result<Vec<crate::bundler::resources::icons::IconInfo>> {
        use crate::bundler::resources::icons::load_icons;

        if let Some(icon_paths) = &self.bundle_settings.icon {
            load_icons(icon_paths)
        } else {
            Err(crate::bundler::Error::IconPathError)
        }
    }

    /// Returns the package homepage URL.
    pub fn homepage(&self) -> Option<&str> {
        self.package.homepage.as_deref()
    }

    /// Returns the package authors.
    pub fn authors(&self) -> Option<&[String]> {
        self.package.authors.as_deref()
    }
}

/// Builder for constructing [`Settings`].
///
/// Provides a fluent API for building bundler settings with validation.
///
/// # Examples
///
/// ```no_run
/// use kodegen_bundler_release::bundler::{SettingsBuilder, PackageSettings, BundleBinary};
///
/// # fn example() -> kodegen_bundler_release::bundler::Result<()> {
/// let settings = SettingsBuilder::new()
///     .project_out_directory("target/release")
///     .package_settings(PackageSettings {
///         product_name: "MyApp".into(),
///         version: "1.0.0".into(),
///         description: "My application".into(),
///         ..Default::default()
///     })
///     .binaries(vec![
///         BundleBinary::new("myapp".into(), true),
///     ])
///     .target("x86_64-unknown-linux-gnu".into())
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// # See Also
///
/// - [`Settings`] - The built settings struct
#[derive(Default)]
pub struct SettingsBuilder {
    project_out_directory: Option<PathBuf>,
    package_settings: Option<PackageSettings>,
    bundle_settings: BundleSettings,
    package_types: Option<Vec<crate::bundler::platform::PackageType>>,
    binaries: Vec<BundleBinary>,
    target: Option<String>,
}

impl SettingsBuilder {
    /// Creates a new settings builder.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the project output directory.
    ///
    /// This should point to where compiled binaries are located,
    /// typically `target/release` or `target/debug`.
    ///
    /// # Required
    ///
    /// This field is required for building.
    pub fn project_out_directory<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.project_out_directory = Some(path.as_ref().to_path_buf());
        self
    }

    /// Sets package metadata.
    ///
    /// # Required
    ///
    /// This field is required for building.
    pub fn package_settings(mut self, settings: PackageSettings) -> Self {
        self.package_settings = Some(settings);
        self
    }

    /// Sets bundle configuration.
    ///
    /// Default: Empty [`BundleSettings`]
    pub fn bundle_settings(mut self, settings: BundleSettings) -> Self {
        self.bundle_settings = settings;
        self
    }

    /// Sets specific package types to create.
    ///
    /// If not set, uses platform defaults (e.g., .deb on Debian systems).
    ///
    /// Default: None (platform defaults)
    pub fn package_types(mut self, types: Vec<crate::bundler::platform::PackageType>) -> Self {
        self.package_types = Some(types);
        self
    }

    /// Sets binaries to bundle.
    ///
    /// Default: Empty (no binaries bundled)
    pub fn binaries(mut self, binaries: Vec<BundleBinary>) -> Self {
        self.binaries = binaries;
        self
    }

    /// Sets target triple.
    ///
    /// If not set, uses the `TARGET` environment variable or current architecture.
    ///
    /// Default: Current architecture
    pub fn target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }

    /// Builds the settings.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing:
    /// - `project_out_directory`
    /// - `package_settings`
    pub fn build(self) -> crate::bundler::Result<Settings> {
        use crate::bundler::error::Context;

        let target = self.target.unwrap_or_else(|| {
            std::env::var("TARGET").unwrap_or_else(|_| std::env::consts::ARCH.to_string())
        });

        Ok(Settings {
            package: self
                .package_settings
                .context("package_settings is required")?,
            bundle_settings: self.bundle_settings,
            project_out_directory: self
                .project_out_directory
                .context("project_out_directory is required")?,
            package_types: self.package_types,
            binaries: self.binaries,
            target,
        })
    }
}
