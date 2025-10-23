//! macOS code signing and notarization integration.
//!
//! This module provides integration between the bundler and the kodegen_sign
//! package, adapting bundler Settings to the sign crate's API.

use crate::bundler::{
    error::Result,
    settings::Settings,
};
use std::path::Path;

/// Sign a macOS app bundle using kodegen_sign
///
/// This function:
/// 1. Checks if signing is configured (signing_identity present)
/// 2. Calls kodegen_bundler_sign::macos::sign_with_entitlements with hardened runtime
/// 3. Verifies the signature
///
/// # Arguments
/// * `app_bundle` - Path to the .app bundle to sign
/// * `settings` - Bundler settings containing signing configuration
///
/// # Returns
/// * `Ok(())` - Signing succeeded or was skipped (no identity configured)
/// * `Err(Error)` - Signing failed
///
/// # Example
/// ```no_run
/// # use std::path::Path;
/// # type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// # struct Settings;
/// # fn sign_app(path: &Path, settings: &Settings) -> Result<()> { Ok(()) }
/// # fn example() -> Result<()> {
/// # let settings = Settings;
/// sign_app(Path::new("MyApp.app"), &settings)?;
/// # Ok(())
/// # }
/// ```
pub fn sign_app(app_bundle: &Path, settings: &Settings) -> Result<()> {
    let identity = match &settings.bundle_settings().macos.signing_identity {
        Some(id) => id,
        None => {
            log::info!("No signing identity configured, skipping signing");
            return Ok(());
        }
    };
    
    log::info!("Signing {} with identity '{}'", app_bundle.display(), identity);
    
    // Get entitlements path if configured
    let entitlements = settings.bundle_settings().macos.entitlements.as_deref();
    
    // Sign with hardened runtime (required for notarization)
    kodegen_bundler_sign::macos::sign_with_entitlements(
        app_bundle,
        identity,
        entitlements,
        true, // hardened_runtime = true
    )
    .map_err(|e| crate::bundler::Error::GenericError(format!(
        "Code signing failed: {}", e
    )))?;
    
    log::info!("✓ Successfully signed {}", app_bundle.display());
    
    Ok(())
}

/// Notarize a macOS app bundle with Apple
///
/// This function:
/// 1. Checks if notarization should be skipped
/// 2. Loads credentials from environment variables
/// 3. Calls kodegen_bundler_sign::macos::notarize
/// 4. Waits for completion and staples the ticket
///
/// # Arguments
/// * `app_bundle` - Path to the signed .app bundle
/// * `settings` - Bundler settings containing notarization configuration
///
/// # Returns
/// * `Ok(())` - Notarization succeeded or was skipped
/// * `Err(Error)` - Notarization failed
///
/// # Environment Variables
/// **API Key (Recommended):**
/// - `APPLE_API_KEY` - Key ID from App Store Connect
/// - `APPLE_API_ISSUER` - Issuer ID from App Store Connect  
/// - `APPLE_API_KEY_PATH` - Path to AuthKey_*.p8 file (optional, auto-searched)
///
/// **Apple ID (Legacy):**
/// - `APPLE_ID` - Your Apple ID email
/// - `APPLE_PASSWORD` - App-specific password
/// - `APPLE_TEAM_ID` - Your team ID
///
/// # Example
/// ```bash
/// export APPLE_API_KEY="ABCD123456"
/// export APPLE_API_ISSUER="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
/// export APPLE_API_KEY_PATH="~/.keys/AuthKey_ABCD123456.p8"
/// ```
pub fn notarize_app(app_bundle: &Path, settings: &Settings) -> Result<()> {
    if settings.bundle_settings().macos.skip_notarization {
        log::info!("Notarization disabled (skip_notarization = true)");
        return Ok(());
    }
    
    log::info!("Notarizing {}", app_bundle.display());
    
    // If APPLE_API_KEY_CONTENT is set, write to file and use that path directly
    let auth = if let Some(key_path) = kodegen_bundler_sign::macos::ensure_api_key_file()
        .map_err(|e| crate::bundler::Error::GenericError(format!(
            "Failed to write API key file: {}", e
        )))?
    {
        // We wrote the key, build auth directly with the path (no env var indirection)
        let key_id = std::env::var("APPLE_API_KEY")
            .map_err(|_| crate::bundler::Error::GenericError(
                "APPLE_API_KEY not set".to_string()
            ))?;
        let issuer_id = std::env::var("APPLE_API_ISSUER")
            .map_err(|_| crate::bundler::Error::GenericError(
                "APPLE_API_ISSUER not set".to_string()
            ))?;
        
        kodegen_bundler_sign::macos::NotarizationAuth::ApiKey {
            key_id,
            issuer_id,
            key_path,
        }
    } else {
        // No key content in env, use from_env() which checks APPLE_API_KEY_PATH or searches
        kodegen_bundler_sign::macos::NotarizationAuth::from_env()
            .map_err(|e| crate::bundler::Error::GenericError(format!(
                "Failed to load notarization credentials: {}", e
            )))?
    };
    
    // Wait for notarization to complete
    let wait = true;
    
    // Notarize (will also staple unless skip_stapling is set)
    kodegen_bundler_sign::macos::notarize(app_bundle, &auth, wait)
        .map_err(|e| crate::bundler::Error::GenericError(format!(
            "Notarization failed: {}", e
        )))?;
    
    log::info!("✓ Successfully notarized {}", app_bundle.display());
    
    Ok(())
}

/// Check if an app should be notarized
///
/// Returns true if:
/// - skip_notarization is false
/// - Notarization credentials are available in environment
pub fn should_notarize(settings: &Settings) -> bool {
    !settings.bundle_settings().macos.skip_notarization && 
    kodegen_bundler_sign::macos::NotarizationAuth::from_env().is_ok()
}

/// Sign a DMG file using kodegen_sign
///
/// DMG signing differs from .app signing:
/// - No entitlements needed (pass None)
/// - No hardened runtime needed (pass false)
/// - Just basic code signature for integrity
///
/// # Arguments
/// * `dmg_path` - Path to the .dmg file to sign
/// * `settings` - Bundler settings containing signing configuration
///
/// # Returns
/// * `Ok(())` - Signing succeeded or was skipped
/// * `Err(Error)` - Signing failed
///
/// # Example
/// ```no_run
/// # use std::path::Path;
/// # type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// # struct Settings;
/// # fn sign_dmg(path: &Path, settings: &Settings) -> Result<()> { Ok(()) }
/// # fn example() -> Result<()> {
/// # let settings = Settings;
/// sign_dmg(Path::new("MyApp.dmg"), &settings)?;
/// # Ok(())
/// # }
/// ```
pub fn sign_dmg(dmg_path: &Path, settings: &Settings) -> Result<()> {
    let identity = match &settings.bundle_settings().macos.signing_identity {
        Some(id) => id,
        None => {
            log::info!("No signing identity configured, skipping DMG signing");
            return Ok(());
        }
    };
    
    log::info!("Signing DMG {} with identity '{}'", dmg_path.display(), identity);
    
    // Sign DMG without entitlements or hardened runtime
    kodegen_bundler_sign::macos::sign_with_entitlements(
        dmg_path,
        identity,
        None,    // no entitlements for DMG
        false,   // no hardened runtime for DMG
    )
    .map_err(|e| crate::bundler::Error::GenericError(format!(
        "DMG signing failed: {}", e
    )))?;
    
    log::info!("✓ Successfully signed DMG: {}", dmg_path.display());
    
    Ok(())
}
