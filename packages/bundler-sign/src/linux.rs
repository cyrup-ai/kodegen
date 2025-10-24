//! Linux setup - validates build tools

use crate::config::LinuxSetupConfig;
use crate::error::Result;
use std::io::Write;
use std::process::Command;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

// ============================================================================
// GPG CONFIGURATION
// ============================================================================

/// GPG binary name
const GPG_BINARY: &str = "gpg";

/// GPG command arguments for listing secret keys
const GPG_LIST_SECRET_KEYS_ARGS: &[&str] = &["--list-secret-keys", "--keyid-format", "LONG"];

/// Secret key marker in GPG output
const SECRET_KEY_MARKER: &str = "sec";

/// Installation instructions for missing GPG
const GPG_INSTALL_INSTRUCTIONS: &str = "\
GPG not found. Install with your package manager:

Ubuntu/Debian:  sudo apt-get install gnupg
Fedora/RHEL:    sudo dnf install gnupg2
Arch Linux:     sudo pacman -S gnupg

Then run 'kodegen_sign --interactive' for guided setup.";

// ============================================================================
// ERROR HANDLING STRATEGY
// ============================================================================
//
// This module distinguishes between CRITICAL and DECORATIVE I/O operations:
//
// CRITICAL I/O - Errors propagated with `?` operator:
//   • External commands: Command::new().output() for GPG validation
//   • File operations: If implemented for key management
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
    // Create writer once
    let stdout_writer = BufferWriter::stdout(ColorChoice::Auto);

    let mut buffer = stdout_writer.buffer();
    let _ = writeln!(
        &mut buffer,
        "Linux signing uses GPG. Run 'gpg --list-secret-keys' to see keys."
    );
    let _ = stdout_writer.print(&buffer);

    Ok(())
}

pub fn interactive_setup() -> Result<()> {
    // Create writer once
    let stdout_writer = BufferWriter::stdout(ColorChoice::Auto);

    let mut buffer = stdout_writer.buffer();
    let _ = writeln!(&mut buffer, "\n🐧 Linux Setup");
    let _ = writeln!(&mut buffer, "Linux code signing uses GPG.");
    let _ = writeln!(&mut buffer, "\nTo generate a signing key:");
    let _ = writeln!(&mut buffer, "  gpg --full-generate-key");
    let _ = writeln!(&mut buffer, "\nTo list existing keys:");
    let _ = writeln!(&mut buffer, "  gpg --list-secret-keys --keyid-format LONG");
    let _ = stdout_writer.print(&buffer);

    Ok(())
}

/// Check that GPG is installed and accessible
///
/// This is a CRITICAL check - returns error if GPG is not available.
/// Called early in setup to fail fast if GPG is missing.
///
/// # Returns
/// * `Ok(String)` - GPG version string for display
/// * `Err(SetupError::MissingDependency)` - GPG not found or not working
fn check_gpg_installed() -> Result<String> {
    let output = Command::new(GPG_BINARY)
        .arg("--version")
        .output()
        .map_err(|_| {
            crate::error::SetupError::MissingDependency(GPG_INSTALL_INSTRUCTIONS.to_string())
        })?;

    if !output.status.success() {
        // Use strict UTF-8 parsing for error messages
        let stderr_msg = std::str::from_utf8(&output.stderr).unwrap_or("(non-UTF-8 error message)");

        return Err(crate::error::SetupError::CommandExecution(format!(
            "GPG command failed: {}",
            stderr_msg
        )));
    }

    // Lossy is fine for display
    let version = String::from_utf8_lossy(&output.stdout);
    Ok(version.lines().next().unwrap_or("GPG").to_string())
}

