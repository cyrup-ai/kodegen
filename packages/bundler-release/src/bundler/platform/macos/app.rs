//! macOS application bundle (.app) creation.

use crate::bundler::{
    error::{Context, ErrorExt, Result},
    settings::Settings,
    utils::fs,
};
use std::{
    fs as std_fs,
    path::{Path, PathBuf},
};

/// Bundles the project as a macOS .app bundle.
///
/// Creates the bundle structure with Info.plist, binaries, resources, and optional frameworks.
/// Returns a vector containing the path to the created .app bundle.
pub fn bundle_project(settings: &Settings) -> Result<Vec<PathBuf>> {
    let app_name = format!("{}.app", settings.product_name());
    let app_bundle_path = settings
        .project_out_directory()
        .join("bundle/macos")
        .join(&app_name);

    log::info!("Bundling {} at {}", app_name, app_bundle_path.display());

    // Remove old bundle if it exists
    if app_bundle_path.exists() {
        std_fs::remove_dir_all(&app_bundle_path)
            .fs_context("failed to remove old app bundle", &app_bundle_path)?;
    }

    // Create bundle directory structure
    let contents_dir = app_bundle_path.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    let resources_dir = contents_dir.join("Resources");

    std_fs::create_dir_all(&macos_dir)
        .fs_context("failed to create MacOS directory", &macos_dir)?;
    std_fs::create_dir_all(&resources_dir)
        .fs_context("failed to create Resources directory", &resources_dir)?;

    // Create icon file - use pre-made ICNS if available, otherwise convert from PNGs
    let icon_filename = format!("{}.icns", settings.product_name());
    let icon_path = resources_dir.join(&icon_filename);

    // Check for pre-made ICNS file in workspace assets
    let premade_icns = settings
        .project_out_directory()
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("assets/img/kodegen.icns"));

    if let Some(ref premade_path) = premade_icns {
        if premade_path.exists() {
            // Use pre-made ICNS file
            std_fs::copy(premade_path, &icon_path)
                .fs_context("failed to copy pre-made ICNS", &icon_path)?;
        } else {
            // Fall back to converting from PNGs
            let icons = settings.icon_files().context("failed to load icon files")?;
            super::icon::create_icns_file(&icons, &icon_path)
                .context("failed to create app icon")?;
        }
    } else {
        // Fall back to converting from PNGs
        let icons = settings.icon_files().context("failed to load icon files")?;
        super::icon::create_icns_file(&icons, &icon_path).context("failed to create app icon")?;
    }

    // Create Info.plist
    create_info_plist(&contents_dir, Some(&icon_path), settings)?;

    // Copy frameworks if configured
    copy_frameworks(&contents_dir, settings)?;

    // Copy binaries and set executable permissions
    copy_binaries(&macos_dir, settings)?;

    // Copy custom files
    copy_custom_files(&contents_dir, settings)?;

    // Sign if configured
    if settings.bundle_settings().macos.signing_identity.is_some() {
        super::sign::sign_app(&app_bundle_path, settings)?;
    }

    // Notarize if configured and credentials available
    if super::sign::should_notarize(settings) {
        super::sign::notarize_app(&app_bundle_path, settings)?;
    }

    Ok(vec![app_bundle_path])
}

/// Creates the Info.plist file for the macOS bundle
fn create_info_plist(
    contents_dir: &Path,
    icon_path: Option<&PathBuf>,
    settings: &Settings,
) -> Result<()> {
    use plist::Value;

    let mut dict = plist::Dictionary::new();

    // Required bundle metadata
    dict.insert("CFBundleDevelopmentRegion".into(), "English".into());
    dict.insert("CFBundleDisplayName".into(), settings.product_name().into());
    dict.insert(
        "CFBundleExecutable".into(),
        main_binary_name(settings)?.into(),
    );
    dict.insert(
        "CFBundleIdentifier".into(),
        bundle_identifier(settings)?.into(),
    );
    dict.insert("CFBundleName".into(), settings.product_name().into());
    dict.insert("CFBundlePackageType".into(), "APPL".into());
    dict.insert(
        "CFBundleShortVersionString".into(),
        settings.version_string().into(),
    );
    dict.insert("CFBundleVersion".into(), settings.version_string().into());
    dict.insert("CFBundleInfoDictionaryVersion".into(), "6.0".into());

    // Icon file reference
    if let Some(icon) = icon_path
        && let Some(filename) = icon.file_name()
    {
        dict.insert(
            "CFBundleIconFile".into(),
            filename.to_string_lossy().into_owned().into(),
        );
    }

    // Application category
    if let Some(category) = settings.bundle_settings().category.as_ref() {
        dict.insert("LSApplicationCategoryType".into(), category.clone().into());
    }

    // Minimum macOS version
    if let Some(version) = settings
        .bundle_settings()
        .macos
        .minimum_system_version
        .as_ref()
    {
        dict.insert("LSMinimumSystemVersion".into(), version.clone().into());
    }

    // Enable high resolution support
    dict.insert("NSHighResolutionCapable".into(), true.into());

    // Copyright notice
    if let Some(copyright) = settings.bundle_settings().copyright.as_ref() {
        dict.insert("NSHumanReadableCopyright".into(), copyright.clone().into());
    }

    // Write the plist to disk
    let plist_path = contents_dir.join("Info.plist");
    Value::Dictionary(dict)
        .to_file_xml(&plist_path)
        .map_err(crate::bundler::error::Error::Plist)?;

    Ok(())
}

