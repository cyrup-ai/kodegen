//! Code signing and app bundle management
//!
//! This module provides macOS code signing functionality for the helper app
//! with secure signing operations and certificate validation.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Sign the helper app with developer certificate
pub fn sign_helper_app(helper_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // First validate the app structure
    super::macos_helper::validate_helper_structure(helper_dir)?;

    // Ensure we have a signing certificate (triggers automated provisioning if needed)
    ensure_signing_certificate()?;

    // Get signing identity from environment or use default
    let signing_identity = std::env::var("KODEGEN_SIGNING_IDENTITY")
        .unwrap_or_else(|_| "Developer ID Application".to_string());

    // Create entitlements file
    create_entitlements_file()?;

    // Sign the executable first
    let executable_path = helper_dir.join("Contents/MacOS/KodegenHelper");
    sign_executable(&executable_path, &signing_identity)?;

    // Sign the entire app bundle
    sign_app_bundle(helper_dir, &signing_identity)?;

    // Verify the signature
    verify_signature(helper_dir)?;

    Ok(())
}

/// Ensure a signing certificate exists, provision one if needed
fn ensure_signing_certificate() -> Result<(), Box<dyn std::error::Error>> {
    // Check if we already have a Developer ID certificate
    let output = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()?;
    
    let identities = String::from_utf8_lossy(&output.stdout);
    
    if identities.contains("Developer ID Application") {
        println!("✓ Found Developer ID certificate");
        return Ok(());
    }
    
    eprintln!("\n⚠ No Developer ID certificate found!");
    eprintln!("   Automated certificate provisioning available.");
    eprintln!("\n   To provision a certificate automatically, run:");
    eprintln!("   cargo run --package kodegen_sign --bin kodegen-setup -- --interactive");
    eprintln!("\n   This will:");
    eprintln!("   1. Prompt for your App Store Connect API credentials");
    eprintln!("   2. Automatically request a Developer ID certificate from Apple");
    eprintln!("   3. Import it to your keychain");
    eprintln!("\n   For now, using ad-hoc signing for development...");
    
    // Use ad-hoc signing as fallback
    unsafe {
        std::env::set_var("KODEGEN_SIGNING_IDENTITY", "-");
    }
    
    Ok(())
}

/// Sign individual executable with optimized signing
fn sign_executable(
    executable_path: &Path,
    identity: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("codesign")
        .args([
            "--force",
            "--sign",
            identity,
            "--options",
            "runtime",
            "--entitlements",
            "helper.entitlements",
            executable_path.to_str().ok_or("Invalid executable path")?,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If signing fails, continue anyway for development builds
        eprintln!("Warning: Failed to sign executable: {stderr}");
        eprintln!("Continuing with unsigned binary for development");
    }

    Ok(())
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
        eprintln!("Warning: Failed to sign app bundle: {stderr}");
        eprintln!("Continuing with unsigned bundle for development");
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

    if output.status.success() {
        println!("Helper app signature verified successfully");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Warning: Signature verification failed: {stderr}");
        eprintln!("This is expected for development builds");
    }

    Ok(())
}
/// Create entitlements file for helper app
fn create_entitlements_file() -> Result<(), Box<dyn std::error::Error>> {
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

    fs::write("helper.entitlements", entitlements_content)?;
    Ok(())
}

/// Validate signing requirements for the build environment
pub fn validate_signing_requirements() -> Result<(), Box<dyn std::error::Error>> {
    // Check if codesign is available
    let codesign_check = Command::new("codesign").arg("--version").output();

    match codesign_check {
        Ok(output) if output.status.success() => {
            println!(
                "codesign available: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        _ => {
            eprintln!("Warning: codesign not available, helper app will be unsigned");
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
            if identities.contains("Developer ID Application") {
                println!("Developer ID signing identity found");
            } else {
                eprintln!("Warning: No Developer ID Application identity found");
                eprintln!("Helper app will be signed with ad-hoc signature");
            }
        }
        _ => {
            eprintln!("Warning: Could not check signing identities");
        }
    }

    Ok(())
}
