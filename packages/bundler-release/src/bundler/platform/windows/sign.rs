//! Windows code signing integration.
//!
//! This module provides integration between the bundler and the kodegen_sign
//! package, adapting bundler Settings to the sign crate's API.

use crate::bundler::{
    error::Result,
    settings::Settings,
};
use std::path::Path;

/// Sign a Windows executable or installer using kodegen_sign
///
/// This function:
/// 1. Checks if signing is configured (cert_path present)
/// 2. Builds SignConfig from WindowsSettings
/// 3. Calls kodegen_bundler_sign::windows::sign_binary
/// 4. Generates SHA-256 integrity hash
///
/// # Arguments
/// * `binary_path` - Path to the .exe or .msi file to sign
/// * `settings` - Bundler settings containing signing configuration
///
/// # Returns
/// * `Ok(())` - Signing succeeded or was skipped (no cert configured)
/// * `Err(Error)` - Signing failed
///
/// # Example
/// ```no_run
/// sign_file(Path::new("MyApp_1.0.0_x64.msi"), &settings)?;
/// ```
pub fn sign_file(binary_path: &Path, settings: &Settings) -> Result<()> {
    let windows = &settings.bundle_settings().windows;
    
    // Check if signing is configured
    let cert_path = match &windows.cert_path {
        Some(path) => path,
        None => {
            log::info!("No certificate configured (cert_path), skipping Windows signing");
            return Ok(());
        }
    };
    
    log::info!("Signing {} with Authenticode", binary_path.display());
    
    // Build SignConfig from WindowsSettings
    let sign_config = kodegen_bundler_sign::windows::SignConfig {
        cert_path: cert_path.clone(),
        key_path: windows.key_path.clone(),
        password: windows.password.clone(),
        timestamp_url: windows.timestamp_url.clone()
            .or_else(|| Some("http://timestamp.digicert.com".to_string())),
        app_name: Some(settings.product_name().to_string()),
        app_url: settings.homepage().map(|s| s.to_string()),
    };
    
    // Sign the binary
    kodegen_bundler_sign::windows::sign_binary(binary_path, &sign_config)
        .map_err(|e| crate::bundler::Error::GenericError(format!(
            "Windows code signing failed: {}", e
        )))?;
    
    // Generate integrity hash
    let hash = kodegen_bundler_sign::windows::generate_integrity_hash(binary_path)
        .map_err(|e| crate::bundler::Error::GenericError(format!(
            "Hash generation failed: {}", e
        )))?;
    
    log::info!("✓ Successfully signed {} (SHA-256: {})", binary_path.display(), &hash[..16]);
    
    Ok(())
}

/// Check if Windows signing is configured
///
/// Returns true if cert_path is set in WindowsSettings
pub fn should_sign(settings: &Settings) -> bool {
    settings.bundle_settings().windows.cert_path.is_some()
}
