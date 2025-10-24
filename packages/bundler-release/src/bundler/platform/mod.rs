//! Platform-specific bundling implementations.
//!
//! This module contains platform-specific code for creating native installers
//! on Linux, macOS, and Windows.
//!
//! # Supported Platforms
//!
//! | Platform | Package Types | Module |
//! |----------|--------------|---------|
//! | Linux | .deb, .rpm, AppImage | [`linux`] |
//! | macOS | .app, .dmg | [`macos`] |
//! | Windows | .msi, .exe (NSIS) | [`windows`] |
//!
//! # Platform Detection
//!
//! The bundler automatically detects the current platform and provides
//! appropriate package types via [`PackageType::all_for_current_platform()`].
//!
//! # Bundling Order
//!
//! Some package types depend on others being built first. For example, DMG
//! installers require the .app bundle to exist. The [`PackageType::priority()`]
//! method ensures correct build order.

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

use std::fmt;

/// Supported package types for bundling.
///
/// Represents the different installer formats that can be created by the bundler.
/// Each platform supports specific package types.
///
/// # Platform Support
///
/// - **Linux**: [`Deb`](Self::Deb), [`Rpm`](Self::Rpm), [`AppImage`](Self::AppImage)
/// - **macOS**: [`MacOsBundle`](Self::MacOsBundle), [`Dmg`](Self::Dmg)
/// - **Windows**: [`WindowsMsi`](Self::WindowsMsi), [`Nsis`](Self::Nsis)
///
/// # Examples
///
/// ```no_run
/// use kodegen_bundler_release::bundler::PackageType;
///
/// // Get all package types for current platform
/// let types = PackageType::all_for_current_platform();
///
/// for pkg_type in types {
///     println!("Creating {} package", pkg_type);
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum PackageType {
    /// macOS application bundle (.app).
    ///
    /// Creates a `.app` bundle for macOS with proper Info.plist and structure.
    MacOsBundle,

    /// macOS DMG disk image (.dmg).
    ///
    /// Creates a distributable disk image containing the .app bundle.
    /// Requires [`MacOsBundle`](Self::MacOsBundle) to be built first.
    Dmg,

    /// Windows MSI installer (.msi).
    ///
    /// Creates a Windows Installer package using WiX Toolset.
    WindowsMsi,

    /// NSIS installer (.exe).
    ///
    /// Creates a lightweight Windows installer using NSIS.
    Nsis,

    /// Debian package (.deb).
    ///
    /// Creates a Debian package for Ubuntu, Debian, and derivatives.
    Deb,

    /// RPM package (.rpm).
    ///
    /// Creates an RPM package for Fedora, RHEL, CentOS, and derivatives.
    Rpm,

    /// Linux AppImage (.AppImage).
    ///
    /// Creates a portable, self-contained executable for Linux.
    AppImage,
}

impl PackageType {
    /// Returns the short name for this package type.
    ///
    /// This is the lowercase identifier used in CLI output and file paths.
    pub fn short_name(&self) -> &'static str {
        match self {
            PackageType::MacOsBundle => "app",
            PackageType::Dmg => "dmg",
            PackageType::WindowsMsi => "msi",
            PackageType::Nsis => "nsis",
            PackageType::Deb => "deb",
            PackageType::Rpm => "rpm",
            PackageType::AppImage => "appimage",
        }
    }

    /// Returns the priority for bundling order.
    ///
    /// Lower numbers are bundled first. This ensures dependencies are built
    /// before dependent types (e.g., .app before .dmg).
    ///
    /// # Priority Values
    ///
    /// - `0`: Independent packages (deb, rpm, msi, nsis, app, appimage)
    /// - `1`: Dependent packages (dmg - requires .app)
    pub fn priority(&self) -> u32 {
        match self {
            PackageType::MacOsBundle => 0,
            PackageType::WindowsMsi => 0,
            PackageType::Nsis => 0,
            PackageType::Deb => 0,
            PackageType::Rpm => 0,
            PackageType::AppImage => 0,
            PackageType::Dmg => 1, // Requires .app to be built first
        }
    }

    /// Returns all package types available on the current platform.
    ///
    /// Automatically detects the operating system and returns appropriate
    /// package types.
    ///
    /// # Returns
    ///
    /// - **Linux**: `[Deb, Rpm, AppImage]`
    /// - **macOS**: `[MacOsBundle, Dmg]`
    /// - **Windows**: `[WindowsMsi, Nsis]`
    /// - **Other**: `[]` (empty)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kodegen_bundler_release::bundler::PackageType;
    ///
    /// let types = PackageType::all_for_current_platform();
    /// println!("Available package types: {:?}", types);
    /// ```
    pub fn all_for_current_platform() -> Vec<PackageType> {
        #[cfg(target_os = "linux")]
        {
            vec![PackageType::Deb, PackageType::Rpm, PackageType::AppImage]
        }
        #[cfg(target_os = "macos")]
        {
            vec![PackageType::MacOsBundle, PackageType::Dmg]
        }
        #[cfg(target_os = "windows")]
        {
            vec![PackageType::WindowsMsi, PackageType::Nsis]
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            vec![]
        }
    }
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_name())
    }
}
