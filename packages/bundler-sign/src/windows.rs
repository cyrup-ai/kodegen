//! Windows setup - validates `SignTool` and certificates

use crate::config::WindowsSetupConfig;
use crate::error::Result;
use std::io::Write;
use std::process::Command;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

// ============================================================================
// ERROR HANDLING STRATEGY
// ============================================================================
//
// This module distinguishes between CRITICAL and DECORATIVE I/O operations:
//
// CRITICAL I/O - Errors propagated with `?` operator:
//   • External commands: Command::new().output() for signtool/certutil validation
//   • File operations: If implemented for certificate management
//
//   These MUST succeed for the program to function correctly.
//   Errors are propagated to the caller for proper handling.
//
// DECORATIVE I/O - Errors ignored with `let _ =`:
//   • Terminal coloring: buffer.set_color(), writeln!(), bufwtr.print()
//   • Status messages: Success/warning/error indicators with colors
//
//   These are nice-to-have but non-essential. If stderr/stdout is closed,
//   TTY is detached, or output is redirected to a broken pipe, the program
//   should continue without colors - not crash.
//
// This follows Rust CLI ecosystem best practices (cargo, rustc, ripgrep).
// ============================================================================

pub fn show_config() -> Result<()> {
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(
        &mut buffer,
        "Windows signing uses Authenticode. Check certificates in certmgr.msc."
    );
    let _ = bufwtr.print(&buffer);
    Ok(())
}

pub fn interactive_setup() -> Result<()> {
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(&mut buffer, "\n🪟 Windows Setup");
    let _ = writeln!(
        &mut buffer,
        "Windows code signing uses Authenticode certificates."
    );
    let _ = writeln!(&mut buffer, "\nTo import a certificate:");
    let _ = writeln!(
        &mut buffer,
        "  certutil -user -importpfx code_signing_cert.pfx"
    );
    let _ = writeln!(&mut buffer, "\nTo view installed certificates:");
    let _ = writeln!(&mut buffer, "  certmgr.msc");
    let _ = bufwtr.print(&buffer);
    Ok(())
}

pub fn setup_from_config(config: &WindowsSetupConfig, _dry_run: bool, verbose: bool) -> Result<()> {
    if verbose {
        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = writeln!(&mut buffer, "🪟 Windows Setup Validation\n");
        let _ = bufwtr.print(&buffer);
    }

    if config.validate_signtool {
        // Check if signtool.exe is available
        match Command::new("signtool.exe").output() {
            Ok(_output) => {
                // If Command succeeded at all, signtool exists
                if verbose {
                    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
                    let mut buffer = bufwtr.buffer();
                    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                    let _ = write!(&mut buffer, "✓ ");
                    let _ = buffer.reset();
                    let _ = writeln!(&mut buffer, "signtool.exe found in PATH");
                    let _ = bufwtr.print(&buffer);
                }

                // Try to check for certificates using certutil
                if let Ok(cert_output) = Command::new("certutil")
                    .args(["-store", "-user", "My"])
                    .output()
                {
                    let cert_str = String::from_utf8_lossy(&cert_output.stdout);

                    if cert_str.contains("Certificate") {
                        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
                        let mut buffer = bufwtr.buffer();
                        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                        let _ = write!(&mut buffer, "✓ ");
                        let _ = buffer.reset();
                        let _ = writeln!(&mut buffer, "User certificates found in store");

                        if cert_str.contains("Code Signing")
                            || cert_str.contains("1.3.6.1.5.5.7.3.3")
                        {
                            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                            let _ = write!(&mut buffer, "✓ ");
                            let _ = buffer.reset();
                            let _ = writeln!(&mut buffer, "Code signing certificate detected");
                        } else {
                            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
                            let _ = writeln!(
                                &mut buffer,
                                "⚠️  No obvious code signing certificates found"
                            );
                            let _ = buffer.reset();
                            let _ = writeln!(&mut buffer, "   View certificates: certmgr.msc");
                            let _ = writeln!(
                                &mut buffer,
                                "   Import .pfx: certutil -user -importpfx cert.pfx"
                            );
                        }

                        if verbose {
                            let _ = writeln!(
                                &mut buffer,
                                "\nCertificate store contents:\n{}",
                                cert_str.trim()
                            );
                        }
                        let _ = bufwtr.print(&buffer);
                    } else {
                        let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
                        let mut buffer = bufwtr.buffer();
                        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
                        let _ = writeln!(&mut buffer, "⚠️  No certificates found in user store");
                        let _ = buffer.reset();
                        let _ = writeln!(&mut buffer, "   Import code signing certificate:");
                        let _ =
                            writeln!(&mut buffer, "   certutil -user -importpfx code_signing.pfx");
                        let _ = bufwtr.print(&buffer);
                    }
                } else if verbose {
                    let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
                    let mut buffer = bufwtr.buffer();
                    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
                    let _ = writeln!(&mut buffer, "⚠️  Could not query certificate store");
                    let _ = buffer.reset();
                    let _ = bufwtr.print(&buffer);
                }
            }
            Err(_) => {
                // signtool.exe not found in PATH
                return Err(crate::error::SetupError::MissingDependency(
                    "signtool.exe not found in PATH.\n\
                     \n\
                     Install Windows SDK:\n\
                     https://developer.microsoft.com/en-us/windows/downloads/windows-sdk/\n\
                     \n\
                     Or install Visual Studio with 'Desktop development with C++' workload.\n\
                     \n\
                     After installation, signtool.exe is typically at:\n\
                     C:\\Program Files (x86)\\Windows Kits\\10\\bin\\<version>\\x64\\signtool.exe\n\
                     \n\
                     Then run 'kodegen_sign --interactive' for guided setup."
                        .to_string(),
                ));
            }
        }
    }

    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(&mut buffer, "\n✅ Windows validation complete");
    let _ = buffer.reset();
    let _ = bufwtr.print(&buffer);

    Ok(())
}