/// Copies binaries to the MacOS directory and sets executable permissions
fn copy_binaries(macos_dir: &Path, settings: &Settings) -> Result<()> {
    // Get Resources directory for bundled binaries
    let resources_dir = macos_dir
        .parent()
        .ok_or_else(|| {
            crate::bundler::error::Error::GenericError("Invalid MacOS directory path".into())
        })?
        .join("Resources");

    for binary in settings.binaries() {
        let src = settings.binary_path(binary);
        let bin_name = binary.name();

        // Main binary (kodegen_install) goes in MacOS/ - launchable by user/system
        // Other binaries (kodegen, kodegend) go in Resources/ - extracted during install
        let dst = if binary.main() {
            macos_dir.join(bin_name)
        } else {
            resources_dir.join(bin_name)
        };

        fs::copy_file(&src, &dst)
            .with_context(|| format!("failed to copy {} to .app bundle", bin_name))?;

        // Set executable permissions on all binaries
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std_fs::set_permissions(&dst, std_fs::Permissions::from_mode(0o755))
                .fs_context("failed to set executable permissions", &dst)?;
        }
    }
    Ok(())
}

/// Copies frameworks to the Frameworks directory
fn copy_frameworks(contents_dir: &Path, settings: &Settings) -> Result<()> {
    let frameworks = match &settings.bundle_settings().macos.frameworks {
        Some(f) if !f.is_empty() => f,
        _ => return Ok(()), // No frameworks to copy
    };

    let frameworks_dir = contents_dir.join("Frameworks");
    std_fs::create_dir_all(&frameworks_dir)
        .fs_context("failed to create Frameworks directory", &frameworks_dir)?;

    for framework in frameworks {
        if framework.ends_with(".framework") {
            // Copy .framework bundle
            let src = PathBuf::from(framework);
            let name = src.file_name().ok_or_else(|| {
                crate::bundler::error::Error::GenericError(format!(
                    "Invalid framework path: {}",
                    framework
                ))
            })?;
            let dst = frameworks_dir.join(name);
            fs::copy_dir(&src, &dst)
                .with_context(|| format!("failed to copy framework {}", framework))?;
        } else if framework.ends_with(".dylib") {
            // Copy .dylib file
            let src = PathBuf::from(framework);
            let name = src.file_name().ok_or_else(|| {
                crate::bundler::error::Error::GenericError(format!(
                    "Invalid dylib path: {}",
                    framework
                ))
            })?;
            let dst = frameworks_dir.join(name);
            fs::copy_file(&src, &dst)
                .with_context(|| format!("failed to copy dylib {}", framework))?;
        } else {
            // Search standard framework locations
            copy_framework_from_standard_locations(&frameworks_dir, framework)?;
        }
    }
    Ok(())
}

/// Searches for a framework in standard macOS locations
fn copy_framework_from_standard_locations(dest_dir: &Path, framework: &str) -> Result<()> {
    let framework_name = format!("{}.framework", framework);

    // Search paths in order of preference
    let mut search_paths = vec![
        PathBuf::from("/Library/Frameworks"),
        PathBuf::from("/Network/Library/Frameworks"),
    ];

    // Add user's home directory if available
    if let Ok(home) = std::env::var("HOME") {
        search_paths.insert(0, PathBuf::from(home).join("Library/Frameworks"));
    }

    for search_path in search_paths {
        let src = search_path.join(&framework_name);
        if src.exists() {
            let dst = dest_dir.join(&framework_name);
            fs::copy_dir(&src, &dst)?;
            return Ok(());
        }
    }

    Err(crate::bundler::error::Error::GenericError(format!(
        "Framework not found: {}",
        framework
    )))
}

/// Copies custom files to the bundle
fn copy_custom_files(contents_dir: &Path, settings: &Settings) -> Result<()> {
    for (dest_path, src_path) in &settings.bundle_settings().macos.files {
        // Strip leading slash if absolute path
        let dest_path = if dest_path.is_absolute() {
            dest_path.strip_prefix("/").map_err(|e| {
                crate::bundler::error::Error::GenericError(format!(
                    "Failed to strip prefix from path: {}",
                    e
                ))
            })?
        } else {
            dest_path
        };

        let full_dest = contents_dir.join(dest_path);

        if src_path.is_file() {
            fs::copy_file(src_path, &full_dest).with_context(|| {
                format!("failed to copy file {:?} to {:?}", src_path, dest_path)
            })?;
        } else if src_path.is_dir() {
            fs::copy_dir(src_path, &full_dest).with_context(|| {
                format!("failed to copy directory {:?} to {:?}", src_path, dest_path)
            })?;
        } else {
            return Err(crate::bundler::error::Error::GenericError(format!(
                "{:?} is not a file or directory",
                src_path
            )));
        }
    }
    Ok(())
}

/// Returns the name of the main binary
fn main_binary_name(settings: &Settings) -> Result<&str> {
    settings
        .binaries()
        .iter()
        .find(|b| b.main())
        .map(|b| b.name())
        .ok_or_else(|| crate::bundler::error::Error::GenericError("No main binary found".into()))
}

/// Returns the bundle identifier
fn bundle_identifier(settings: &Settings) -> Result<&str> {
    settings
        .bundle_settings()
        .identifier
        .as_deref()
        .ok_or_else(|| {
            crate::bundler::error::Error::GenericError(
                "Bundle identifier required for macOS bundles".into(),
            )
        })
}
