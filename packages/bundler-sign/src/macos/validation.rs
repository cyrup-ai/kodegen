//! Path validation, dependency checks, and keychain validation

use crate::error::{Result, SetupError};
use crate::warn;
use jsonwebtoken::EncodingKey;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use termcolor::WriteColor;

/// Expand tilde in path, returning error if HOME is not set
///
/// This function wraps `shellexpand::tilde()` with proper error handling.
/// If the path starts with `~` and expansion fails (HOME not set), returns
/// a clear error message instructing the user to use absolute paths.
///
/// # Arguments
/// * `path` - Path that may contain tilde prefix
///
/// # Returns
/// * `Ok(String)` - Expanded path
/// * `Err(SetupError)` - If tilde expansion failed
pub fn expand_tilde_path(path: &str) -> Result<String> {
    let expanded = shellexpand::tilde(path).to_string();

    // Check if tilde expansion failed (HOME not set)
    // When HOME is unset, shellexpand leaves ~ unchanged
    if path.starts_with('~') && expanded.starts_with('~') {
        return Err(SetupError::InvalidConfig(
            "Could not expand ~ in path (HOME environment variable not set).\n\
             Please use absolute path instead.\n\
             Example: /Users/username/key.p8 instead of ~/key.p8"
                .to_string(),
        ));
    }

    Ok(expanded)
}

/// Check that required external dependencies are available
///
/// Validates that:
/// - `security` command exists (macOS system command)
/// - `openssl` command exists (required for PKCS#12 operations)
///
/// # Returns
/// * `Ok(())` - All dependencies are available
/// * `Err(SetupError::MissingDependency)` - A required command is missing
pub fn check_dependencies() -> Result<()> {
    // Validate security command exists
    let security_check = Command::new("security").arg("help").output();

    if security_check.is_err() {
        return Err(SetupError::MissingDependency(
            "'security' command not available.\n\
             This tool requires macOS with the security framework."
                .to_string(),
        ));
    }

    // Validate openssl exists and check version
    let openssl_version = Command::new("openssl")
        .arg("version")
        .output()
        .map_err(|_| {
            SetupError::MissingDependency(
                "OpenSSL/LibreSSL not found in PATH.\n\
             \n\
             To install on macOS:\n\
             • Homebrew: brew install openssl\n\
             • Or use system LibreSSL: /usr/bin/openssl\n\
             \n\
             Required for: Creating PKCS#12 certificate bundles"
                    .to_string(),
            )
        })?;

    let version_str = String::from_utf8_lossy(&openssl_version.stdout);
    if !version_str.is_empty() {
        eprintln!("✓ Found: {}", version_str.trim());
    }

    Ok(())
}

/// Verify keychain is accessible and unlocked
///
/// Uses `security show-keychain-info` to check keychain state.
/// If locked, provides clear error message to user.
///
/// # Arguments
/// * `keychain` - Keychain name (e.g., "login.keychain-db")
///
/// # Returns
/// * `Ok(())` - Keychain is accessible and unlocked
/// * `Err(SetupError::KeychainOperation)` - Keychain is locked or inaccessible
pub fn ensure_keychain_accessible(keychain: &str) -> Result<()> {
    let output = Command::new("security")
        .args(["show-keychain-info", keychain])
        .output()
        .map_err(|e| {
            SetupError::CommandExecution(format!("Failed to check keychain status: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Keychain locked detection
        if stderr.contains("locked") {
            return Err(SetupError::KeychainOperation(format!(
                "Keychain '{keychain}' is locked.\n\
                 Please unlock it before running setup:\n\
                 1. Open Keychain Access app\n\
                 2. Right-click '{keychain}' → Unlock Keychain\n\
                 3. Or run: security unlock-keychain {keychain}"
            )));
        }

        // Keychain doesn't exist
        if stderr.contains("does not exist") || stderr.contains("not found") {
            return Err(SetupError::KeychainOperation(format!(
                "Keychain '{keychain}' not found.\n\
                 Using default 'login.keychain-db' instead."
            )));
        }

        // Other errors
        return Err(SetupError::KeychainOperation(format!(
            "Keychain check failed: {}",
            stderr.trim()
        )));
    }

    Ok(())
}

/// Verify config directory is writable
///
/// Creates config directory if needed and performs write test.
/// This prevents failures after certificate creation.
///
/// # Returns
/// * `Ok(())` - Directory exists and is writable
/// * `Err(SetupError::Io)` - Cannot create directory or write test file
pub fn ensure_config_directory_writable() -> Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| {
            SetupError::MissingConfig("Could not determine config directory".to_string())
        })?
        .join("kodegen");

    // Create directory with secure permissions
    std::fs::create_dir_all(&config_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir_perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&config_dir, dir_perms)?;
    }

    // Test write permissions with temporary file
    let test_file = config_dir.join(".write_test");
    std::fs::write(&test_file, "test").map_err(|e| {
        SetupError::Io(std::io::Error::new(
            e.kind(),
            format!("Config directory not writable: {e}"),
        ))
    })?;
    std::fs::remove_file(&test_file)?;

    Ok(())
}

