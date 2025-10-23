//! AppImage bundler - portable Linux applications.

use crate::bundler::{
    error::{bail, Context, ErrorExt, Result},
    settings::Settings,
    utils::http,
};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

const LINUXDEPLOY_BASE_URL: &str =
    "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous";

/// Bundle project as AppImage.
///
/// Creates a portable, self-contained AppImage executable that runs on any Linux distribution.
///
/// # Process
///
/// 1. Downloads linuxdeploy tool (cached in .tools/)
/// 2. Creates AppDir structure (usr/bin, usr/lib)
/// 3. Copies binaries and resources
/// 4. Generates .desktop file
/// 5. Invokes linuxdeploy to create AppImage
///
/// # Returns
///
/// Vector containing the path to the generated .AppImage file.
pub fn bundle_project(settings: &Settings) -> Result<Vec<PathBuf>> {
    // 1. Map architecture
    let arch = match settings.binary_arch() {
        crate::bundler::settings::Arch::X86_64 => "x86_64",
        crate::bundler::settings::Arch::X86 => "i386",
        crate::bundler::settings::Arch::AArch64 => "aarch64",
        _ => bail!(
            "Unsupported architecture for AppImage: {:?}",
            settings.binary_arch()
        ),
    };

    log::info!("Building AppImage for {}", settings.product_name());
    log::debug!("Using architecture: {}", arch);

    // 2. Setup directories
    let output_dir = settings.project_out_directory().join("bundle/appimage");
    let tools_dir = output_dir.join(".tools");

    std::fs::create_dir_all(&tools_dir).fs_context("creating tools directory", &tools_dir)?;

    // 3. Download linuxdeploy
    let linuxdeploy = download_linuxdeploy(&tools_dir, arch)
        .context("failed to download linuxdeploy tool")?;

    // 4. Create AppDir structure
    let app_dir = output_dir.join(format!("{}.AppDir", settings.product_name()));

    // Clean any existing AppDir
    if app_dir.exists() {
        std::fs::remove_dir_all(&app_dir).fs_context("removing old AppDir", &app_dir)?;
    }

    // Create directory structure
    let usr_dir = app_dir.join("usr");
    let bin_dir = usr_dir.join("bin");
    let lib_dir = usr_dir.join("lib");

    for dir in [&usr_dir, &bin_dir, &lib_dir] {
        std::fs::create_dir_all(dir).fs_context("creating AppDir structure", dir)?;
    }

    // 5. Copy binaries
    for binary in settings.binaries() {
        let src = settings.binary_path(binary);
        let dst = bin_dir.join(binary.name());

        std::fs::copy(&src, &dst).fs_context("copying binary", &dst)?;

        // Ensure executable permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755))?;
        }
    }

    // 6. Create desktop file
    create_desktop_file(settings, &app_dir)?;

    // 7. Copy icon (if available)
    if let Some(icon_paths) = &settings.bundle_settings().icon {
        // Find first PNG icon (AppImage requires PNG)
        if let Some(icon_path) = icon_paths
            .iter()
            .find(|p| p.extension().and_then(|e| e.to_str()) == Some("png"))
        {
            let icon_name = format!("{}.png", settings.product_name());
            let dst_icon = app_dir.join(&icon_name);

            std::fs::copy(icon_path, &dst_icon).fs_context("copying icon", &dst_icon)?;

            // Create .DirIcon symlink (required by AppImage spec)
            #[cfg(unix)]
            {
                let diricon_path = app_dir.join(".DirIcon");
                std::os::unix::fs::symlink(&icon_name, &diricon_path)?;
            }
        }
    }

    // 8. Invoke linuxdeploy
    let appimage_path = output_dir.join(format!(
        "{}-{}-{}.AppImage",
        settings.product_name(),
        settings.version_string(),
        arch
    ));

    let app_dir_str = app_dir
        .to_str()
        .context("AppDir path contains invalid UTF-8")?;

    let status = Command::new(&linuxdeploy)
        .env("OUTPUT", &appimage_path)
        .env("ARCH", arch)
        .args(["--appdir", app_dir_str, "--output", "appimage"])
        .status()
        .context("failed to execute linuxdeploy")?;

    if !status.success() {
        bail!("linuxdeploy failed with exit code: {:?}", status.code());
    }

    // 9. Set final permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&appimage_path, std::fs::Permissions::from_mode(0o755))?;
    }

    log::info!("✓ Created AppImage: {}", appimage_path.display());

    Ok(vec![appimage_path])
}

/// Download linuxdeploy tool.
///
/// Downloads the linuxdeploy AppImage from GitHub and caches it locally.
/// Returns early if the tool is already cached.
fn download_linuxdeploy(tools_dir: &Path, arch: &str) -> Result<PathBuf> {
    let tool_name = format!("linuxdeploy-{}.AppImage", arch);
    let tool_path = tools_dir.join(&tool_name);

    // Return early if already downloaded
    if tool_path.exists() {
        log::debug!("linuxdeploy already cached at {:?}", tool_path);
        return Ok(tool_path);
    }

    log::info!("Downloading linuxdeploy for {}...", arch);

    let url = format!("{}/{}", LINUXDEPLOY_BASE_URL, tool_name);
    let data = http::download(&url)?;

    std::fs::write(&tool_path, data).fs_context("writing linuxdeploy tool", &tool_path)?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tool_path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(tool_path)
}

/// Create .desktop file for the AppImage.
///
/// Generates a freedesktop.org compliant desktop entry with application metadata.
fn create_desktop_file(settings: &Settings, app_dir: &Path) -> Result<()> {
    let desktop_file = app_dir.join(format!("{}.desktop", settings.product_name()));
    let mut file =
        std::fs::File::create(&desktop_file).fs_context("creating desktop file", &desktop_file)?;

    writeln!(file, "[Desktop Entry]")?;
    writeln!(file, "Type=Application")?;
    writeln!(file, "Name={}", settings.product_name())?;

    // Find main binary name
    let main_binary = settings
        .binaries()
        .iter()
        .find(|b| b.main())
        .context("no main binary found")?;

    writeln!(file, "Exec={}", main_binary.name())?;
    writeln!(file, "Icon={}", settings.product_name())?;

    // Optional fields from bundle settings
    let bundle = settings.bundle_settings();

    if !settings.description().is_empty() {
        writeln!(file, "Comment={}", settings.description())?;
    }

    if let Some(category) = &bundle.category {
        writeln!(file, "Categories={}", category)?;
    }

    writeln!(file, "Terminal=false")?;

    file.flush()?;
    Ok(())
}
