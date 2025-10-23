//! macOS DMG disk image creator.
//!
//! Creates professional drag-to-install DMG files using the native hdiutil tool.
//! The DMG includes the .app bundle and an Applications symlink for easy installation.

use crate::bundler::{
    error::{Context, ErrorExt, Result},
    settings::Settings,
    utils::fs,
};
use std::{
    fs as std_fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Bundle project as DMG disk image
///
/// # Process
/// 1. Find existing .app or create new one via app::bundle_project()
/// 2. Create temporary staging directory
/// 3. Copy .app into staging directory
/// 4. Sign and notarize the staged .app (Task 12 integration)
/// 5. Create Applications symlink for drag-to-install
/// 6. Generate DMG using hdiutil with UDZO compression
/// 7. Sign DMG if signing identity configured
/// 8. Clean up temporary files
///
/// # Returns
/// Vector containing path to created DMG file.
///
/// # Example
/// ```no_run
/// # use std::path::PathBuf;
/// # type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// # struct Settings;
/// # fn bundle_project(settings: &Settings) -> Result<Vec<PathBuf>> { Ok(vec![]) }
/// # fn example() -> Result<()> {
/// # let settings = Settings;
/// let paths = bundle_project(&settings)?;
/// if !paths.is_empty() {
///     println!("Created DMG: {}", paths[0].display());
/// }
/// # Ok(())
/// # }
/// ```
pub fn bundle_project(settings: &Settings) -> Result<Vec<PathBuf>> {
    log::info!("Creating DMG for {}", settings.product_name());
    
    // Step 1: Find or create .app bundle
    let app_bundle_path = find_or_create_app_bundle(settings)?;
    
    // Step 2: Prepare DMG output directory
    let output_dir = settings.project_out_directory().join("bundle/dmg");
    std_fs::create_dir_all(&output_dir)
        .fs_context("creating DMG output directory", &output_dir)?;
    
    // Step 3: Create DMG file
    let dmg_path = create_dmg(settings, &app_bundle_path, &output_dir)?;
    
    // Step 4: Sign DMG if configured
    if should_sign_dmg(settings) {
        super::sign::sign_dmg(&dmg_path, settings)?;
    }
    
    Ok(vec![dmg_path])
}

/// Find existing .app bundle or create new one
///
/// # Logic
/// 1. Check if .app exists in expected location: `bundle/macos/{ProductName}.app`
/// 2. If found and is directory → use existing
/// 3. If not found → call `app::bundle_project()` to create it
///
/// # Returns
/// PathBuf to the .app bundle
fn find_or_create_app_bundle(settings: &Settings) -> Result<PathBuf> {
    let app_name = format!("{}.app", settings.product_name());
    let expected_path = settings
        .project_out_directory()
        .join("bundle/macos")
        .join(&app_name);
    
    if expected_path.exists() && expected_path.is_dir() {
        log::debug!("Using existing .app bundle: {}", expected_path.display());
        return Ok(expected_path);
    }
    
    // Create .app bundle using existing app bundler
    log::info!("Creating .app bundle for DMG...");
    use super::app;
    let paths = app::bundle_project(settings)?;
    
    paths
        .into_iter()
        .next()
        .ok_or_else(|| crate::bundler::Error::GenericError(
            "Failed to create .app bundle".into()
        ))
}

/// Create DMG from .app bundle using hdiutil
///
/// # DMG Creation Steps
/// 1. Create temporary staging directory using tempfile crate
/// 2. Copy .app bundle to staging directory
/// 3. Sign and notarize the staged .app (Task 12: BEFORE DMG creation)
/// 4. Create Applications symlink: `staging/Applications -> /Applications`
/// 5. Run hdiutil create with:
///    - `-volname`: Product name (shown when mounted)
///    - `-srcfolder`: Staging directory path
///    - `-ov`: Overwrite if exists
///    - `-format UDZO`: Compressed read-only (zlib compression)
/// 6. Verify hdiutil succeeded
/// 7. Automatic cleanup (tempfile handles it)
///
/// # DMG Naming Convention
/// Format: `{ProductName}-{Version}.dmg`
/// Examples:
/// - `MyApp-1.0.0.dmg`
/// - `CoolTool-2.3.1.dmg`
///
/// # Returns
/// PathBuf to created DMG file
fn create_dmg(
    settings: &Settings,
    app_bundle: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    let dmg_name = format!(
        "{}-{}.dmg",
        settings.product_name(),
        settings.version_string()
    );
    let dmg_path = output_dir.join(&dmg_name);
    
    // Remove old DMG if exists
    if dmg_path.exists() {
        std_fs::remove_file(&dmg_path)
            .fs_context("removing old DMG file", &dmg_path)?;
    }
    
    // Create temporary staging directory
    let temp_dir = tempfile::tempdir()
        .map_err(|e| crate::bundler::Error::GenericError(
            format!("Failed to create temporary directory for DMG contents: {}", e)
        ))?;
    let staging_path = temp_dir.path();
    
    // Copy .app bundle to staging directory
    let app_name = app_bundle
        .file_name()
        .ok_or_else(|| crate::bundler::Error::GenericError(
            "Invalid app bundle path".into()
        ))?;
    let staged_app = staging_path.join(app_name);
    
    log::debug!("Copying .app to staging: {}", staged_app.display());
    fs::copy_dir(app_bundle, &staged_app)
        .with_context(|| format!("copying .app bundle to staging directory: {}", staged_app.display()))?;
    
    // Task 12: Sign and notarize the .app bundle BEFORE creating the DMG
    // This ensures the .app inside the DMG is properly signed and notarized
    if settings.bundle_settings().macos.signing_identity.is_some() {
        super::sign::sign_app(&staged_app, settings)?;
    }
    
    if super::sign::should_notarize(settings) {
        super::sign::notarize_app(&staged_app, settings)?;
    }
    
    // Create Applications symlink for drag-to-install UX
    #[cfg(unix)]
    {
        let applications_link = staging_path.join("Applications");
        std::os::unix::fs::symlink("/Applications", &applications_link)
            .fs_context("creating Applications symlink", &applications_link)?;
    }
    
    // Determine if customization is needed
    let dmg_settings = &settings.bundle_settings().dmg;
    let needs_customization = dmg_settings.background.is_some() 
        || dmg_settings.window_size.is_some();

    // Choose format: UDRW if customizing (so changes persist), UDZO if not
    let dmg_format = if needs_customization { "UDRW" } else { "UDZO" };

    log::info!("Creating DMG with format {}...", dmg_format);
    
    let staging_str = staging_path
        .to_str()
        .ok_or_else(|| crate::bundler::Error::GenericError(
            "Invalid staging path (contains non-UTF8 characters)".into()
        ))?;
    
    let dmg_str = dmg_path
        .to_str()
        .ok_or_else(|| crate::bundler::Error::GenericError(
            "Invalid DMG path (contains non-UTF8 characters)".into()
        ))?;
    
    let output = Command::new("hdiutil")
        .args([
            "create",
            "-volname",
            settings.product_name(),
            "-srcfolder",
            staging_str,
            "-ov", // Overwrite if exists
            "-format",
            dmg_format, // UDRW if customizing, UDZO if not
            dmg_str,
        ])
        .output()
        .map_err(|e| crate::bundler::Error::GenericError(
            format!("Failed to execute hdiutil command: {}", e)
        ))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::bundler::Error::GenericError(
            format!("hdiutil failed: {}", stderr)
        ));
    }
    
    log::info!("✓ Created {} DMG: {}", dmg_format, dmg_path.display());
    
    // tempfile automatically cleans up staging directory
    drop(temp_dir);
    
    // Apply customizations and convert to compressed format
    if needs_customization {
        apply_dmg_customizations(&dmg_path, settings)?;
        
        // Convert UDRW to UDZO for final compressed DMG
        convert_dmg_to_compressed(&dmg_path)?;
    }
    
    Ok(dmg_path)
}