// ============================================================================
// RUNTIME SIGNING OPERATIONS
// ============================================================================

/// Configuration for Windows binary signing
#[derive(Debug, Clone)]
pub struct SignConfig {
    /// Path to certificate file (.pem, .crt, .pfx)
    pub cert_path: std::path::PathBuf,
    /// Path to private key file (.pem, .key) - not needed for PKCS#12
    pub key_path: Option<std::path::PathBuf>,
    /// Password for encrypted key or PKCS#12 file
    pub password: Option<String>,
    /// Timestamp server URL (highly recommended)
    pub timestamp_url: Option<String>,
    /// Application name to embed in signature
    pub app_name: Option<String>,
    /// Application URL to embed in signature
    pub app_url: Option<String>,
}

impl Default for SignConfig {
    fn default() -> Self {
        Self {
            cert_path: std::path::PathBuf::new(),
            key_path: None,
            password: None,
            timestamp_url: Some("http://timestamp.digicert.com".to_string()),
            app_name: None,
            app_url: None,
        }
    }
}

/// Find osslsigncode binary (for cross-platform signing)
fn find_osslsigncode() -> Result<String> {
    which::which("osslsigncode")
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|_| {
            crate::error::SetupError::MissingDependency(
                "osslsigncode not found. Install with:\n\
             \n\
             macOS:    brew install osslsigncode\n\
             Linux:    Build from source: https://github.com/mtrojnar/osslsigncode\n\
             \n\
             osslsigncode enables Windows Authenticode signing from non-Windows platforms."
                    .to_string(),
            )
        })
}

/// Sign a binary file with Authenticode using osslsigncode (cross-platform)
///
/// This function works on macOS, Linux, and Windows by using osslsigncode,
/// an open-source Authenticode signing tool based on OpenSSL.
///
/// # Arguments
/// * `binary_path` - Path to the executable to sign (.exe, .dll, .sys, etc.)
/// * `config` - Signing configuration (certificates, keys, options)
///
/// # Returns
/// * `Ok(())` - Signing succeeded
/// * `Err(SetupError)` - Signing failed
///
/// # Example
/// ```no_run
/// use std::path::PathBuf;
///
/// // Sign with separate cert and key files
/// let config = SignConfig {
///     cert_path: PathBuf::from("cert.pem"),
///     key_path: Some(PathBuf::from("key.pem")),
///     password: Some("keypass".to_string()),
///     timestamp_url: Some("http://timestamp.digicert.com".to_string()),
///     app_name: Some("MyApp".to_string()),
///     ..Default::default()
/// };
/// sign_binary(Path::new("myapp.exe"), &config)?;
///
/// // Sign with PKCS#12 (.pfx) file
/// let config = SignConfig {
///     cert_path: PathBuf::from("cert.pfx"),
///     key_path: None, // Not needed for PKCS#12
///     password: Some("pfxpass".to_string()),
///     ..Default::default()
/// };
/// sign_binary(Path::new("myapp.exe"), &config)?;
/// ```
pub fn sign_binary(binary_path: &std::path::Path, config: &SignConfig) -> Result<()> {
    let osslsigncode = find_osslsigncode()?;
    let mut args = vec!["sign".to_string()];

    // Detect certificate format: PKCS#12 (.pfx) or separate cert/key
    let is_pkcs12 = config
        .cert_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "pfx" || ext == "p12");

    if is_pkcs12 {
        // PKCS#12 format: osslsigncode sign -pkcs12 cert.pfx -pass password ...
        args.push("-pkcs12".to_string());
        args.push(config.cert_path.to_string_lossy().to_string());

        if let Some(password) = &config.password {
            args.push("-pass".to_string());
            args.push(password.clone());
        }
    } else {
        // Separate cert/key: osslsigncode sign -certs cert.pem -key key.pem -pass password ...
        args.push("-certs".to_string());
        args.push(config.cert_path.to_string_lossy().to_string());

        if let Some(key_path) = &config.key_path {
            args.push("-key".to_string());
            args.push(key_path.to_string_lossy().to_string());

            if let Some(password) = &config.password {
                args.push("-pass".to_string());
                args.push(password.clone());
            }
        } else {
            return Err(crate::error::SetupError::InvalidConfig(
                "key_path must be provided when not using PKCS#12 certificate".to_string(),
            ));
        }
    }

    // Add application name if provided
    if let Some(app_name) = &config.app_name {
        args.push("-n".to_string());
        args.push(app_name.clone());
    }

    // Add application URL if provided
    if let Some(app_url) = &config.app_url {
        args.push("-i".to_string());
        args.push(app_url.clone());
    }

    // Add timestamp server (highly recommended)
    if let Some(timestamp_url) = &config.timestamp_url {
        args.push("-t".to_string());
        args.push(timestamp_url.clone());
    }

    // Input file
    args.push("-in".to_string());
    args.push(binary_path.to_string_lossy().to_string());

    // Output file (in-place signing: overwrite original)
    args.push("-out".to_string());
    args.push(binary_path.to_string_lossy().to_string());

    // Execute osslsigncode
    let output = Command::new(&osslsigncode)
        .args(&args)
        .output()
        .map_err(|e| {
            crate::error::SetupError::CommandExecution(format!(
                "Failed to execute osslsigncode: {e}"
            ))
        })?;

    if !output.status.success() {
        let stderr = std::str::from_utf8(&output.stderr).unwrap_or("(non-UTF-8 error message)");
        let stdout = std::str::from_utf8(&output.stdout).unwrap_or("(non-UTF-8 output)");

        return Err(crate::error::SetupError::CommandExecution(format!(
            "Authenticode signing failed:\n\
                    Command: osslsigncode {}\n\
                    \n\
                    Output:\n{}\n\
                    \n\
                    Error:\n{}\n\
                    \n\
                    Common issues:\n\
                    - Invalid certificate or key file\n\
                    - Wrong password\n\
                    - Certificate expired\n\
                    - Timestamp server unreachable",
            args.join(" "),
            stdout.trim(),
            stderr.trim()
        )));
    }

    Ok(())
}

