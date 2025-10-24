//! Certificate and keychain operations for macOS

use crate::error::{Result, SetupError};
use crate::success;
use fs2::FileExt;
use rand::Rng;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use termcolor::WriteColor;
use zeroize::Zeroize;

/// Import a p12 file into keychain with exclusive locking to prevent race conditions
///
/// This function provides two layers of protection:
/// 1. Exclusive file lock prevents concurrent kodegen instances from importing simultaneously
/// 2. Check-before-import prevents duplicate certificates
///
/// # Arguments
/// * `p12_path` - Path to the temporary p12 file to import
/// * `keychain` - Name of the keychain (e.g., "login.keychain-db")
/// * `password` - Password protecting the p12 file
/// * `common_name` - Common name of the certificate (used for duplicate checking)
///
/// # Returns
/// * `Ok(())` - Import succeeded or certificate already exists
/// * `Err(SetupError)` - Import failed
///
/// # Lock Behavior
/// - Blocks if another kodegen instance is currently importing
/// - Automatically releases lock when function returns (lock file dropped)
/// - Lock file location: ~/.cache/kodegen/keychain.lock
pub fn import_p12_with_lock(
    p12_path: &Path,
    keychain: &str,
    password: &str,
    common_name: &str,
) -> Result<()> {
    use std::fs::OpenOptions;

    // Create lock file path in cache directory
    let lock_dir = dirs::cache_dir()
        .ok_or_else(|| {
            SetupError::MissingConfig("Could not determine cache directory".to_string())
        })?
        .join("kodegen");

    // Ensure lock directory exists
    std::fs::create_dir_all(&lock_dir).map_err(SetupError::Io)?;

    let lock_path = lock_dir.join("keychain.lock");

    // Open/create lock file
    let lock_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&lock_path)
        .map_err(SetupError::Io)?;

    // Acquire exclusive lock - BLOCKS until available
    // This serializes all keychain imports across all kodegen process instances
    lock_file.lock_exclusive().map_err(|e| {
        SetupError::KeychainOperation(format!("Failed to acquire keychain lock: {e}"))
    })?;

    // Lock acquired - now safely check and import

    // Layer 2: Check if certificate already exists
    let check_output = Command::new("security")
        .args(["find-certificate", "-c", common_name, keychain])
        .output()
        .map_err(|e| {
            SetupError::CommandExecution(format!("Failed to check existing certificate: {e}"))
        })?;

    if check_output.status.success() {
        // Certificate already exists - skip import
        success!("Certificate already exists in keychain, skipping import");

        // Lock automatically released when lock_file is dropped
        return Ok(());
    }

    // Certificate doesn't exist - proceed with import
    let import_output = Command::new("security")
        .args([
            "import",
            p12_path
                .to_str()
                .ok_or_else(|| SetupError::InvalidConfig("Invalid p12 path".to_string()))?,
            "-k",
            keychain,
            "-P",
            password,
            "-T",
            "/usr/bin/codesign",
        ])
        .output()
        .map_err(|e| {
            SetupError::KeychainOperation(format!("Failed to execute security import: {e}"))
        })?;

    if !import_output.status.success() {
        let stderr = String::from_utf8_lossy(&import_output.stderr);
        return Err(SetupError::KeychainOperation(format!(
            "Keychain import failed: {stderr}"
        )));
    }

    // Import successful
    // Lock automatically released when lock_file is dropped at function exit

    Ok(())
}

