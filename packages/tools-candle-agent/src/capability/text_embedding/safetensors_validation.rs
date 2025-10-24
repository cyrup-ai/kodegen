//! SafeTensors file validation before unsafe memory mapping
//!
//! Provides validation to prevent crashes from corrupted or malicious SafeTensors files.

use anyhow::{Context, Result, bail};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Maximum reasonable size for JSON metadata (100MB)
const MAX_JSON_LENGTH: u64 = 100_000_000;

/// Minimum valid SafeTensors file size (8-byte header + minimal JSON)
const MIN_FILE_SIZE: u64 = 20;

/// Validate SafeTensors file format before unsafe mmap
///
/// Performs basic integrity checks:
/// - File exists and is readable
/// - Minimum file size met
/// - Header contains valid JSON length
/// - JSON length is reasonable (< 100MB)
/// - Total file size is consistent
///
/// # Arguments
/// * `path` - Path to SafeTensors file
///
/// # Returns
/// * `Ok(())` if file passes validation
/// * `Err` with detailed error message if validation fails
///
/// # Example
/// ```rust,ignore
/// validate_safetensors_file(&weights_path)?;
/// let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], dtype, device)? };
/// ```
pub fn validate_safetensors_file(path: &Path) -> Result<()> {
    // Check file exists and get metadata
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Cannot access SafeTensors file '{}'", path.display()))?;

    // Check minimum file size
    let file_size = metadata.len();
    if file_size < MIN_FILE_SIZE {
        bail!(
            "SafeTensors file '{}' too small: {} bytes (minimum {})",
            path.display(),
            file_size,
            MIN_FILE_SIZE
        );
    }

    // Open file and read 8-byte header
    let mut file = File::open(path)
        .with_context(|| format!("Cannot open SafeTensors file '{}'", path.display()))?;

    let mut header = [0u8; 8];
    file.read_exact(&mut header)
        .with_context(|| format!("Cannot read SafeTensors header from '{}'", path.display()))?;

    // Parse header: u64 little-endian JSON length
    let json_len = u64::from_le_bytes(header);

    // Validate JSON length is reasonable
    if json_len == 0 {
        bail!(
            "SafeTensors file '{}' has zero-length JSON metadata",
            path.display()
        );
    }

    if json_len > MAX_JSON_LENGTH {
        bail!(
            "SafeTensors file '{}' has suspiciously large JSON metadata: {} bytes (max {})",
            path.display(),
            json_len,
            MAX_JSON_LENGTH
        );
    }

    // Validate total file size is consistent
    let expected_min_size = 8 + json_len; // header + JSON (tensor data follows)
    if file_size < expected_min_size {
        bail!(
            "SafeTensors file '{}' truncated: {} bytes total, expected at least {} (header + JSON)",
            path.display(),
            file_size,
            expected_min_size
        );
    }

    // Optional: Read and parse JSON metadata
    // This catches JSON syntax errors before mmap
    let mut json_bytes = vec![0u8; json_len as usize];
    file.read_exact(&mut json_bytes)
        .with_context(|| format!("Cannot read JSON metadata from '{}'", path.display()))?;

    // Validate JSON is parseable
    serde_json::from_slice::<serde_json::Value>(&json_bytes).with_context(|| {
        format!(
            "SafeTensors file '{}' has invalid JSON metadata",
            path.display()
        )
    })?;

    Ok(())
}

/// Validate multiple SafeTensors files (convenience wrapper)
pub fn validate_safetensors_files(paths: &[impl AsRef<Path>]) -> Result<()> {
    for path in paths {
        validate_safetensors_file(path.as_ref())?;
    }
    Ok(())
}
