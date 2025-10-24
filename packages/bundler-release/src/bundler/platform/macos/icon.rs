//! ICNS icon file creation for macOS applications.

#![cfg(target_os = "macos")]

use crate::bundler::error::{ErrorExt, Result};
use crate::bundler::resources::icons::{IconInfo, find_icon_for_size, load_and_resize};
use icns::{IconFamily, IconType, Image as IconsImage};
use std::path::Path;

/// Create ICNS file from source icons using unified infrastructure
pub fn create_icns_file(icons: &[IconInfo], output: &Path) -> Result<()> {
    let mut family = IconFamily::new();

    let icon_types = [
        (IconType::RGBA32_16x16, 16, "16x16"),
        (IconType::RGBA32_16x16_2x, 32, "16x16@2x"),
        (IconType::RGBA32_32x32, 32, "32x32"),
        (IconType::RGBA32_32x32_2x, 64, "32x32@2x"),
        (IconType::RGBA32_64x64, 64, "64x64"),
        (IconType::RGBA32_128x128, 128, "128x128"),
        (IconType::RGBA32_128x128_2x, 256, "128x128@2x"),
        (IconType::RGBA32_256x256, 256, "256x256"),
        (IconType::RGBA32_256x256_2x, 512, "256x256@2x"),
        (IconType::RGBA32_512x512, 512, "512x512"),
        (IconType::RGBA32_512x512_2x, 1024, "512x512@2x"),
    ];

    for (icon_type, size, name) in icon_types {
        if let Some(icon_info) = find_icon_for_size(icons, size) {
            log::debug!("Adding {} from {}", name, icon_info.path.display());

            let rgba = load_and_resize(&icon_info.path, size, size)?;

            let icns_img =
                IconsImage::from_data(icns::PixelFormat::RGBA, size, size, rgba.into_raw())
                    .map_err(|e| {
                        crate::bundler::Error::GenericError(format!(
                            "creating ICNS image for {}: {}",
                            name, e
                        ))
                    })?;

            family
                .add_icon_with_type(&icns_img, icon_type)
                .map_err(|e| {
                    crate::bundler::Error::GenericError(format!(
                        "adding {} to icon family: {}",
                        name, e
                    ))
                })?;
        } else {
            log::warn!("No suitable source icon for {}", name);
        }
    }

    let file = std::fs::File::create(output).fs_context("creating ICNS output file", output)?;
    family
        .write(file)
        .map_err(|e| crate::bundler::Error::GenericError(format!("writing ICNS data: {}", e)))?;

    log::info!("Created ICNS file: {}", output.display());
    Ok(())
}
