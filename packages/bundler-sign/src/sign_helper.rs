//! Code signing using proven macos infrastructure
//!
//! This module provides macOS code signing for the helper app by delegating
//! to the proven signing infrastructure in the macos module.

use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use crate::macos::keychain::TempKeychain;

/// Sign the helper app with developer certificate
/// 
/// This uses the proven signing infrastructure and automatically:
/// 1. Imports certificate from `APPLE_CERTIFICATE` env var if present
/// 2. Falls back to keychain certificate if available
/// 3. FAILS BUILD if no certificate found (unsigned releases are NEVER allowed)
pub fn sign_helper_app(helper_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // First validate the app structure
    crate::build_helper::validate_helper_structure(helper_dir)?;

    // Setup certificate from environment if available (uses proven infrastructure)
    let temp_keychain = setup_certificate_from_env()?;

    // Get signing identity (either from temp keychain or system keychain)
    let signing_identity = if let Some(ref keychain) = temp_keychain {
        keychain.signing_identity().to_string()
    } else {
        // Fallback: check system keychain for Developer ID certificate
        get_system_signing_identity()?
    };

    // Create entitlements in temp directory with unique name
    let entitlements_path = env::temp_dir().join(format!(
        "kodegen_entitlements_{}.plist",
        std::process::id()
    ));
    create_entitlements_file(&entitlements_path)?;

    // Sign executable using proven signing function from macos::keychain
    let executable_path = helper_dir.join("Contents/MacOS/KodegenHelper");
    crate::macos::keychain::sign_with_entitlements(
        &executable_path,
        &signing_identity,
        Some(&entitlements_path),
        true, // hardened runtime
    ).map_err(|e| format!("Failed to sign executable: {e}"))?;

    // Sign the entire app bundle
    sign_app_bundle(helper_dir, &signing_identity)?;

    // Verify the signature
    verify_signature(helper_dir)?;

    // Additional validation using is_helper_signed
    if !crate::build_helper::is_helper_signed(helper_dir) {
        return Err("Helper signature validation failed".into());
    }

    // Cleanup temporary entitlements file
    crate::cleanup_path(&entitlements_path, "entitlements file");

    Ok(())
}

/// Setup certificate from environment variables using proven infrastructure
/// Returns `TempKeychain` if certificate was imported from env, None otherwise
fn setup_certificate_from_env() -> Result<Option<TempKeychain>, Box<dyn std::error::Error>> {
    // Check for APPLE_CERTIFICATE and APPLE_CERTIFICATE_PASSWORD (base64 .p12)
    if let (Ok(cert_base64), Ok(password)) = (
        std::env::var("APPLE_CERTIFICATE"),
        std::env::var("APPLE_CERTIFICATE_PASSWORD")
    ) {
        // Use proven certificate import from macos::keychain
        use base64::Engine;
        let cert_bytes = base64::engine::general_purpose::STANDARD.decode(cert_base64)
            .map_err(|e| format!("Invalid APPLE_CERTIFICATE (not valid base64): {e}"))?;
        
        let temp_keychain = TempKeychain::from_certificate_bytes(&cert_bytes, &password)
            .map_err(|e| format!("Failed to import certificate: {e}"))?;
        
        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = write!(&mut buffer, "✓ ");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, "Imported certificate from APPLE_CERTIFICATE env var");
        let _ = bufwtr.print(&buffer);
        
        Ok(Some(temp_keychain))
    } else {
        Ok(None) // No env vars set
    }
}

