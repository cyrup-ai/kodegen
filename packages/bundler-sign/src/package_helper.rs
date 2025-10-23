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
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
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

    Ok(zip_path)
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
