//! Icon format conversion for multi-platform bundling.
//!
//! This module provides platform-agnostic utilities for loading, analyzing,
//! and selecting icon files for conversion to platform-specific formats.
//!
//! # Supported Formats
//!
//! The bundler accepts PNG icons and automatically converts to platform-specific formats:
//!
//! | Platform | Output Format | Sizes Required |
//! |----------|---------------|----------------|
//! | Linux | PNG (copied) | 32, 64, 128, 256, 512 |
//! | macOS | ICNS | 16, 32, 64, 128, 256, 512, 1024 |
//! | Windows | ICO | 16, 32, 48, 64, 128, 256 |
//!
//! # Configuration
//!
//! ```toml
//! [package.metadata.bundle]
//! icon = [
//!     "assets/icon-32.png",
//!     "assets/icon-128.png",
//!     "assets/icon-512.png"
//! ]
//! ```
//!
//! # Best Practices
//!
//! - Use square PNG images (1:1 aspect ratio)
//! - Provide multiple sizes for best rendering quality
//! - Use 32-bit RGBA format with transparency
//! - Keep icon design simple and recognizable at small sizes
//! - Test icons at all target sizes before release
//!
//! # Icon Selection Algorithm
//!
//! When selecting icons for specific target sizes, the module uses a heuristic:
//! 1. Exact size match (best)
//! 2. Nearest size (prefer larger over smaller for downscaling quality)
//! 3. Square icons over non-square (penalized by 10000 in scoring)

use crate::bundler::error::Result;
use std::path::{Path, PathBuf};

/// Icon metadata with dimensions.
///
/// Contains information about a loaded icon file including its path and dimensions.
/// Used by the icon selection algorithm to choose appropriate icons for target sizes.
#[derive(Debug, Clone)]
pub struct IconInfo {
    /// Path to the icon file.
    pub path: PathBuf,

    /// Icon width in pixels.
    pub width: u32,

    /// Icon height in pixels.
    pub height: u32,
}

impl IconInfo {
    /// Returns whether this icon is square (width == height).
    ///
    /// Square icons are strongly preferred as they render correctly on all platforms.
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }

    /// Calculates Manhattan distance from target size.
    ///
    /// Returns the sum of absolute differences in width and height from target.
    /// Used by the icon selection algorithm to find nearest-sized icons.
    ///
    /// # Arguments
    ///
    /// * `target` - Target size (assumes square target)
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::path::PathBuf;
    /// # #[derive(Debug, Clone)]
    /// # pub struct IconInfo {
    /// #     pub path: PathBuf,
    /// #     pub width: u32,
    /// #     pub height: u32,
    /// # }
    /// # impl IconInfo {
    /// #     pub fn size_diff(&self, target: u32) -> u32 {
    /// #         ((self.width as i32 - target as i32).abs() +
    /// #          (self.height as i32 - target as i32).abs()) as u32
    /// #     }
    /// # }
    /// let icon = IconInfo {
    ///     path: PathBuf::from("icon.png"),
    ///     width: 128,
    ///     height: 128,
    /// };
    /// let diff = icon.size_diff(256); // Returns 256 (128 from width + 128 from height)
    /// ```
    pub fn size_diff(&self, target: u32) -> u32 {
        ((self.width as i32 - target as i32).abs() + (self.height as i32 - target as i32).abs())
            as u32
    }
}

/// Loads icons from filesystem and extracts metadata.
///
/// Opens each icon file, reads its dimensions, and returns metadata for all
/// valid icons. Invalid or missing icons are logged as warnings and skipped.
///
/// # Arguments
///
/// * `icon_paths` - PNG file paths from bundle settings
///
/// # Returns
///
/// * `Ok(Vec<IconInfo>)` - Icon metadata for all valid icons
/// * `Err(IconPathError)` - If no valid icons were found
///
/// # Errors
///
/// Returns `IconPathError` if all provided paths are invalid or missing.
/// Individual icon failures are logged but don't fail the entire operation.
///
/// # Examples
///
/// ```no_run
/// # use std::path::PathBuf;
/// # type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// # #[derive(Debug, Clone)]
/// # struct IconInfo { path: PathBuf, width: u32, height: u32 }
/// # fn load_icons(icon_paths: &[PathBuf]) -> Result<Vec<IconInfo>> { Ok(vec![]) }
/// # fn example() -> Result<()> {
/// let paths = vec![
///     PathBuf::from("assets/icon-32.png"),
///     PathBuf::from("assets/icon-512.png")
/// ];
/// let icons = load_icons(&paths)?;
/// println!("Loaded {} icons", icons.len());
/// # Ok(())
/// # }
/// ```
pub fn load_icons(icon_paths: &[PathBuf]) -> Result<Vec<IconInfo>> {
    let mut icons = Vec::new();

    for path in icon_paths {
        if !path.exists() {
            log::warn!("Icon path does not exist: {}", path.display());
            continue;
        }

        // Open image to get dimensions
        let img = image::open(path).map_err(|e| crate::bundler::Error::Fs {
            context: "failed to open icon",
            path: path.clone(),
            error: std::io::Error::other(e),
        })?;

        icons.push(IconInfo {
            path: path.clone(),
            width: img.width(),
            height: img.height(),
        });

        log::debug!(
            "Loaded icon: {}x{} from {}",
            img.width(),
            img.height(),
            path.display()
        );
    }

    if icons.is_empty() {
        return Err(crate::bundler::Error::IconPathError);
    }

    Ok(icons)
}

