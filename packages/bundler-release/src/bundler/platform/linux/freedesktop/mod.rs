//! FreeDesktop.org desktop entry file generation.
//!
//! This module handles creation of .desktop files for Linux applications.

use crate::bundler::error::{ErrorExt, Result};
use crate::bundler::resources::icons::{IconInfo, find_icon_for_size, load_and_resize};
use std::path::{Path, PathBuf};

/// Copy icons to freedesktop.org standard locations
///
/// Creates directory structure:
/// ```
/// /usr/share/icons/hicolor/
///   ├── 16x16/apps/{app_name}.png
///   ├── 32x32/apps/{app_name}.png
///   ├── 48x48/apps/{app_name}.png
///   ├── 128x128/apps/{app_name}.png
///   └── 256x256/apps/{app_name}.png
/// ```
///
/// Used by Debian, RPM, and AppImage builders.
pub fn install_icons(icons: &[IconInfo], dest_dir: &Path, app_name: &str) -> Result<Vec<PathBuf>> {
    let mut installed = Vec::new();

    // Freedesktop.org standard icon sizes
    let sizes = [16, 24, 32, 48, 64, 128, 256, 512];

    for size in sizes {
        if let Some(icon_info) = find_icon_for_size(icons, size) {
            let size_dir = dest_dir
                .join("usr/share/icons/hicolor")
                .join(format!("{}x{}", size, size))
                .join("apps");

            std::fs::create_dir_all(&size_dir)
                .fs_context("creating icon size directory", &size_dir)?;

            let dest = size_dir.join(format!("{}.png", app_name));

            // Resize and save as PNG
            let rgba = load_and_resize(&icon_info.path, size, size)?;
            let img = image::DynamicImage::ImageRgba8(rgba);
            img.save(&dest).fs_context("saving icon", &dest)?;

            installed.push(dest);
            log::debug!("Installed {}x{} icon to {}", size, size, dest.display());
        }
    }

    Ok(installed)
}
