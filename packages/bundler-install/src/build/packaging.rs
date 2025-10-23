//! ZIP packaging and directory handling utilities
//!
//! This module provides ZIP creation and directory traversal functionality
//! for packaging the macOS helper app with zero allocation patterns and
//! blazing-fast performance.

use std::fs;
use std::io::Write;
use std::path::Path;

/// Create ZIP package for helper app embedding
pub fn create_helper_zip(
    helper_dir: &Path,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let zip_path = out_dir.join("KodegenHelper.app.zip");
    let file = fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // Add the helper app to the ZIP
    add_directory_to_zip(
        &mut zip,
        helper_dir,
        helper_dir.parent().unwrap_or(helper_dir),
        &options,
    )?;

    zip.finish()?;

    // Generate integrity hash
    generate_zip_hash(&zip_path)?;

    // Generate the include statement for the build
    let include_stmt = format!(
        "const APP_ZIP_DATA: &[u8] = include_bytes!(\"{}\");",
        zip_path.to_string_lossy()
    );

    let include_file = out_dir.join("app_zip_data.rs");
    fs::write(&include_file, include_stmt)?;

    println!("cargo:rustc-env=HELPER_ZIP_PATH={}", zip_path.display());
    println!(
        "cargo:rustc-env=HELPER_ZIP_INCLUDE_FILE={}",
        include_file.display()
    );

    Ok(())
}

/// Recursively add directory contents to ZIP archive
pub fn add_directory_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Path,
    base: &Path,
    options: &zip::write::FileOptions<'static, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(base)?;

        if path.is_dir() {
            // Add directory entry
            let dir_name = format!("{}/", relative_path.to_string_lossy());
            zip.add_directory(&dir_name, *options)?;

            // Recursively add directory contents
            add_directory_to_zip(zip, &path, base, options)?;
        } else {
            // Add file entry
            let mut file = fs::File::open(&path)?;
            zip.start_file(relative_path.to_string_lossy().as_ref(), *options)?;
            std::io::copy(&mut file, zip)?;
        }
    }
    Ok(())
}
/// Extract ZIP archive to directory
#[allow(dead_code)]
pub fn extract_zip(zip_path: &Path, extract_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    fs::create_dir_all(extract_dir)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = extract_dir.join(file.name());

        if file.name().ends_with('/') {
            // Directory
            fs::create_dir_all(&outpath)?;
        } else {
            // File
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;

            // Set permissions on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(())
}
/// Create functional ZIP with proper helper app and atomic rollback
pub fn create_functional_zip(zip_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Validate input paths before any operations
    if !zip_path.parent().is_some_and(std::path::Path::exists) {
        return Err(format!(
            "ZIP parent directory does not exist: {}",
            zip_path.parent().unwrap_or(zip_path).display()
        )
        .into());
    }

    let out_dir = zip_path
        .parent()
        .ok_or("Invalid zip path - no parent directory")?;
    let helper_dir = out_dir.join("KodegenHelper.app");

    // Create temporary working directory for atomic operations
    let temp_dir = out_dir.join("tmp_helper_build");
    let temp_zip = out_dir.join("tmp_helper.zip");

    // Cleanup function for atomic rollback
    let cleanup = || {
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::remove_file(&temp_zip);
        let _ = std::fs::remove_dir_all(&helper_dir);
    };

    // Ensure clean state before starting
    cleanup();

    // Create temporary directory with error handling
    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        cleanup();
        format!(
            "Failed to create temporary directory {}: {}",
            temp_dir.display(),
            e
        )
    })?;

    // Build the actual helper app first using existing infrastructure
    // Use temporary directory to ensure atomicity

    // Validate out_dir is writable before proceeding
    let test_file = out_dir.join("test_write_permissions");
    std::fs::write(&test_file, "test").map_err(|e| {
        cleanup();
        format!(
            "Output directory is not writable {}: {}",
            out_dir.display(),
            e
        )
    })?;
    std::fs::remove_file(&test_file).map_err(|e| {
        cleanup();
        format!("Failed to cleanup test file {}: {}", test_file.display(), e)
    })?;

    // Build helper app with comprehensive error handling
    super::macos_helper::build_and_sign_helper().map_err(|e| {
        cleanup();
        format!("Failed to build and sign helper: {e}")
    })?;

    // Validate the helper was built properly with atomic rollback
    if !helper_dir.exists() {
        cleanup();
        return Err("Helper directory was not created by build system".into());
    }

    super::macos_helper::validate_helper_structure(&helper_dir).map_err(|e| {
        cleanup();
        format!("Helper validation failed: {e}")
    })?;

    // Create ZIP with atomic operations - write to temp location first
    create_helper_zip(&helper_dir, &temp_dir).map_err(|e| {
        cleanup();
        format!("Failed to create helper ZIP: {e}")
    })?;

    let temp_zip_actual = temp_dir.join("KodegenHelper.app.zip");

    // Validate ZIP was created successfully
    if !temp_zip_actual.exists() {
        cleanup();
        return Err("ZIP file was not created successfully".into());
    }

    // Verify ZIP integrity before moving to final location
    verify_zip_integrity(&temp_zip_actual).map_err(|e| {
        cleanup();
        format!("ZIP integrity check failed: {e}")
    })?;

    // Atomic move to final location (last step that can fail)
    std::fs::rename(&temp_zip_actual, zip_path).map_err(|e| {
        cleanup();
        format!(
            "Failed to move ZIP to final location {}: {}",
            zip_path.display(),
            e
        )
    })?;

    // Success - cleanup temporary files but keep final artifacts
    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(())
}

/// Verify ZIP integrity and structure
fn verify_zip_integrity(zip_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Check for required macOS app bundle structure
    let required_files = [
        "KodegenHelper.app/Contents/Info.plist",
        "KodegenHelper.app/Contents/MacOS/KodegenHelper",
    ];

    for required_file in &required_files {
        let mut found = false;
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            if file.name() == *required_file {
                found = true;
                break;
            }
        }
        if !found {
            return Err(format!("Required file {required_file} not found in ZIP").into());
        }
    }

    // Verify we can read all files without corruption
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let mut buffer = Vec::new();
        std::io::copy(&mut file, &mut buffer)?;

        // Basic sanity check - files should not be empty for critical components
        if file.name().ends_with("KodegenHelper") && buffer.is_empty() {
            return Err(format!("Critical executable {} is empty", file.name()).into());
        }

        if file.name().ends_with("Info.plist") && buffer.len() < 100 {
            return Err(format!("Info.plist {} is suspiciously small", file.name()).into());
        }
    }

    Ok(())
}

/// Generate integrity hash for ZIP file
fn generate_zip_hash(zip_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};

    let zip_data = fs::read(zip_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&zip_data);
    let hash = hasher.finalize();

    let hash_hex = hex::encode(hash);
    let hash_path = zip_path.with_extension("zip.sha256");

    fs::write(&hash_path, &hash_hex)?;

    println!("cargo:rustc-env=MACOS_HELPER_ZIP_HASH={hash_hex}");

    Ok(())
}