/// Validate that a path points to a readable .p8 file
///
/// Checks:
/// - File exists
/// - Is a regular file (not directory)
/// - Has .p8 extension (warning if not)
/// - Is not empty
/// - Is readable
/// - Contains valid PEM key (EC or RSA)
pub fn validate_p8_file(path: &Path) -> Result<()> {
    // File existence check
    if !path.exists() {
        return Err(SetupError::InvalidConfig(format!(
            "File not found: {}\n   \
             Please verify the path is correct",
            path.display()
        )));
    }

    // Type check - must be a file not directory
    if !path.is_file() {
        return Err(SetupError::InvalidConfig(format!(
            "Path is not a file: {}\n   \
             Please provide path to .p8 file",
            path.display()
        )));
    }

    // Extension check - warn if not .p8 (but don't fail)
    match path.extension().and_then(|e| e.to_str()) {
        Some("p8") => {} // Correct extension
        Some(ext) => {
            warn!("Warning: Expected .p8 extension, found .{}", ext);
            println!("   File: {}", path.display());
            println!("   This may cause authentication errors");
        }
        None => {
            warn!("Warning: File has no extension");
            println!("   Expected: .p8 file");
            println!("   File: {}", path.display());
        }
    }

    // Readability and size check
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() == 0 => {
            return Err(SetupError::InvalidConfig(format!(
                "File is empty: {}\n   \
                 Please provide a valid .p8 private key file",
                path.display()
            )));
        }
        Ok(_) => {} // File is readable and non-empty
        Err(e) => {
            return Err(SetupError::InvalidConfig(format!(
                "Cannot access file: {}\n   \
                 Error: {}\n   \
                 Please check file permissions",
                path.display(),
                e
            )));
        }
    }

    // Content validation using actual key parsing (matches apple_api.rs pattern)
    // This catches: invalid UTF-8, binary files, corrupted keys, wrong key types
    let key_data = std::fs::read(path)?;

    // Attempt to parse as EC private key (Apple requires EC for App Store Connect)
    if let Err(e) = EncodingKey::from_ec_pem(&key_data) {
        // Try RSA as fallback (some older keys use RSA)
        if EncodingKey::from_rsa_pem(&key_data).is_err() {
            // Neither EC nor RSA worked - file is invalid
            return Err(SetupError::InvalidConfig(format!(
                "File is not a valid private key: {}\n   \
                 Error: {}\n   \
                 \n   \
                 Expected: PEM-encoded EC or RSA private key (.p8 file)\n   \
                 Common causes:\n   \
                 • File is binary (.p12, .der) instead of text (.p8)\n   \
                 • File is corrupted or has invalid UTF-8\n   \
                 • File is not actually a private key\n   \
                 \n   \
                 Download your .p8 key from:\n   \
                 https://appstoreconnect.apple.com/access/api",
                path.display(),
                e
            )));
        }
        // RSA key found - warn that EC is preferred but allow it
        warn!("Warning: RSA key found, but EC key is recommended");
        println!("   File: {}", path.display());
        println!("   Apple recommends EC keys for App Store Connect API");
        println!("   Your RSA key will work, but consider regenerating as EC");
    }

    Ok(())
}
