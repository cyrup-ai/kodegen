//! Windows MSI installer creator using WiX Toolset.
//!
//! This module creates professional Windows installers (.msi) using the WiX Toolset v3.14.
//! The bundler automatically downloads WiX if not present, generates WiX source files from
//! templates, and produces MSI packages with proper upgrade handling and shortcuts.
//!
//! ## Architecture Support
//! - x64 (64-bit Intel/AMD)
//! - x86 (32-bit Intel/AMD)  
//! - arm64 (64-bit ARM)
//!
//! ## WiX Workflow
//! 1. Download WiX binaries to cache directory
//! 2. Generate main.wxs from handlebars template
//! 3. Run candle.exe to compile .wxs → .wixobj
//! 4. Run light.exe to link .wixobj → .msi

use crate::bundler::{
    error::{Context, ErrorExt, Result, bail},
    Error,
    settings::Settings,
    utils::http,
};
use super::sign;
use handlebars::Handlebars;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;

// WiX Toolset v3.14 download information
// Source: https://github.com/wixtoolset/wix3/releases/tag/wix3141rtm
const WIX_URL: &str = "https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip";
const WIX_SHA256: &str = "6ac824e1642d6f7277d0ed7ea09411a508f6116ba6fae0aa5f2c7daa2ff43d31";

// Required WiX files for MSI creation
// Missing any of these indicates corrupted installation
const WIX_REQUIRED_FILES: &[&str] = &[
    "candle.exe",        // WiX compiler
    "candle.exe.config", // Compiler configuration
    "light.exe",         // WiX linker
    "light.exe.config",  // Linker configuration
    "WixUIExtension.dll", // UI components
    "WixUtilExtension.dll", // Utility components
    "darice.cub",        // ICE validation
    "wconsole.dll",      // Console helpers
    "winterop.dll",      // Interop
    "wix.dll",           // Core WiX
];

// Namespace UUID for generating v5 UUIDs
// This specific UUID is used to generate deterministic upgrade codes
// from bundle identifiers (same ID = same upgrade code)
const UUID_NAMESPACE: [u8; 16] = [
    0xfd, 0x85, 0x95, 0xa8, 0x17, 0xa3, 0x47, 0x4e,
    0xa6, 0x16, 0x76, 0x14, 0x8d, 0xfa, 0x0c, 0x7b,
];

/// Creates a Command for executing Windows binaries.
/// On Windows: Runs the .exe directly
/// On Linux: Wraps with Wine
fn windows_command(exe_path: &Path) -> Command {
    #[cfg(target_os = "windows")]
    {
        Command::new(exe_path)
    }

    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("wine");
        cmd.arg(exe_path);
        cmd
    }
}

/// Bundle project as MSI installer.
///
/// Returns vector containing path to created .msi file.
pub fn bundle_project(settings: &Settings) -> Result<Vec<PathBuf>> {
    log::info!("Building MSI installer for {}", settings.product_name());
    
    // Get or download WiX toolset
    let wix_path = get_wix_toolset()?;
    
    // Verify WiX installation is complete
    verify_wix_installation(&wix_path)?;
    
    // Build MSI
    build_msi(settings, &wix_path)
}

/// Get WiX toolset, downloading if necessary.
///
/// WiX is cached in:
/// - Linux/macOS: ~/.cache/cyrup/WixTools314  
/// - Windows: %LOCALAPPDATA%\cyrup\WixTools314
fn get_wix_toolset() -> Result<PathBuf> {
    // Determine cache directory
    let cache_dir = if let Ok(cache) = std::env::var("CYRUP_CACHE_DIR") {
        PathBuf::from(cache)
    } else {
        dirs::cache_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("cyrup")
    };
    
    let wix_path = cache_dir.join("WixTools314");
    
    // Check if WiX already exists and is complete
    if wix_path.exists() && WIX_REQUIRED_FILES.iter().all(|f| wix_path.join(f).exists()) {
        log::debug!("WiX toolset found at {}", wix_path.display());
        return Ok(wix_path);
    }
    
    // Remove incomplete installation
    if wix_path.exists() {
        log::warn!("WiX installation incomplete, re-downloading...");
        std::fs::remove_dir_all(&wix_path)
            .fs_context("removing incomplete WiX directory", &wix_path)?;
    }
    
    log::info!("Downloading WiX toolset v3.14...");
    
    // Download and verify
    let data = http::download_and_verify(
        WIX_URL,
        WIX_SHA256,
        http::HashAlgorithm::Sha256,
    )?;
    
    log::info!("Extracting WiX toolset to {}", wix_path.display());
    
    // Create cache directory
    std::fs::create_dir_all(&cache_dir)
        .fs_context("creating cache directory", &cache_dir)?;
    
    // Extract to cache
    http::extract_zip(&data, &wix_path)?;
    
    Ok(wix_path)
}

