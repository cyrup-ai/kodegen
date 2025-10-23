//! Certificate provisioning and helper building for code signing

use std::io::Write;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

/// Attempt to remove a file or directory, logging warnings on failure.
///
/// This function implements best-effort cleanup:
/// - Succeeds silently when cleanup works
/// - Logs detailed warnings when cleanup fails
/// - Never panics or returns errors (cleanup is best-effort)
/// - Handles both files and directories automatically
///
/// # Arguments
///
/// * `path` - Path to file or directory to remove
/// * `description` - Human-readable description of what's being cleaned up
pub fn cleanup_path<P: AsRef<std::path::Path>>(path: P, description: &str) {
    let path = path.as_ref();
    
    // If path doesn't exist, nothing to clean up
    if !path.exists() {
        return;
    }
    
    // Attempt removal based on type
    let result = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    
    // Log warnings on failure
    if let Err(e) = result {
        // NotFound is OK - race condition where file was already removed
        if e.kind() != std::io::ErrorKind::NotFound {
            let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
            let _ = writeln!(&mut buffer, "⚠️  Warning: Failed to cleanup {description}");
            let _ = buffer.reset();
            let _ = writeln!(&mut buffer, "   Path: {}", path.display());
            let _ = writeln!(&mut buffer, "   Error: {e}");
            
            // Provide actionable suggestions based on error kind
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    let _ = writeln!(&mut buffer, "   Suggestion: Check file permissions or try: sudo rm -rf {}", path.display());
                }
                std::io::ErrorKind::DirectoryNotEmpty => {
                    let _ = writeln!(&mut buffer, "   Suggestion: Directory may contain locked files");
                }
                _ => {
                    let _ = writeln!(&mut buffer, "   Suggestion: Manual cleanup may be needed");
                }
            }
            
            let _ = bufwtr.print(&buffer);
        }
    }
}

pub mod error;
pub mod config;
pub mod apple_api;

#[cfg(target_os = "macos")]
#[macro_use]
pub mod macos;

#[cfg(target_os = "macos")]
pub mod build_helper;

#[cfg(target_os = "macos")]
pub mod sign_helper;

#[cfg(target_os = "macos")]
pub mod package_helper;

#[cfg(target_os = "linux")]
pub mod linux;

// Windows signing module is available on all platforms
// Uses osslsigncode for cross-platform Authenticode signing
pub mod windows;

// Re-export common types
pub use config::{SetupConfig, PlatformConfig};
pub use error::SetupError;
pub use windows::sign_binary_with_fallback;

#[cfg(target_os = "macos")]
pub use build_helper::build_and_sign_helper;
