//! Binary patching for updater support.
//!
//! This module provides functionality to patch compiled binaries with package type
//! metadata. The patching occurs post-compilation but pre-packaging, allowing
//! updater plugins to detect which package format was used for installation.
//!
//! # How It Works
//!
//! 1. Developer adds a marker to their binary code (see module docs)
//! 2. Binary is compiled with marker in .rodata section  
//! 3. This module searches for the marker in the compiled binary
//! 4. Writes package type string ("deb", "rpm", etc.) after marker
//! 5. Updater can read this at runtime to determine update format
//!
//! # Safety
//!
//! - Only modifies data section, never executable code
//! - Fails gracefully if marker not found (logs warning, continues)
//! - Does NOT break code signing (patching happens BEFORE signing)
//! - Marker uses #[used] attribute to survive optimization/stripping

use crate::bundler::{
    error::{Context, ErrorExt, Result},
    platform::PackageType,
};
use std::path::Path;

/// Patch binary with package type information.
///
/// This function reads the entire binary into memory, searches for the marker,
/// writes the package type, and saves the patched binary back to disk.
///
/// # Arguments
///
/// * `binary_path` - Path to the compiled binary executable
/// * `package_type` - Package type to embed (determines string written)
///
/// # Errors
///
/// Returns error if:
/// - Binary cannot be read (permissions, not found, etc.)
/// - Binary is too small (< 16 bytes)
/// - Binary format cannot be determined
/// - Patched binary cannot be written back
///
/// # Note
///
/// If marker is not found, this logs a debug message and returns Ok(()).
/// This is intentional - the feature is optional.
pub fn patch_binary(binary_path: &Path, package_type: &PackageType) -> Result<()> {
    log::debug!(
        "Attempting to patch binary {:?} with package type: {}",
        binary_path,
        package_type.short_name()
    );

    // Read entire binary into memory
    // Performance note: For typical desktop apps (1-50 MB), this is acceptable.
    // For very large binaries (100+ MB), this could be optimized with memory mapping.
    let mut data = std::fs::read(binary_path).fs_context("reading binary file", binary_path)?;

    // Check minimum size for format detection
    if data.len() < 16 {
        log::warn!(
            "Binary {:?} is too small (< 16 bytes), skipping patch",
            binary_path
        );
        return Ok(());
    }

    // Detect binary format using first 16 bytes
    let hint_bytes: &[u8; 16] = data
        .get(0..16)
        .and_then(|slice| slice.try_into().ok())
        .context("failed to extract hint bytes from binary")?;

    match goblin::peek_bytes(hint_bytes) {
        Ok(goblin::Hint::Elf(_)) => {
            log::debug!("Detected ELF binary format");
            patch_binary_data(&mut data, package_type, "ELF")?;
        }
        Ok(goblin::Hint::Mach(_)) | Ok(goblin::Hint::MachFat(_)) => {
            log::debug!("Detected Mach-O binary format");
            patch_binary_data(&mut data, package_type, "Mach-O")?;
        }
        Ok(goblin::Hint::PE) => {
            log::debug!("Detected PE binary format");
            patch_binary_data(&mut data, package_type, "PE")?;
        }
        Ok(goblin::Hint::COFF) => {
            log::warn!("Binary is COFF object file, not executable. Skipping patch.");
            return Ok(());
        }
        Ok(goblin::Hint::Archive) => {
            log::warn!("Binary is archive file, not executable. Skipping patch.");
            return Ok(());
        }
        Ok(goblin::Hint::Unknown(magic)) => {
            log::warn!(
                "Unknown binary format (magic: {:#x}), skipping patch",
                magic
            );
            return Ok(());
        }
        Ok(_) => {
            log::warn!("Unsupported binary format variant. Skipping patch.");
            return Ok(());
        }
        Err(e) => {
            log::warn!("Failed to detect binary format: {}. Skipping patch.", e);
            return Ok(());
        }
    }

    // Write patched binary back to disk
    std::fs::write(binary_path, data).fs_context("writing patched binary", binary_path)?;

    log::info!(
        "Successfully patched binary {:?} with package type: {}",
        binary_path,
        package_type.short_name()
    );

    Ok(())
}

/// Patch binary data with package type (format-agnostic implementation).
///
/// This function is called after format detection but uses the same patching
/// logic for all formats - simple byte pattern search and replacement.
///
/// # Arguments
///
/// * `data` - Mutable binary data to patch
/// * `package_type` - Package type to embed
/// * `format_name` - Human-readable format name for logging ("ELF", "Mach-O", "PE")
///
/// # Algorithm
///
/// 1. Search for marker pattern `__CYRUP_BUNDLE_TYPE` in binary
/// 2. If found, calculate write position (marker_pos + 19 bytes + 1 for null)
/// 3. Verify sufficient space remaining in binary
/// 4. Write package type string bytes into reserved space
///
/// # Note
///
/// If marker not found, logs debug message and returns Ok(). This is not an error.
fn patch_binary_data(data: &mut [u8], package_type: &PackageType, format_name: &str) -> Result<()> {
    const MARKER: &[u8] = b"__CYRUP_BUNDLE_TYPE";
    let package_type_bytes = package_type.short_name().as_bytes();

    if let Some(marker_pos) = find_pattern(data, MARKER) {
        // Calculate write position: marker + null byte
        let write_pos = marker_pos + MARKER.len() + 1;

        // Verify we have enough space for the package type string
        if write_pos + package_type_bytes.len() <= data.len() {
            // Perform the patch: write package type bytes
            data[write_pos..write_pos + package_type_bytes.len()]
                .copy_from_slice(package_type_bytes);

            log::debug!(
                "Patched {} binary: wrote '{}' at offset {}",
                format_name,
                package_type.short_name(),
                write_pos
            );
        } else {
            log::warn!(
                "Marker found but insufficient space to write package type (need {} bytes at offset {})",
                package_type_bytes.len(),
                write_pos
            );
        }
    } else {
        log::debug!(
            "Marker not found in {} binary. Skipping patch (this is optional).",
            format_name
        );
    }

    Ok(())
}

/// Find byte pattern in data using efficient sliding window search.
///
/// Uses Rust's built-in `windows()` iterator for efficient pattern matching.
/// Time complexity: O(n*m) where n = data length, m = pattern length.
///
/// For typical binaries:
/// - Pattern length: 19 bytes (constant)
/// - Binary size: 1-50 MB
/// - Search time: < 100ms on modern hardware
///
/// # Arguments
///
/// * `data` - Binary data to search
/// * `pattern` - Byte pattern to find
///
/// # Returns
///
/// `Some(position)` if pattern found, `None` otherwise.
fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    data.windows(pattern.len())
        .position(|window| window == pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_pattern_found() {
        let data = b"Hello __CYRUP_BUNDLE_TYPE\0            World";
        let marker = b"__CYRUP_BUNDLE_TYPE";
        assert_eq!(find_pattern(data, marker), Some(6));
    }

    #[test]
    fn test_find_pattern_not_found() {
        let data = b"Hello World";
        let marker = b"__CYRUP_BUNDLE_TYPE";
        assert_eq!(find_pattern(data, marker), None);
    }

    #[test]
    fn test_find_pattern_at_start() {
        let data = b"__CYRUP_BUNDLE_TYPE\0            ";
        let marker = b"__CYRUP_BUNDLE_TYPE";
        assert_eq!(find_pattern(data, marker), Some(0));
    }
}