/// Verify WiX installation has all required files.
fn verify_wix_installation(wix_path: &Path) -> Result<()> {
    let missing: Vec<_> = WIX_REQUIRED_FILES
        .iter()
        .filter(|f| !wix_path.join(f).exists())
        .collect();
    
    if !missing.is_empty() {
        bail!(
            "WiX installation missing required files: {}",
            missing.iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    
    Ok(())
}

/// Build MSI installer for target architecture.
fn build_msi(settings: &Settings, wix_path: &Path) -> Result<Vec<PathBuf>> {
    // Map architecture to WiX terminology
    let arch = match settings.binary_arch() {
        crate::bundler::settings::Arch::X86_64 => "x64",
        crate::bundler::settings::Arch::X86 => "x86",
        crate::bundler::settings::Arch::AArch64 => "arm64",
        _ => bail!("Unsupported architecture for MSI: {:?}", settings.binary_arch()),
    };
    
    // Create output directory: bundle/msi/{arch}/
    let output_dir = settings.project_out_directory()
        .join("bundle")
        .join("msi")
        .join(arch);
    
    std::fs::create_dir_all(&output_dir)
        .fs_context("creating MSI output directory", &output_dir)?;
    
    // Generate WiX source file (.wxs)
    let wxs_path = generate_wxs(settings, &output_dir, arch)?;
    
    // Compile: .wxs → .wixobj
    let wixobj_path = run_candle(wix_path, &wxs_path, &output_dir, arch)?;
    
    // Link: .wixobj → .msi
    let msi_path = run_light(wix_path, &wixobj_path, settings, &output_dir, arch)?;
    
    // Sign the MSI if configured
    if sign::should_sign(settings) {
        sign::sign_file(&msi_path, settings)
            .context("signing MSI installer")?;
    }
    
    log::info!("✓ Created MSI: {}", msi_path.display());
    
    Ok(vec![msi_path])
}

/// Generate WiX source (.wxs) file from template.
fn generate_wxs(
    settings: &Settings,
    output_dir: &Path,
    arch: &str,
) -> Result<PathBuf> {
    let mut handlebars = Handlebars::new();
    
    // CRITICAL: Disable HTML escaping for XML content
    // WiX XML must not have &quot; etc. - it needs literal quotes
    handlebars.register_escape_fn(handlebars::no_escape);
    
    let mut data = BTreeMap::new();
    
    // Basic product info
    data.insert("product_name", settings.product_name().to_string());
    data.insert("version", normalize_version(settings.version_string())?);
    
    // Manufacturer (publisher or bundle ID)
    let manufacturer = settings.bundle_settings()
        .publisher
        .as_ref()
        .or(settings.bundle_settings().identifier.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("Unknown");
    data.insert("manufacturer", manufacturer.to_string());
    
    // GUIDs - CRITICAL for Windows Installer
    // Upgrade code: UUIDv5 (deterministic) - MUST stay same for upgrades to work
    let bundle_id = settings.bundle_settings()
        .identifier
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or(settings.product_name());
    let upgrade_code = generate_guid(bundle_id.as_bytes());
    data.insert("upgrade_code", upgrade_code.to_string());
    
    // Component GUID: UUIDv4 (random) - unique per build
    let component_guid = Uuid::new_v4();
    data.insert("component_guid", component_guid.to_string());
    
    // Architecture settings
    data.insert("arch", arch.to_string());
    
    // Win64 flag - required for proper 64-bit installation
    let win64 = if arch == "x64" || arch == "arm64" { "yes" } else { "no" };
    data.insert("win64", win64.to_string());
    
    // Program Files folder - 64-bit vs 32-bit
    let program_files = if arch == "x64" || arch == "arm64" {
        "ProgramFiles64Folder"
    } else {
        "ProgramFilesFolder"
    };
    data.insert("program_files", program_files.to_string());
    
    // Include ALL binaries in MSI package
    let binaries: Vec<BTreeMap<String, String>> = settings.binaries()
        .iter()
        .map(|bin| {
            let mut bin_data = BTreeMap::new();
            bin_data.insert("name".to_string(), bin.name().to_string());
            bin_data.insert("path".to_string(), 
                settings.binary_path(bin).display().to_string());
            bin_data.insert("is_main".to_string(), 
                bin.main().to_string());
            
            // Generate unique IDs for WiX Components and Files
            // Component ID format: Binary_kodegen, Binary_kodegend, Binary_kodegen_install
            // File ID format: File_kodegen, File_kodegend, File_kodegen_install
            bin_data.insert("component_id".to_string(),
                format!("Binary_{}", bin.name()));
            bin_data.insert("file_id".to_string(),
                format!("File_{}", bin.name()));
            
            bin_data
        })
        .collect();

    data.insert("binaries", binaries);

    // Main binary name for Start Menu shortcuts
    let main_binary = settings.binaries()
        .iter()
        .find(|b| b.main())
        .context("No main binary found in configuration")?;
        
    data.insert("main_binary_name", format!("{}.exe", main_binary.name()));
    
    // Custom branding images
    let wix_settings = &settings.bundle_settings().windows.wix;
    
    if let Some(banner) = &wix_settings.banner_path {
        data.insert("banner_path", banner.display().to_string());
    }
    
    if let Some(dialog) = &wix_settings.dialog_image_path {
        data.insert("dialog_image_path", dialog.display().to_string());
    }
    
    if let Some(license) = &wix_settings.license {
        data.insert("license_path", license.display().to_string());
    }
    
    // Register embedded template
    let template = include_str!("main.wxs");
    handlebars.register_template_string("main.wxs", template)
        .map_err(|e| Error::GenericError(format!("Failed to register WiX template: {}", e)))?;
    
    // Render template
    let wxs_content = handlebars.render("main.wxs", &data)
        .map_err(|e| Error::GenericError(format!("Failed to render WiX template: {}", e)))?;
    
    // Write to output directory
    let wxs_path = output_dir.join("main.wxs");
    std::fs::write(&wxs_path, wxs_content)
        .fs_context("writing WiX source file", &wxs_path)?;
    
    log::debug!("Generated WiX source: {}", wxs_path.display());
    
    Ok(wxs_path)
}

/// Run candle.exe (WiX compiler).
///
/// Compiles .wxs (XML source) into .wixobj (object file).
fn run_candle(
    wix_path: &Path,
    wxs_path: &Path,
    output_dir: &Path,
    arch: &str,
) -> Result<PathBuf> {
    log::info!("Running candle.exe (WiX compiler)...");
    
    let candle = wix_path.join("candle.exe");
    let wixobj_path = output_dir.join("main.wixobj");
    
    let wixobj_str = wixobj_path.to_str()
        .context("wixobj path contains invalid UTF-8")?;
    let wxs_str = wxs_path.to_str()
        .context("wxs path contains invalid UTF-8")?;

    let output = windows_command(&candle)
        .args(&[
            "-arch", arch,
            "-out", wixobj_str,
            wxs_str,
        ])
        .output()
        .map_err(|e| Error::GenericError(format!("Failed to execute candle.exe: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("candle.exe failed:\n{}", stderr);
    }
    
    log::debug!("Compiled to: {}", wixobj_path.display());
    
    Ok(wixobj_path)
}

/// Run light.exe (WiX linker).
///
/// Links .wixobj into final .msi installer.
fn run_light(
    wix_path: &Path,
    wixobj_path: &Path,
    settings: &Settings,
    output_dir: &Path,
    arch: &str,
) -> Result<PathBuf> {
    log::info!("Running light.exe (WiX linker)...");
    
    let light = wix_path.join("light.exe");
    
    // MSI filename: {product}_{version}_{arch}.msi
    let msi_name = format!(
        "{}_{}_{}.msi",
        settings.product_name().replace(' ', "_"),
        settings.version_string(),
        arch
    );
    
    let msi_path = output_dir.join(&msi_name);
    
    let msi_str = msi_path.to_str()
        .context("msi path contains invalid UTF-8")?;
    let wixobj_str = wixobj_path.to_str()
        .context("wixobj path contains invalid UTF-8")?;

    let output = windows_command(&light)
        .args(&[
            "-out", msi_str,
            wixobj_str,
            "-ext", "WixUIExtension",  // UI components
            "-ext", "WixUtilExtension", // Utility components
            "-sval", // Skip validation (faster builds)
        ])
        .output()
        .map_err(|e| Error::GenericError(format!("Failed to execute light.exe: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("light.exe failed:\n{}", stderr);
    }
    
    Ok(msi_path)
}

/// Generate GUID using UUIDv5 (deterministic).
///
/// UUIDv5 generates the same UUID for the same input,
/// which is critical for upgrade codes - they MUST remain
/// constant across versions for Windows Installer to recognize upgrades.
fn generate_guid(key: &[u8]) -> Uuid {
    let namespace = Uuid::from_bytes(UUID_NAMESPACE);
    Uuid::new_v5(&namespace, key)
}

/// Normalize version for WiX.
///
/// WiX requires version in format: major.minor.patch.build
/// All components must be numeric. Pre-release tags are not supported.
fn normalize_version(version: &str) -> Result<String> {
    let version = semver::Version::parse(version)
        .map_err(|e| Error::GenericError(format!("Invalid version string for MSI: {}", e)))?;
    
    // WiX version format: X.Y.Z.0
    Ok(format!(
        "{}.{}.{}.0",
        version.major,
        version.minor,
        version.patch
    ))
}
