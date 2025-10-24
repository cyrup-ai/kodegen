//! HTTP utilities for downloading bundler tools.
//!
//! Provides functions for downloading files with hash verification
//! and extracting ZIP archives.

#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::bundler::error::Result;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::bundler::error::Error;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::Path,
};

/// Hash algorithm for verification.
#[derive(Debug, Clone, Copy)]
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub enum HashAlgorithm {
    /// SHA-1 hashing algorithm
    Sha1,
    /// SHA-256 hashing algorithm
    Sha256,
}

/// Downloads a file from a URL.
///
/// Returns the file contents as a byte vector.
///
/// Used by:
/// - Linux: AppImage bundler (downloads linuxdeploy tool)
/// - Windows: MSI/NSIS bundlers (via download_and_verify for WiX/NSIS downloads)
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn download(url: &str) -> Result<Vec<u8>> {
    log::info!("Downloading {}", url);

    let response = ureq::get(url).call().map_err(Box::new)?;

    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;

    Ok(bytes)
}

/// Downloads a file and verifies its hash.
///
/// Returns the file contents if the hash matches, otherwise returns an error.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn download_and_verify(
    url: &str,
    expected_hash: &str,
    algorithm: HashAlgorithm,
) -> Result<Vec<u8>> {
    let data = download(url)?;
    log::info!("validating hash");
    verify_hash(&data, expected_hash, algorithm)?;
    Ok(data)
}

/// Verifies that data matches the expected hash.
///
/// Compares the hash case-insensitively. Returns an error if the hashes don't match.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn verify_hash(data: &[u8], expected_hash: &str, algorithm: HashAlgorithm) -> Result<()> {
    use sha1::Digest as _;
    use sha2::Digest as _;

    let actual_hash = match algorithm {
        HashAlgorithm::Sha1 => {
            let mut hasher = sha1::Sha1::new();
            hasher.update(data);
            hex::encode(hasher.finalize())
        }
        HashAlgorithm::Sha256 => {
            let mut hasher = sha2::Sha256::new();
            hasher.update(data);
            hex::encode(hasher.finalize())
        }
    };

    if actual_hash.eq_ignore_ascii_case(expected_hash) {
        Ok(())
    } else {
        Err(Error::HashMismatch {
            expected: expected_hash.to_string(),
            actual: actual_hash,
        })
    }
}

/// Extracts a ZIP archive from memory into a destination directory.
///
/// Creates parent directories as needed and handles both files and directories in the archive.
/// Uses `enclosed_name()` to prevent path traversal attacks.
///
/// Used by:
/// - Windows: MSI bundler (extracts WiX toolset)
/// - Windows: NSIS bundler (extracts NSIS toolset)
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn extract_zip(data: &[u8], dest: &Path) -> Result<()> {
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        if let Some(name) = file.enclosed_name() {
            let dest_path = dest.join(name);

            if file.is_dir() {
                fs::create_dir_all(&dest_path)?;
                continue;
            }

            if let Some(parent) = dest_path.parent()
                && !parent.exists()
            {
                fs::create_dir_all(parent)?;
            }

            let mut buff = Vec::new();
            file.read_to_end(&mut buff)?;

            let mut fileout = File::create(dest_path)?;
            fileout.write_all(&buff)?;
        }
    }

    Ok(())
}
