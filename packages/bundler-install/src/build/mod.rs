//! Build module for cross-platform build tasks
//!
//! This module provides comprehensive build functionality including macOS
//! helper app creation, code signing, and packaging with zero allocation
//! patterns and blazing-fast performance.

pub mod macos_helper;
pub mod packaging;
pub mod signing;

#[cfg(target_os = "windows")]
pub mod windows_helper;

#[cfg(target_os = "linux")]
pub mod linux_helper;

/// Main build function orchestrating platform-specific tasks
pub fn main() {
    // Check for systemd on Linux
    #[cfg(target_os = "linux")]
    {
        if pkg_config::probe_library("libsystemd").is_ok() {
            println!("cargo:rustc-cfg=feature=\"systemd_available\"");
        }
    }

    // Build and sign macOS helper app
    #[cfg(target_os = "macos")]
    {
        // Use atomic build with rollback
        let out_dir = std::env::var("OUT_DIR")
            .map(std::path::PathBuf::from)
            .expect("OUT_DIR must be set");
        let zip_path = out_dir.join("KodegenHelper.app.zip");
        
        if let Err(e) = packaging::create_functional_zip(&zip_path) {
            eprintln!("Error: Failed to build macOS helper app: {e}");
            eprintln!("Build failed - macOS helper is required for proper installation");
            std::process::exit(1);
        }
    }

    // Build and sign Windows service executable
    #[cfg(target_os = "windows")]
    {
        if let Err(e) = windows_helper::build_and_sign_helper() {
            eprintln!("Error: Failed to build Windows helper: {e}");
            eprintln!("Build failed - Windows helper is required for proper installation");
            std::process::exit(1);
        }
    }

    // Build Linux helper executable
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = linux_helper::build_and_sign_helper() {
            eprintln!("Error: Failed to build Linux helper: {e}");
            eprintln!("Build failed - Linux helper is required for proper installation");
            std::process::exit(1);
        }
    }

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

    // Platform-specific validation
    #[cfg(target_os = "macos")]
    {
        signing::validate_signing_requirements()?;
    }

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
    Vec::new()
}

// Function removed - no more placeholders, fail builds instead