/// Parse security find-identity output and extract Developer ID Application certificates
///
/// Returns a vector of certificate names that start with "Developer ID Application:"
///
/// # Arguments
/// * `output` - The complete Command output from security find-identity
///
/// # Returns
/// * `Ok(Vec<String>)` - List of Developer ID certificate names found
/// * `Err(SetupError)` - If command failed or output is invalid UTF-8
///
/// # Example Output Parsing
/// Input: `  1) ABC123... "Developer ID Application: Acme Corp (TEAM123)"`
/// Output: `["Developer ID Application: Acme Corp (TEAM123)"]`
pub fn check_for_developer_certificates(output: std::process::Output) -> Result<Vec<String>> {
    // Validate command succeeded before parsing
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SetupError::CommandExecution(format!(
            "security find-identity failed with status {}: {}",
            output.status.code().unwrap_or(-1),
            stderr
        )));
    }

    // Use strict UTF-8 parsing - fail if output contains invalid UTF-8
    // This is better than lossy which could corrupt certificate names
    let output_str = std::str::from_utf8(&output.stdout).map_err(|e| {
        SetupError::CommandExecution(format!("security command output is not valid UTF-8: {e}"))
    })?;

    let mut developer_certs = Vec::new();

    // Parse each line to extract certificate names
    for line in output_str.lines() {
        let trimmed = line.trim();

        // Skip empty lines and summary line
        if trimmed.is_empty() || trimmed.contains("valid identities found") {
            continue;
        }

        // Extract certificate name from within double quotes
        // Format: `  1) HASH... "Certificate Name Here"`
        if let Some(start_quote) = trimmed.find('"') {
            // Find the closing quote
            if let Some(end_quote) = trimmed[start_quote + 1..].find('"') {
                let cert_name = &trimmed[start_quote + 1..start_quote + 1 + end_quote];

                // Only include Developer ID Application certificates
                if cert_name.starts_with("Developer ID Application:") {
                    developer_certs.push(cert_name.to_string());
                }
            }
        }
    }

    Ok(developer_certs)
}

// ============================================================================
// CERTIFICATE EXPIRY VALIDATION
// ============================================================================

use chrono::{DateTime, Utc};

/// Certificate information with expiry date
pub struct CertificateInfo {
    pub name: String,
    pub sha1_hash: String,
    pub expiry: DateTime<Utc>,
    pub is_valid: bool,
}

/// Check certificate expiry via security command
pub fn check_certificate_expiry(cert_name: &str) -> Result<CertificateInfo> {
    let output = Command::new("security")
        .args(["find-certificate", "-c", cert_name, "-p"])
        .output()?;

    if !output.status.success() {
        return Err(SetupError::KeychainOperation(
            "Certificate not found".to_string(),
        ));
    }

    // Parse PEM output and extract expiry using openssl
    let pem_data = String::from_utf8_lossy(&output.stdout);

    // Write to temp file for openssl parsing
    let temp_dir = TempDir::new()?;
    let cert_path = temp_dir.path().join("cert.pem");
    std::fs::write(&cert_path, pem_data.as_bytes())?;

    // Get expiry date via openssl
    let openssl_out = Command::new("openssl")
        .args(["x509", "-enddate", "-noout", "-in"])
        .arg(&cert_path)
        .output()?;

    // Parse: notAfter=Dec 31 23:59:59 2025 GMT
    let date_str = String::from_utf8_lossy(&openssl_out.stdout);
    let expiry_str = date_str
        .strip_prefix("notAfter=")
        .ok_or_else(|| SetupError::CertificateGeneration("Invalid date format".to_string()))?
        .trim();

    let expiry = DateTime::parse_from_str(expiry_str, "%b %d %H:%M:%S %Y %Z")
        .map_err(|e| SetupError::CertificateGeneration(format!("Failed to parse date: {e}")))?
        .with_timezone(&Utc);

    let now = Utc::now();
    let is_valid = expiry > now;

    Ok(CertificateInfo {
        name: cert_name.to_string(),
        sha1_hash: get_cert_hash(&cert_path)?,
        expiry,
        is_valid,
    })
}

fn get_cert_hash(cert_path: &Path) -> Result<String> {
    let output = Command::new("openssl")
        .args(["x509", "-fingerprint", "-sha1", "-noout", "-in"])
        .arg(cert_path)
        .output()?;

    let hash_line = String::from_utf8_lossy(&output.stdout);
    let hash = hash_line
        .strip_prefix("SHA1 Fingerprint=")
        .ok_or_else(|| SetupError::CertificateGeneration("Invalid hash format".to_string()))?
        .trim()
        .replace(':', "");

    Ok(hash)
}