/// Apply DMG customizations (background image and window size)
///
/// # Process
/// 1. Mount DMG in read-write mode
/// 2. Copy background image to .background folder (if configured)
/// 3. Run AppleScript to customize window appearance
/// 4. Wait for .DS_Store file to be created
/// 5. Detach DMG
///
/// # Background
/// DMG appearance customization requires:
/// - Mounting the DMG to modify its .DS_Store file
/// - Using AppleScript to set Finder window properties
/// - The .DS_Store file persists these settings when DMG is unmounted
fn apply_dmg_customizations(dmg_path: &Path, settings: &Settings) -> Result<()> {
    log::info!("Applying DMG customizations...");
    
    let dmg_settings = &settings.bundle_settings().dmg;
    
    // Step 1: Mount DMG in read-write mode
    let volume_name = settings.product_name();
    let mount_point = mount_dmg_rw(dmg_path, volume_name)?;
    
    // Step 2: Copy background image if configured
    if let Some(bg_path) = &dmg_settings.background {
        let bg_dir = mount_point.join(".background");
        std_fs::create_dir_all(&bg_dir)
            .fs_context("creating .background directory", &bg_dir)?;
        
        let bg_filename = bg_path
            .file_name()
            .ok_or_else(|| crate::bundler::Error::GenericError(
                "Invalid background image path".into()
            ))?;
        
        let dest_bg = bg_dir.join(bg_filename);
        std_fs::copy(bg_path, &dest_bg)
            .fs_context("copying background image", &dest_bg)?;
        
        log::debug!("Copied background image to {}", dest_bg.display());
    }
    
    // Step 3: Run AppleScript to customize window
    let window_size = dmg_settings.window_size.unwrap_or((600, 400));
    let has_background = dmg_settings.background.is_some();
    
    run_dmg_applescript(volume_name, settings, window_size, has_background)?;
    
    // Step 4: Detach DMG
    detach_dmg(volume_name)?;
    
    log::info!("✓ DMG customizations applied");
    
    Ok(())
}