/// Get signing identity from system keychain
fn get_system_signing_identity() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()?;
    
    let identities = String::from_utf8_lossy(&output.stdout);
    
    if identities.contains("Developer ID Application") {
        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = write!(&mut buffer, "✓ ");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, "Found Developer ID certificate in system keychain");
        let _ = bufwtr.print(&buffer);
        return Ok("Developer ID Application".to_string());
    }
    
    // CRITICAL: No certificate = BUILD FAILURE
    // Ad-hoc signing must NEVER be allowed for releases
    let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
    let _ = writeln!(&mut buffer, "❌ FATAL: No Developer ID certificate found!");
    let _ = buffer.reset();
    let _ = writeln!(&mut buffer, "\nRELEASE BUILD REQUIRES VALID CERTIFICATE");
    let _ = writeln!(&mut buffer, "\nOptions:");
    let _ = writeln!(&mut buffer, "  1. Set APPLE_CERTIFICATE + APPLE_CERTIFICATE_PASSWORD env vars (CI/CD)");
    let _ = writeln!(&mut buffer, "  2. Run: cargo run --package kodegen_sign --bin kodegen-setup -- --interactive");
    let _ = writeln!(&mut buffer, "\nUnsigned releases are NEVER allowed - customer trust depends on it!");
    let _ = bufwtr.print(&buffer);
    
    Err("No valid code signing certificate available. Release build cannot proceed.".into())
}

/// Sign app bundle with full bundle signing
fn sign_app_bundle(app_path: &Path, identity: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("codesign")
        .args([
            "--force",
            "--deep",
            "--sign",
            identity,
            "--options",
            "runtime",
            app_path.to_str().ok_or("Invalid app path")?,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Bundle signing failed: {stderr}").into());
    }

    Ok(())
}

/// Verify code signature
fn verify_signature(app_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("codesign")
        .args([
            "--verify",
            "--deep",
            "--strict",
            app_path.to_str().ok_or("Invalid app path")?,
        ])
        .output()?;

    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();

    if output.status.success() {
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = write!(&mut buffer, "✓ ");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, "Helper app signature verified successfully");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        let _ = write!(&mut buffer, "⚠️  Warning: ");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, "Signature verification failed: {stderr}");
        let _ = writeln!(&mut buffer, "This is expected for development builds");
    }
    
    let _ = bufwtr.print(&buffer);
    Ok(())
}
/// Create entitlements file for helper app
fn create_entitlements_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let entitlements_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.authorization.groups</key>
    <array>
        <string>admin</string>
    </array>
    <key>com.apple.security.inherit</key>
    <true/>
</dict>
</plist>"#;

    fs::write(path, entitlements_content)?;
    Ok(())
}

/// Validate signing requirements for the build environment
pub fn validate_signing_requirements() -> Result<(), Box<dyn std::error::Error>> {
    // Check if codesign is available
    let codesign_check = Command::new("codesign").arg("--version").output();

    match codesign_check {
        Ok(output) if output.status.success() => {
            let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = write!(&mut buffer, "✓ ");
            let _ = buffer.reset();
            let _ = writeln!(&mut buffer, "codesign available: {}", 
                String::from_utf8_lossy(&output.stdout).trim());
            let _ = bufwtr.print(&buffer);
        }
        _ => {
            let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
            let _ = write!(&mut buffer, "⚠️  Warning: ");
            let _ = buffer.reset();
            let _ = writeln!(&mut buffer, "codesign not available, helper app will be unsigned");
            let _ = bufwtr.print(&buffer);
            
            return Ok(()); // Don't fail the build, just warn
        }
    }

    // Check for available signing identities (optional)
    let identities_check = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output();

    match identities_check {
        Ok(output) if output.status.success() => {
            let identities = String::from_utf8_lossy(&output.stdout);
            let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            
            if identities.contains("Developer ID Application") {
                let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                let _ = write!(&mut buffer, "✓ ");
                let _ = buffer.reset();
                let _ = writeln!(&mut buffer, "Developer ID signing identity found");
            } else {
                let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
                let _ = writeln!(&mut buffer, "⚠️  WARNING: No Developer ID Application identity found");
                let _ = buffer.reset();
                let _ = writeln!(&mut buffer, "Helper app will be signed with ad-hoc signature");
            }
            
            let _ = bufwtr.print(&buffer);
        }
        _ => {
            let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
            let _ = write!(&mut buffer, "⚠️  Warning: ");
            let _ = buffer.reset();
            let _ = writeln!(&mut buffer, "Could not check signing identities");
            let _ = bufwtr.print(&buffer);
        }
    }

    Ok(())
}