/// Generate SHA-256 integrity hash for a binary
///
/// Creates a SHA-256 hash of the binary and writes it to a `.sha256` file.
///
/// # Arguments
/// * `binary_path` - Path to the binary to hash
///
/// # Returns
/// * `Ok(String)` - Hex-encoded SHA-256 hash
/// * `Err` - Failed to read file or generate hash
///
/// # Example
/// ```no_run
/// let hash = generate_integrity_hash(Path::new("myapp.exe"))?;
/// // Creates myapp.exe.sha256 and returns hash string
/// ```
pub fn generate_integrity_hash(binary_path: &std::path::Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let binary_data = std::fs::read(binary_path).map_err(crate::error::SetupError::Io)?;

    let mut hasher = Sha256::new();
    hasher.update(&binary_data);
    let hash = hasher.finalize();

    let hash_hex = hex::encode(hash);
    let hash_path = binary_path.with_extension("exe.sha256");

    std::fs::write(&hash_path, &hash_hex).map_err(crate::error::SetupError::Io)?;

    Ok(hash_hex)
}

// ============================================================================
// TIMESTAMP SERVER FALLBACK
// ============================================================================

const TIMESTAMP_SERVERS: &[&str] = &[
    "http://timestamp.digicert.com",
    "http://timestamp.comodoca.com",
    "http://timestamp.sectigo.com",
    "http://timestamp.globalsign.com",
];

/// Sign binary with timestamp server fallback
pub fn sign_binary_with_fallback(binary_path: &std::path::Path, config: &SignConfig) -> Result<()> {
    // Try configured timestamp server first
    if config.timestamp_url.is_some() {
        match sign_binary(binary_path, config) {
            Ok(()) => return Ok(()),
            Err(e) => {
                eprintln!("⚠️  Primary timestamp server failed: {e}");
            }
        }
    }

    // Fallback to alternative timestamp servers
    for (i, timestamp_url) in TIMESTAMP_SERVERS.iter().enumerate() {
        eprintln!(
            "Trying timestamp server {}/{}: {}",
            i + 1,
            TIMESTAMP_SERVERS.len(),
            timestamp_url
        );

        let mut fallback_config = config.clone();
        fallback_config.timestamp_url = Some((*timestamp_url).to_string());

        match sign_binary(binary_path, &fallback_config) {
            Ok(()) => {
                eprintln!("✓ Signed with timestamp server: {timestamp_url}");
                return Ok(());
            }
            Err(e) => {
                eprintln!("⚠️  Timestamp server {timestamp_url} failed: {e}");
                continue;
            }
        }
    }

    // All timestamp servers failed - sign without timestamp as last resort
    eprintln!("⚠️  All timestamp servers failed. Signing without timestamp.");
    let mut no_timestamp_config = config.clone();
    no_timestamp_config.timestamp_url = None;
    sign_binary(binary_path, &no_timestamp_config)
}
