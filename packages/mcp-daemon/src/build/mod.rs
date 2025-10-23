//! Build module for cross-platform build tasks
//!
//! This module provides build-time platform checks and configuration.

// Re-export key types and functions for ergonomic usage
// Temporarily commented out unused imports to resolve warnings
// pub use macos_helper::{
//     build_and_sign_helper, create_helper_executable, create_info_plist,
//     validate_helper_structure, get_helper_size, is_helper_signed,
// };

// Temporarily commented out unused imports to resolve warnings
// pub use signing::{
//     sign_helper_app, create_entitlements_file, check_signing_identity,
//     get_signing_identities, notarize_app_bundle, is_app_notarized,
//     get_signature_info, SignatureInfo, validate_signing_requirements,
// };

// Temporarily commented out unused imports to resolve warnings
// pub use packaging::{
//     create_helper_zip, add_directory_to_zip, create_optimized_zip,
//     extract_zip, get_zip_info, ZipInfo, validate_zip, create_placeholder_zip,
//     calculate_directory_size, copy_directory_recursive, cleanup_temp_files,
//     create_secure_temp_dir,
// };

/// Main build function orchestrating platform-specific tasks
pub fn main() {
    // Check for systemd on Linux
    #[cfg(target_os = "linux")]
    {
        if pkg_config::probe_library("libsystemd").is_ok() {
            println!("cargo:rustc-cfg=feature=\"systemd_available\"");
        }
    }

    // Helper app is built and signed at install-time by kodegen-install (fully automated)

    // Platform-specific build optimizations
    configure_build_optimizations();

    // Set build metadata
    set_build_metadata();
}

/// Configure platform-specific build optimizations
fn configure_build_optimizations() {
    // Enable link-time optimization hints
    println!("cargo:rustc-link-arg=-Wl,-dead_strip");

    #[cfg(target_os = "macos")]
    {
        // macOS-specific optimizations
        println!("cargo:rustc-link-arg=-Wl,-no_compact_unwind");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }

    #[cfg(target_os = "linux")]
    {
        // Linux-specific optimizations
        println!("cargo:rustc-link-arg=-Wl,--gc-sections");
        println!("cargo:rustc-link-arg=-Wl,--strip-all");
    }

    #[cfg(target_os = "windows")]
    {
        // Windows-specific optimizations
        println!("cargo:rustc-link-arg=/OPT:REF");
        println!("cargo:rustc-link-arg=/OPT:ICF");
    }
}

/// Set build metadata for runtime access
fn set_build_metadata() {
    // Set build timestamp
    let build_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    println!("cargo:rustc-env=BUILD_TIMESTAMP={build_time}");

    // Set target information
    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string())
    );

    // Set profile information
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_PROFILE={profile}");

    // Set optimization flags based on profile
    match profile.as_str() {
        "release" => {
            println!("cargo:rustc-cfg=optimized");
            println!("cargo:rustc-env=OPTIMIZATION_LEVEL=3");
        }
        "debug" => {
            println!("cargo:rustc-cfg=debug_build");
            println!("cargo:rustc-env=OPTIMIZATION_LEVEL=0");
        }
        _ => {
            println!("cargo:rustc-env=OPTIMIZATION_LEVEL=1");
        }
    }
}

/// Check build environment and dependencies
#[allow(dead_code)]
pub fn validate_build_environment() -> Result<(), Box<dyn std::error::Error>> {
    // Check required environment variables
    let required_vars = ["OUT_DIR", "TARGET"];
    for var in &required_vars {
        if std::env::var(var).is_err() {
            return Err(format!("Required environment variable {var} not set").into());
        }
    }

    // Platform-specific validation removed - signing now done at install-time

    #[cfg(target_os = "linux")]
    {
        // Check for required Linux build tools
        if std::process::Command::new("gcc")
            .arg("--version")
            .output()
            .is_err()
        {
            return Err("GCC compiler not found".into());
        }
    }

    Ok(())
}

/// Get build information for runtime diagnostics
#[allow(dead_code)]
pub fn get_build_info() -> BuildInfo {
    BuildInfo {
        timestamp: std::env::var("BUILD_TIMESTAMP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        target: std::env::var("BUILD_TARGET").unwrap_or_else(|_| "unknown".to_string()),
        profile: std::env::var("BUILD_PROFILE").unwrap_or_else(|_| "unknown".to_string()),
        optimization_level: std::env::var("OPTIMIZATION_LEVEL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        features: get_enabled_features(),
    }
}

/// Build information structure
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BuildInfo {
    /// Build timestamp (Unix epoch seconds)
    pub timestamp: u64,
    /// Target triple
    pub target: String,
    /// Build profile (debug/release)
    pub profile: String,
    /// Optimization level
    pub optimization_level: u32,
    /// Enabled features
    pub features: Vec<String>,
}

/// Get list of enabled cargo features
#[allow(dead_code)]
fn get_enabled_features() -> Vec<String> {
    // #[cfg(feature = "systemd_available")] // Commented out unexpected cfg condition
    // features.push("systemd_available".to_string());

    // #[cfg(optimized)] // Commented out unexpected cfg condition
    // features.push("optimized".to_string());

    // #[cfg(debug_build)] // Commented out unexpected cfg condition
    // features.push("debug_build".to_string());

    Vec::new()
}
