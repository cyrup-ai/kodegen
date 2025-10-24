//! Platform detection and classification for Docker-based builds.
//!
//! Determines which platforms can be built natively vs which require Docker containers.

use crate::bundler::PackageType;

#[cfg(target_os = "linux")]
use std::sync::OnceLock;

/// Splits package types into native (run locally) vs containerized (run in Docker).
///
/// Based on the current host OS, determines which platforms can be built natively
/// and which require a Docker container.
///
/// # Platform Support
///
/// - **macOS**: Native=[MacOsBundle, Dmg], Container=[Deb, Rpm, AppImage, Nsis, WindowsMsi]
/// - **Linux**: Native=[Deb, Rpm, AppImage, Nsis, WindowsMsi], Container=[]
/// - **Windows**: Native=[Nsis, WindowsMsi], Container=[Deb, Rpm, AppImage]
///
/// Note: macOS packages cannot be built in containers due to Apple licensing restrictions.
///
/// # Arguments
///
/// * `platforms` - Requested package types
///
/// # Returns
///
/// * `(native, containerized)` - Tuple of (platforms to build locally, platforms to build in Docker)
pub fn split_platforms_by_host(platforms: &[PackageType]) -> (Vec<PackageType>, Vec<PackageType>) {
    let mut native = Vec::new();
    let mut containerized = Vec::new();

    for &platform in platforms {
        if is_native_platform(platform) {
            native.push(platform);
        } else {
            containerized.push(platform);
        }
    }

    (native, containerized)
}

/// Checks if Wine is available for building Windows packages on Linux.
///
/// Returns true if `wine --version` executes successfully, false otherwise.
/// This enables runtime detection instead of compile-time assumptions.
///
/// # Examples
///
/// On Linux with Wine installed:
/// ```no_run
/// assert_eq!(has_wine(), true);
/// ```
///
/// On Linux without Wine or non-Linux systems:
/// ```no_run
/// assert_eq!(has_wine(), false);
/// ```
#[cfg(target_os = "linux")]
fn has_wine() -> bool {
    /// Cached result of Wine availability check.
    ///
    /// This static is initialized once on first access and reused thereafter,
    /// avoiding repeated process spawns for `wine --version` checks.
    static WINE_AVAILABLE: OnceLock<bool> = OnceLock::new();

    *WINE_AVAILABLE.get_or_init(|| {
        use std::process::Stdio;
        std::process::Command::new("wine")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Wine is not available on non-Linux platforms.
#[cfg(not(target_os = "linux"))]
fn has_wine() -> bool {
    false
}

/// Checks if a platform can be built natively on the current host OS.
///
/// Uses runtime OS detection via `std::env::consts::OS` instead of compile-time
/// cfg attributes. This enables dynamic capability checking (e.g., Wine availability).
///
/// # Platform Support
///
/// - **macOS**: MacOsBundle, Dmg (native only, cannot be built in containers)
/// - **Linux**: Deb, Rpm, AppImage (always native)
/// - **Linux + Wine**: Nsis, WindowsMsi (requires Wine at runtime)
/// - **Windows**: Nsis, WindowsMsi (native)
/// - **All others**: Require Docker container
///
/// # Returns
///
/// - `true` - Platform can be built natively on current OS
/// - `false` - Platform requires Docker container
fn is_native_platform(platform: PackageType) -> bool {
    use PackageType::*;

    match (std::env::consts::OS, platform) {
        // macOS native packages (cannot be built in Linux containers)
        ("macos", MacOsBundle | Dmg) => true,

        // Linux native packages
        ("linux", Deb | Rpm | AppImage) => true,

        // Linux with Wine can build Windows packages
        // Runtime check ensures Wine is actually installed
        ("linux", Nsis | WindowsMsi) => has_wine(),

        // Windows native packages
        ("windows", Nsis | WindowsMsi) => true,

        // Everything else needs Docker
        _ => false,
    }
}

/// Converts PackageType to string for CLI arguments.
pub fn platform_type_to_string(platform: PackageType) -> &'static str {
    match platform {
        PackageType::Deb => "deb",
        PackageType::Rpm => "rpm",
        PackageType::AppImage => "appimage",
        PackageType::MacOsBundle => "app",
        PackageType::Dmg => "dmg",
        PackageType::WindowsMsi => "msi",
        PackageType::Nsis => "nsis",
    }
}

/// Returns emoji for platform type (for pretty output).
pub fn platform_emoji(platform: PackageType) -> &'static str {
    match platform {
        PackageType::Deb | PackageType::Rpm | PackageType::AppImage => "🐧",
        PackageType::MacOsBundle | PackageType::Dmg => "🍎",
        PackageType::WindowsMsi | PackageType::Nsis => "🪟",
    }
}