/// Check for GPG signing keys (informational only)
///
/// This is INFORMATIONAL - does not fail if keys can't be listed.
/// Returns None if listing fails or if permissions prevent access.
///
/// # Returns
/// * `Some(String)` - Raw output from gpg --list-secret-keys
/// * `None` - Could not list keys (non-fatal)
fn check_gpg_keys() -> Option<String> {
    let output = Command::new(GPG_BINARY)
        .args(GPG_LIST_SECRET_KEYS_ARGS)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

pub fn setup_from_config(config: &LinuxSetupConfig, _dry_run: bool, verbose: bool) -> Result<()> {
    // Create writers ONCE at function start - reuse throughout
    let stdout_writer = BufferWriter::stdout(ColorChoice::Auto);
    let stderr_writer = BufferWriter::stderr(ColorChoice::Auto);

    if verbose {
        let mut buffer = stdout_writer.buffer();
        let _ = writeln!(&mut buffer, "🐧 Linux Setup Validation\n");
        let _ = stdout_writer.print(&buffer);
    }

    if config.validate_gpg {
        // ================================================================
        // TOCTOU LIMITATION NOTICE
        // ================================================================
        // This validation performs point-in-time checks of system state.
        // Between the GPG version check and key listing operations,
        // system state may change (GPG uninstalled, keyring deleted, etc.).
        //
        // This is acceptable for a validation tool because:
        // 1. We check current state, not future state
        // 2. Environment changes after ANY tool exits are not detectable
        // 3. The tool handles command failures gracefully (no crashes)
        // 4. In normal desktop use, the race window (~100ms) is negligible
        //
        // In containerized/CI environments with concurrent modifications,
        // the validation may report inconsistent results. This is expected
        // behavior - the environment IS inconsistent during such operations.
        // ================================================================

        // CRITICAL: Check GPG is installed (propagates errors)
        let version = check_gpg_installed()?;

        if verbose {
            let mut buffer = stdout_writer.buffer();
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = write!(&mut buffer, "✓ ");
            let _ = buffer.reset();
            let _ = writeln!(&mut buffer, "{}", version);
            let _ = stdout_writer.print(&buffer);
        }

        // INFORMATIONAL: Check for keys (warnings only, never fails)
        // NOTE: This is a separate operation from version check above.
        // If it fails, we warn but don't error (environment may have changed).
        match check_gpg_keys() {
            Some(keys) if keys.contains(SECRET_KEY_MARKER) => {
                let mut buffer = stdout_writer.buffer();
                let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                let _ = write!(&mut buffer, "✓ ");
                let _ = buffer.reset();
                let _ = writeln!(&mut buffer, "GPG signing keys found");
                if verbose {
                    let _ = writeln!(&mut buffer, "\n{}", keys.trim());
                }
                let _ = stdout_writer.print(&buffer);
            }
            Some(_) => {
                let mut buffer = stderr_writer.buffer();
                let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
                let _ = writeln!(&mut buffer, "⚠️  No GPG signing keys detected");
                let _ = buffer.reset();
                let _ = writeln!(&mut buffer, "   Generate with: gpg --full-generate-key");
                let _ = writeln!(
                    &mut buffer,
                    "   Then export: gpg --export -a 'Your Name' > public.key"
                );
                let _ = stderr_writer.print(&buffer);
            }
            None if verbose => {
                let mut buffer = stderr_writer.buffer();
                let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
                let _ = writeln!(&mut buffer, "⚠️  Could not list GPG keys");
                let _ = buffer.reset();
                let _ = writeln!(
                    &mut buffer,
                    "   (Note: GPG state may have changed since version check)"
                );
                let _ = stderr_writer.print(&buffer);
            }
            None => {
                // Silent fail in non-verbose mode - keys are informational
            }
        }
    }

    let mut buffer = stdout_writer.buffer();
    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(&mut buffer, "\n✅ Linux validation complete");
    let _ = buffer.reset();
    let _ = stdout_writer.print(&buffer);

    Ok(())
}

// ============================================================================
// RUNTIME SIGNING OPERATIONS
// ============================================================================

/// Find GPG binary on the system
///
/// Tries gpg2 first, then gpg. Returns the path to the first available binary.
///
/// # Returns
/// * `Ok(String)` - Path to GPG binary
/// * `Err(SetupError::MissingDependency)` - No GPG binary found
fn find_gpg_binary() -> Result<String> {
    // Try gpg2 first (more common on modern systems)
    if let Ok(gpg2_path) = which::which("gpg2") {
        return Ok(gpg2_path.to_string_lossy().to_string());
    }

    // Fall back to gpg
    if let Ok(gpg_path) = which::which("gpg") {
        return Ok(gpg_path.to_string_lossy().to_string());
    }

    Err(crate::error::SetupError::MissingDependency(
        GPG_INSTALL_INSTRUCTIONS.to_string(),
    ))
}

/// Sign a binary file with GPG
///
/// Creates a detached ASCII-armored signature for the specified binary.
/// The signature is written to a file with the same name plus `.sig` extension.
///
/// # Arguments
/// * `binary_path` - Path to the binary to sign
/// * `key_id` - Optional GPG key ID to use for signing. If None, uses default key.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the generated signature file
/// * `Err(SetupError)` - Signing failed
///
/// # Example
/// ```no_run
/// let sig_path = sign_binary(Path::new("myapp"), Some("ABC123"))?;
/// // Creates myapp.sig
/// ```
pub fn sign_binary(
    binary_path: &std::path::Path,
    key_id: Option<&str>,
) -> Result<std::path::PathBuf> {
    use std::path::PathBuf;

    // Find GPG binary
    let gpg = find_gpg_binary()?;

    // Build GPG signing arguments
    let mut args = vec!["--detach-sign".to_string(), "--armor".to_string()];

    if let Some(key) = key_id {
        args.push("--local-user".to_string());
        args.push(key.to_string());
    }

    let sig_path = binary_path.with_extension("sig");
    args.push("--output".to_string());
    args.push(sig_path.to_string_lossy().to_string());
    args.push(binary_path.to_string_lossy().to_string());

    // Execute GPG signing
    let output = Command::new(&gpg).args(&args).output().map_err(|e| {
        crate::error::SetupError::CommandExecution(format!(
            "Failed to execute GPG for signing: {}",
            e
        ))
    })?;

    if !output.status.success() {
        let stderr = std::str::from_utf8(&output.stderr).unwrap_or("(non-UTF-8 error message)");
        return Err(crate::error::SetupError::CommandExecution(format!(
            "GPG signing failed: {}",
            stderr
        )));
    }

    // Verify signature file was created
    if !sig_path.exists() {
        return Err(crate::error::SetupError::CommandExecution(
            "GPG signing appeared to succeed but signature file was not created".to_string(),
        ));
    }

    Ok(sig_path)
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
/// let hash = generate_integrity_hash(Path::new("myapp"))?;
/// // Creates myapp.sha256 and returns hash string
/// ```
pub fn generate_integrity_hash(binary_path: &std::path::Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let binary_data = std::fs::read(binary_path).map_err(|e| crate::error::SetupError::Io(e))?;

    let mut hasher = Sha256::new();
    hasher.update(&binary_data);
    let hash = hasher.finalize();

    let hash_hex = hex::encode(hash);
    let hash_path = binary_path.with_extension("sha256");

    std::fs::write(&hash_path, &hash_hex).map_err(|e| crate::error::SetupError::Io(e))?;

    Ok(hash_hex)
}