/// Finds the best icon for a target size using heuristic selection.
///
/// Uses a scoring algorithm to select the most appropriate icon:
///
/// # Selection Algorithm
///
/// 1. **Exact size match** - Lowest score (best)
/// 2. **Nearest size** - Scored by Manhattan distance
/// 3. **Square preference** - Non-square icons penalized by 10000
///
/// Larger icons are preferred over smaller ones for downscaling quality.
///
/// # Arguments
///
/// * `icons` - Available icons to choose from
/// * `target_size` - Desired size (assumes square target)
///
/// # Returns
///
/// * `Some(&IconInfo)` - Best matching icon
/// * `None` - If icons slice is empty
///
/// # Examples
///
/// ```
/// # use std::path::PathBuf;
/// # #[derive(Debug, Clone)]
/// # pub struct IconInfo { pub path: PathBuf, pub width: u32, pub height: u32 }
/// # impl IconInfo {
/// #     fn size_diff(&self, target: u32) -> u32 {
/// #         ((self.width as i32 - target as i32).abs() +
/// #          (self.height as i32 - target as i32).abs()) as u32
/// #     }
/// #     fn is_square(&self) -> bool { self.width == self.height }
/// # }
/// # fn find_icon_for_size(icons: &[IconInfo], target_size: u32) -> Option<&IconInfo> {
/// #     icons.iter().min_by_key(|icon| {
/// #         let size_diff = icon.size_diff(target_size);
/// #         let square_penalty = if icon.is_square() { 0 } else { 10000 };
/// #         size_diff + square_penalty
/// #     })
/// # }
/// # fn example(icons: &[IconInfo]) {
/// if let Some(icon) = find_icon_for_size(icons, 256) {
///     println!("Selected {}x{} icon for 256x256 target", icon.width, icon.height);
/// }
/// # }
/// ```
pub fn find_icon_for_size(icons: &[IconInfo], target_size: u32) -> Option<&IconInfo> {
    icons.iter().min_by_key(|icon| {
        let size_diff = icon.size_diff(target_size);
        let square_penalty = if icon.is_square() { 0 } else { 10000 };
        size_diff + square_penalty
    })
}

/// Loads and resizes an icon to exact dimensions.
///
/// Opens the source PNG, resizes it to the specified dimensions using high-quality
/// Lanczos3 filtering, and returns an RGBA8 image buffer ready for platform-specific
/// encoding (ICNS, ICO, etc.).
///
/// # Arguments
///
/// * `source_path` - Path to source PNG file
/// * `target_width` - Desired width in pixels
/// * `target_height` - Desired height in pixels
///
/// # Returns
///
/// * `Ok(RgbaImage)` - RGBA8 image buffer at target dimensions
/// * `Err` - If file cannot be read or decoded
///
/// # Image Quality
///
/// Uses Lanczos3 filtering which provides the best quality for downscaling,
/// preserving sharp edges and minimizing artifacts.
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// # struct RgbaImage;
/// # fn load_and_resize(source_path: &Path, width: u32, height: u32) -> Result<RgbaImage> {
/// #     Ok(RgbaImage)
/// # }
/// # fn example() -> Result<()> {
/// let rgba_image = load_and_resize(
///     Path::new("icon-512.png"),
///     256,
///     256
/// )?;
/// // Use rgba_image for ICNS/ICO encoding
/// # Ok(())
/// # }
/// ```
pub fn load_and_resize(
    source_path: &Path,
    target_width: u32,
    target_height: u32,
) -> Result<image::RgbaImage> {
    let img = image::open(source_path).map_err(|e| crate::bundler::Error::Fs {
        context: "loading icon for resize",
        path: source_path.to_path_buf(),
        error: std::io::Error::other(e),
    })?;

    let resized = img.resize_exact(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3, // Best quality for downscaling
    );

    Ok(resized.to_rgba8())
}