/// Mount DMG in read-write mode
///
/// Returns the mount point path
fn mount_dmg_rw(dmg_path: &Path, volume_name: &str) -> Result<PathBuf> {
    log::debug!("Mounting DMG for customization...");
    
    let dmg_str = dmg_path
        .to_str()
        .ok_or_else(|| crate::bundler::Error::GenericError(
            "DMG path contains non-UTF8 characters".into()
        ))?;
    
    let output = Command::new("hdiutil")
        .args(["attach", dmg_str, "-readwrite", "-noverify", "-nobrowse"])
        .output()
        .map_err(|e| crate::bundler::Error::GenericError(
            format!("Failed to mount DMG: {}", e)
        ))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::bundler::Error::GenericError(
            format!("Failed to mount DMG: {}", stderr)
        ));
    }
    
    // Mount point is /Volumes/{volume_name}
    let mount_point = PathBuf::from(format!("/Volumes/{}", volume_name));
    
    // Wait for mount to be ready
    let max_retries = 10;
    for i in 0..max_retries {
        if mount_point.exists() {
            log::debug!("DMG mounted at {}", mount_point.display());
            return Ok(mount_point);
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        if i == max_retries - 1 {
            return Err(crate::bundler::Error::GenericError(
                format!("DMG mount point not found after {} retries", max_retries)
            ));
        }
    }
    
    Ok(mount_point)
}

/// Escape special characters for AppleScript string literals
///
/// Escapes backslashes and double quotes to prevent script injection
/// and syntax errors when product names contain special characters.
///
/// # Examples
/// ```
/// # fn escape_applescript_string(s: &str) -> String {
/// #     s.replace('\\', r"\\").replace('"', r#"\""#)
/// # }
/// assert_eq!(escape_applescript_string("My\"App"), "My\\\"App");
/// assert_eq!(escape_applescript_string("Path\\File"), "Path\\\\File");
/// ```
fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', r"\\")
     .replace('"', r#"\""#)
}

/// Run AppleScript to customize DMG window appearance
fn run_dmg_applescript(
    volume_name: &str,
    settings: &Settings,
    window_size: (u32, u32),
    has_background: bool,
) -> Result<()> {
    log::debug!("Running AppleScript to customize DMG window...");
    
    let app_name = format!("{}.app", settings.product_name());
    let (width, height) = window_size;
    
    // Escape strings for safe AppleScript interpolation
    let escaped_volume = escape_applescript_string(volume_name);
    let escaped_app = escape_applescript_string(&app_name);
    
    // Extract and escape background filename
    let escaped_bg_filename = if has_background {
        let bg_filename = settings.bundle_settings()
            .dmg
            .background
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("background.png");
        escape_applescript_string(bg_filename)
    } else {
        String::new()
    };
    
    // Build AppleScript (use escaped variables)
    let script = format!(
        r#"
        tell application "Finder"
            tell disk "{volume_name}"
                open
                set current view of container window to icon view
                set toolbar visible of container window to false
                set statusbar visible of container window to false
                set bounds of container window to {{100, 100, {right}, {bottom}}}
                set viewOptions to icon view options of container window
                set arrangement of viewOptions to not arranged
                set icon size of viewOptions to 72
                {background_clause}
                set position of item "{app_name}" to {{180, 170}}
                set position of item "Applications" to {{480, 170}}
                close
                open
                update without registering applications
                delay 2
            end tell
        end tell
        "#,
        volume_name = escaped_volume,
        right = 100 + width,
        bottom = 100 + height,
        app_name = escaped_app,
        background_clause = if has_background {
            format!(
                r#"set background picture of viewOptions to file ".background:{bg_filename}""#,
                bg_filename = escaped_bg_filename
            )
        } else {
            String::new()
        }
    );
    
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| crate::bundler::Error::GenericError(
            format!("Failed to run AppleScript: {}", e)
        ))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("AppleScript execution had issues: {}", stderr);
        // Don't fail - appearance customization is non-critical
    }
    
    Ok(())
}

