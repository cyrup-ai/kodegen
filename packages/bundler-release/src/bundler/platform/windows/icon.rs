//! ICO icon file creation for Windows applications.
//!
//! Converts PNG source images to ICO format with multiple sizes
//! for different Windows contexts (taskbar, alt-tab, etc.).

#![cfg(windows)]

use crate::bundler::error::{ErrorExt, Result};
use crate::bundler::resources::icons::{IconInfo, find_icon_for_size, load_and_resize};
use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use std::path::Path;

/// Create ICO file from source icons
///
/// Generates standard Windows icon sizes used in various contexts:
/// - 16x16: Small icons (window title bars)
/// - 24x24: Small toolbar icons
/// - 32x32: Standard icons (Windows Explorer)
/// - 48x48: Large icons (Windows Explorer large view)
/// - 64x64, 128x128: Extra large icons
/// - 256x256: Windows Vista+ high-res icons
///
/// # Example
/// ```rust
/// let icons = load_icons(&icon_paths)?;
/// create_ico_file(&icons, Path::new("app.ico"))?;
/// ```
pub fn create_ico_file(icons: &[IconInfo], output: &Path) -> Result<()> {
    let mut icon_dir = IconDir::new(ResourceType::Icon);

    // Windows standard icon sizes
    let sizes = [16, 24, 32, 48, 64, 128, 256];

    for size in sizes {
        if let Some(icon_info) = find_icon_for_size(icons, size) {
            log::debug!("Adding {}x{} from {}", size, size, icon_info.path.display());

            // Load and resize to exact dimensions
            let rgba = load_and_resize(&icon_info.path, size, size)?;

            // Create ICO image from RGBA data
            let icon_image = IconImage::from_rgba_data(size, size, rgba.into_raw());

            // Encode and add to directory
            let entry = IconDirEntry::encode(&icon_image).map_err(|e| {
                crate::bundler::Error::GenericError(format!(
                    "encoding {}x{} icon: {}",
                    size, size, e
                ))
            })?;
            icon_dir.add_entry(entry);
        } else {
            log::warn!("No suitable source icon for {}x{}", size, size);
        }
    }

    // Write ICO file
    let file = std::fs::File::create(output).fs_context("creating ICO output file", output)?;
    icon_dir
        .write(file)
        .map_err(|e| crate::bundler::Error::GenericError(format!("writing ICO data: {}", e)))?;

    log::info!("Created ICO file: {}", output.display());
    Ok(())
}