/// Import certificate and private key into macOS keychain
///
/// Creates temporary p12 bundle and imports via `security` command.
/// Temporary files are automatically cleaned up.
pub fn import_certificate_to_keychain(
    cert_der: &[u8],
    private_key_pem: &str,
    keychain: &str,
    common_name: &str,
) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Create secure temporary directory with 0700 permissions
    let temp_dir = TempDir::new()?;

    // Write certificate
    let temp_cert = temp_dir.path().join("cert.der");
    std::fs::write(&temp_cert, cert_der)?;
    std::fs::set_permissions(&temp_cert, std::fs::Permissions::from_mode(0o600))?;

    // Write private key
    let temp_key = temp_dir.path().join("key.pem");
    std::fs::write(&temp_key, private_key_pem)?;
    std::fs::set_permissions(&temp_key, std::fs::Permissions::from_mode(0o600))?;

    // Create p12 bundle
    let temp_p12 = temp_dir.path().join("cert.p12");

    // Generate random password for p12
    let mut password: String = (0..32)
        .map(|_| {
            const CHARSET: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            let idx = rand::rng().random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    let output = Command::new("openssl")
        .args([
            "pkcs12",
            "-export",
            "-inkey",
            temp_key
                .to_str()
                .ok_or_else(|| SetupError::InvalidConfig("Invalid temp key path".to_string()))?,
            "-in",
            temp_cert
                .to_str()
                .ok_or_else(|| SetupError::InvalidConfig("Invalid temp cert path".to_string()))?,
            "-out",
            temp_p12
                .to_str()
                .ok_or_else(|| SetupError::InvalidConfig("Invalid temp p12 path".to_string()))?,
            "-passout",
            &format!("pass:{password}"),
        ])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SetupError::MissingDependency(
                    "openssl command not found. Please install OpenSSL.".to_string(),
                )
            } else {
                SetupError::CommandExecution(format!("Failed to create p12: {e}"))
            }
        })?;

    if !output.status.success() {
        password.zeroize();
        return Err(SetupError::CertificateGeneration(format!(
            "Failed to create p12: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    std::fs::set_permissions(&temp_p12, std::fs::Permissions::from_mode(0o600))?;

    // Import p12 to keychain using locking mechanism to prevent race conditions
    let result = import_p12_with_lock(&temp_p12, keychain, &password, common_name);

    // Zeroize password before checking result
    password.zeroize();

    // Return result (propagate any errors)
    result?;

    // Automatic cleanup when temp_dir is dropped
    Ok(())
}

// ============================================================================
// TEMPORARY KEYCHAIN (CI/CD)
// ============================================================================

use rand::distr::{Alphanumeric, SampleString};

/// Temporary keychain that auto-deletes when dropped
///
/// Used in CI/CD to avoid polluting the login keychain.
/// Creates random-named keychain, imports certificate, auto-cleanup on drop.
///
/// # Example
/// ```no_run
/// let temp = TempKeychain::from_certificate_file("cert.p12", "password")?;
/// // Use temp.signing_identity() for signing
/// // Keychain auto-deleted when temp goes out of scope
/// ```
pub struct TempKeychain {
    path: std::path::PathBuf,
    _password: String, // Kept for lifetime, not accessed after creation
    signing_identity: String,
}

impl TempKeychain {
    /// Create from .p12 certificate file
    pub fn from_certificate_file(cert_path: &Path, cert_password: &str) -> Result<Self> {
        let cert_bytes = std::fs::read(cert_path)?;
        Self::from_certificate_bytes(&cert_bytes, cert_password)
    }

    /// Create from certificate bytes (for base64-decoded env var)
    pub fn from_certificate_bytes(cert_bytes: &[u8], cert_password: &str) -> Result<Self> {
        // Generate random keychain name (16 char alphanumeric)
        let keychain_name = format!(
            "{}.keychain-db",
            Alphanumeric.sample_string(&mut rand::rng(), 16)
        );

        let keychain_path = dirs::home_dir()
            .ok_or_else(|| SetupError::MissingConfig("HOME not set".to_string()))?
            .join("Library/Keychains")
            .join(&keychain_name);

        // Generate random keychain password
        let keychain_password = Alphanumeric.sample_string(&mut rand::rng(), 16);

        // Write certificate to temporary file
        let temp_dir = tempfile::TempDir::new()?;
        let temp_cert = temp_dir.path().join("cert.p12");
        std::fs::write(&temp_cert, cert_bytes)?;

        // Create keychain
        run_security(
            &["create-keychain", "-p", &keychain_password],
            &keychain_path,
        )?;

        // Unlock keychain
        run_security(
            &["unlock-keychain", "-p", &keychain_password],
            &keychain_path,
        )?;

        // Import Apple's Developer ID G2 CA certificate first (required for trust chain)
        let g2_ca_result = Command::new("curl")
            .args([
                "-s",
                "https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer",
            ])
            .output();

        if let Ok(ca_output) = g2_ca_result
            && ca_output.status.success()
            && !ca_output.stdout.is_empty()
        {
            // Properly handle stdin piping with explicit flush and close
            if let Ok(mut child) = Command::new("security")
                .arg("import")
                .arg("-")
                .args(["-k", &keychain_path.to_string_lossy(), "-A"])
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    // Write CA certificate data to stdin
                    let _ = stdin.write_all(&ca_output.stdout);
                    // Explicitly flush to ensure data is sent
                    let _ = stdin.flush();
                    // Drop stdin to close the pipe before waiting
                    drop(stdin);
                }
                // Wait for security command to complete
                let _ = child.wait();
            }
        }

        // Import certificate
        let import_result = Command::new("security")
            .arg("import")
            .arg(&temp_cert)
            .args(["-P", cert_password])
            .args([
                "-T",
                "/usr/bin/codesign",
                "-T",
                "/usr/bin/pkgbuild",
                "-T",
                "/usr/bin/productbuild",
            ])
            .arg("-k")
            .arg(&keychain_path)
            .output();

        // Clean up temp cert
        drop(temp_dir);

        let import_output = import_result?;
        if !import_output.status.success() {
            // Clean up keychain on failure
            let _ = Command::new("security")
                .arg("delete-keychain")
                .arg(&keychain_path)
                .output();

            return Err(SetupError::KeychainOperation(format!(
                "Failed to import certificate: {}",
                String::from_utf8_lossy(&import_output.stderr)
            )));
        }

        // Set keychain settings (1 hour timeout, auto-unlock)
        run_security(
            &["set-keychain-settings", "-t", "3600", "-u"],
            &keychain_path,
        )?;

        // Set partition list (prevents password prompts during signing)
        run_security(
            &[
                "set-key-partition-list",
                "-S",
                "apple-tool:,apple:,codesign:",
                "-s",
                "-k",
                &keychain_password,
            ],
            &keychain_path,
        )?;

        // Add to keychain search list
        let list_output = Command::new("security")
            .args(["list-keychain", "-d", "user"])
            .output()?;

        let current_keychains = String::from_utf8_lossy(&list_output.stdout);
        let mut keychains: Vec<String> = current_keychains
            .lines()
            .map(|l| l.trim().trim_matches('"').to_string())
            .filter(|l| !l.is_empty())
            .collect();

        keychains.push(keychain_path.to_string_lossy().to_string());

        Command::new("security")
            .args(["list-keychain", "-d", "user", "-s"])
            .args(&keychains)
            .output()?;

        // Find signing identity
        let signing_identity = Self::find_signing_identity(&keychain_path)?;

        Ok(Self {
            path: keychain_path,
            _password: keychain_password,
            signing_identity,
        })
    }

    fn find_signing_identity(keychain_path: &Path) -> Result<String> {
        let output = Command::new("security")
            .args(["find-identity", "-v", "-p", "codesigning"])
            .arg(keychain_path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Find Developer ID certificate
        for line in stdout.lines() {
            if (line.contains("\"Developer ID Application:")
                || line.contains("\"Apple Development:")
                || line.contains("\"Mac Developer:"))
                && let Some(start) = line.find('"')
                && let Some(end) = line[start + 1..].find('"')
            {
                return Ok(line[start + 1..start + 1 + end].to_string());
            }
        }

        Err(SetupError::KeychainOperation(
            "No valid signing identity found in temporary keychain".to_string(),
        ))
    }

    #[must_use]
    pub fn signing_identity(&self) -> &str {
        &self.signing_identity
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempKeychain {
    fn drop(&mut self) {
        // Automatic cleanup when TempKeychain goes out of scope
        let result = Command::new("security")
            .arg("delete-keychain")
            .arg(&self.path)
            .output();

        match result {
            Ok(output) if output.status.success() => {
                // Success (silent)
            }
            Ok(output) => {
                eprintln!(
                    "⚠️  Failed to delete temporary keychain: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                eprintln!("⚠️  Could not delete temporary keychain: {e}");
            }
        }
    }
}

// Helper function for security commands
fn run_security(args: &[&str], keychain_path: &Path) -> Result<()> {
    let mut cmd_args = args.to_vec();
    cmd_args.push(
        keychain_path
            .to_str()
            .ok_or_else(|| SetupError::InvalidConfig("Invalid keychain path".to_string()))?,
    );

    let output = Command::new("security").args(&cmd_args).output()?;

    if !output.status.success() {
        return Err(SetupError::KeychainOperation(format!(
            "security {} failed: {}",
            args[0],
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

// ============================================================================
// ENHANCED SIGNING WITH ENTITLEMENTS
// ============================================================================

/// Sign binary with optional entitlements and hardened runtime
///
/// # Arguments
/// * `binary_path` - Path to binary/app to sign
/// * `signing_identity` - Certificate name or hash
/// * `entitlements_path` - Optional path to entitlements.plist
/// * `hardened_runtime` - Enable hardened runtime (required for notarization)
///
/// # Returns
/// * `Ok(())` - Signing succeeded
/// * `Err(SetupError)` - Signing failed
pub fn sign_with_entitlements(
    binary_path: &Path,
    signing_identity: &str,
    entitlements_path: Option<&Path>,
    hardened_runtime: bool,
) -> Result<()> {
    if !binary_path.exists() {
        return Err(SetupError::InvalidConfig(format!(
            "Binary not found: {}",
            binary_path.display()
        )));
    }

    let mut args = vec!["-s", signing_identity];

    // Add hardened runtime flag (required for notarization)
    if hardened_runtime {
        args.push("--options");
        args.push("runtime");
    }

    // Add entitlements if provided
    if let Some(entitlements) = entitlements_path {
        if !entitlements.exists() {
            return Err(SetupError::InvalidConfig(format!(
                "Entitlements file not found: {}",
                entitlements.display()
            )));
        }

        args.push("--entitlements");
        args.push(
            entitlements.to_str().ok_or_else(|| {
                SetupError::InvalidConfig("Invalid entitlements path".to_string())
            })?,
        );
    }

    // Force re-signing
    args.push("--force");

    // Add binary path
    args.push(
        binary_path
            .to_str()
            .ok_or_else(|| SetupError::InvalidConfig("Invalid binary path".to_string()))?,
    );

    // Execute codesign
    let output = Command::new("codesign")
        .args(&args)
        .output()
        .map_err(|e| SetupError::CommandExecution(format!("codesign failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SetupError::CommandExecution(format!(
            "Code signing failed:\n\
             Command: codesign {}\n\
             Error: {}",
            args.join(" "),
            stderr
        )));
    }

    success!("Signed: {}", binary_path.display());

    // Verify signature
    let verify = Command::new("codesign")
        .args(["--verify", "--verbose"])
        .arg(binary_path)
        .output()?;

    if verify.status.success() {
        success!("Signature verified");
    }

    Ok(())
}