/// Detach (unmount) DMG
fn detach_dmg(volume_name: &str) -> Result<()> {
    log::debug!("Detaching DMG...");
    
    let mount_point = format!("/Volumes/{}", volume_name);
    
    // Wait for .DS_Store to be written
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let output = Command::new("hdiutil")
        .args(["detach", &mount_point])
        .output()
        .map_err(|e| crate::bundler::Error::GenericError(
            format!("Failed to detach DMG: {}", e)
        ))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("DMG detach had issues: {}", stderr);
        // Try force detach
        Command::new("hdiutil")
            .args(["detach", &mount_point, "-force"])
            .output()
            .ok();
    }
    
    Ok(())
}

/// Convert read-write DMG (UDRW) to compressed read-only (UDZO)
///
/// This must be done AFTER customizations are applied and the DMG is detached.
/// The conversion creates a new compressed DMG and replaces the original.
///
/// # Process
/// 1. Create temporary output path for compressed DMG
/// 2. Run hdiutil convert with UDZO format
/// 3. Remove original UDRW DMG
/// 4. Rename compressed DMG to original path
///
/// # Background
/// We cannot customize a UDZO DMG because it's compressed and read-only.
/// Changes made to a mounted UDZO with -readwrite are stored in a shadow
/// file which is discarded on detach. The correct workflow is:
/// UDRW → customize → detach → convert to UDZO.
fn convert_dmg_to_compressed(dmg_path: &Path) -> Result<()> {
    log::info!("Converting DMG to compressed format...");
    
    let dmg_str = dmg_path
        .to_str()
        .ok_or_else(|| crate::bundler::Error::GenericError(
            "DMG path contains non-UTF8 characters".into()
        ))?;
    
    // Create temporary path for compressed DMG
    let compressed_path = dmg_path.with_extension("dmg.compressed");
    let compressed_str = compressed_path
        .to_str()
        .ok_or_else(|| crate::bundler::Error::GenericError(
            "Compressed DMG path contains non-UTF8 characters".into()
        ))?;
    
    // Convert UDRW → UDZO
    let output = Command::new("hdiutil")
        .args([
            "convert",
            dmg_str,
            "-format",
            "UDZO",
            "-o",
            compressed_str,
        ])
        .output()
        .map_err(|e| crate::bundler::Error::GenericError(
            format!("Failed to convert DMG: {}", e)
        ))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::bundler::Error::GenericError(
            format!("DMG conversion failed: {}", stderr)
        ));
    }
    
    // Replace UDRW with UDZO
    std_fs::remove_file(dmg_path)
        .fs_context("removing UDRW DMG", dmg_path)?;
    std_fs::rename(&compressed_path, dmg_path)
        .fs_context("renaming compressed DMG", &compressed_path)?;
    
    log::info!("✓ DMG converted to compressed UDZO format");
    
    Ok(())
}

/// Check if DMG should be signed
///
/// Sign DMG when:
/// - ✅ `signing_identity` is configured in MacOsSettings
/// - ✅ Identity is NOT "-" (ad-hoc signature marker)
///
/// # Background
/// The "-" identity is Apple's marker for ad-hoc signatures (self-signing).
/// We skip external signing for ad-hoc signatures to avoid errors.
fn should_sign_dmg(settings: &Settings) -> bool {
    if let Some(identity) = &settings.bundle_settings().macos.signing_identity {
        identity != "-"
    } else {
        false
    }
}
