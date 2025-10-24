//! Notarization workflow for macOS apps
//!
//! Provides functions to upload apps to Apple's notarization service,
//! poll for completion, and staple notarization tickets.

use super::validation::validate_p8_file;
use crate::error::{Result, SetupError};
use serde::Deserialize;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use termcolor::WriteColor;

/// Notarization credentials
#[derive(Debug, Clone)]
pub enum NotarizationAuth {
    /// App Store Connect API key (recommended)
    ApiKey {
        key_id: String,
        issuer_id: String,
        key_path: std::path::PathBuf,
    },
    /// Apple ID with app-specific password
    AppleId {
        apple_id: String,
        password: String,
        team_id: String,
    },
}

impl NotarizationAuth {
    /// Load credentials from environment variables
    ///
    /// Priority order:
    /// 1. `APPLE_API_KEY` + `APPLE_API_ISSUER` + `APPLE_API_KEY_PATH`
    /// 2. `APPLE_ID` + `APPLE_PASSWORD` + `APPLE_TEAM_ID`
    pub fn from_env() -> Result<Self> {
        // Try API key first (modern approach)
        if let (Ok(key_id), Ok(issuer)) = (
            std::env::var("APPLE_API_KEY"),
            std::env::var("APPLE_API_ISSUER"),
        ) {
            let key_path = std::env::var("APPLE_API_KEY_PATH")
                .map(std::path::PathBuf::from)
                .or_else(|_| {
                    // Auto-search standard locations
                    find_p8_key(&key_id)
                        .ok_or_else(|| SetupError::MissingConfig(format!(
                            "APPLE_API_KEY_PATH not set and AuthKey_{key_id}.p8 not found in standard locations"
                        )))
                })?;

            // Validate key file before use
            validate_p8_file(&key_path).map_err(|e| {
                SetupError::InvalidConfig(format!(
                    "Invalid API key file:\n{}\n\n\
                     Key ID: {}\n\
                     Path: {}\n\n\
                     Download your API key from:\n\
                     https://appstoreconnect.apple.com/access/api",
                    e,
                    key_id,
                    key_path.display()
                ))
            })?;

            return Ok(Self::ApiKey {
                key_id,
                issuer_id: issuer,
                key_path,
            });
        }

        // Try Apple ID (legacy)
        if let (Ok(apple_id), Ok(password), Ok(team_id)) = (
            std::env::var("APPLE_ID"),
            std::env::var("APPLE_PASSWORD"),
            std::env::var("APPLE_TEAM_ID"),
        ) {
            return Ok(Self::AppleId {
                apple_id,
                password,
                team_id,
            });
        }

        Err(SetupError::MissingConfig(
            "No notarization credentials found in environment.\n\
             \n\
             Set either:\n\
             • APPLE_API_KEY + APPLE_API_ISSUER + APPLE_API_KEY_PATH (recommended)\n\
             • APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID (legacy)"
                .to_string(),
        ))
    }
}

#[derive(Deserialize)]
struct NotarytoolOutput {
    id: String,
    #[serde(default)]
    status: Option<String>,
    message: String,
}

/// Notarize a macOS app bundle
///
/// # Process
/// 1. Create `PKZip` with `ditto` (Finder-compatible format critical for success)
/// 2. Sign the zip
/// 3. Submit to Apple via `xcrun notarytool submit`
/// 4. Poll for completion (if wait=true)
/// 5. Staple ticket to app (if wait=true and accepted)
///
/// # Arguments
/// * `app_bundle_path` - Path to .app bundle
/// * `auth` - Notarization credentials
/// * `wait` - If true, blocks until notarization completes
///
/// # Returns
/// * `Ok(())` - Success (stapled if wait=true)
/// * `Err(SetupError)` - Notarization failed
///
/// # Example
/// ```no_run
/// let auth = NotarizationAuth::from_env()?;
/// notarize(Path::new("MyApp.app"), &auth, true)?;
/// ```
pub fn notarize(app_bundle_path: &Path, auth: &NotarizationAuth, wait: bool) -> Result<()> {
    use crate::success;

    if !app_bundle_path.exists() {
        return Err(SetupError::InvalidConfig(format!(
            "App bundle not found: {}",
            app_bundle_path.display()
        )));
    }

    println!("🔐 Notarizing {}", app_bundle_path.display());

    // Step 1: Create temporary directory and ZIP
    let temp_dir = TempDir::new()?;
    let zip_name = format!(
        "{}.zip",
        app_bundle_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SetupError::InvalidConfig("Invalid app name".to_string()))?
    );
    let zip_path = temp_dir.path().join(&zip_name);

    println!("  → Creating archive with ditto...");

    // CRITICAL: Use ditto, not zip - creates Finder-compatible PKZip
    // This removes 99% of notarization false alarms
    let ditto_output = Command::new("ditto")
        .args(["-c", "-k", "--keepParent", "--sequesterRsrc"])
        .arg(app_bundle_path)
        .arg(&zip_path)
        .output()
        .map_err(|e| SetupError::CommandExecution(format!("ditto failed: {e}")))?;

    if !ditto_output.status.success() {
        return Err(SetupError::CommandExecution(format!(
            "Failed to create archive: {}",
            String::from_utf8_lossy(&ditto_output.stderr)
        )));
    }

    // Step 2: Sign the ZIP (required by Apple)
    println!("  → Signing archive...");
    let sign_output = Command::new("codesign")
        .args(["-s", "-", "--force"])
        .arg(&zip_path)
        .output()?;

    if !sign_output.status.success() {
        // Non-critical - warn but continue
        eprintln!(
            "⚠️  Archive signing failed (may still work): {}",
            String::from_utf8_lossy(&sign_output.stderr)
        );
    }

    // Step 3: Submit to Apple notarization service
    println!("  → Submitting to Apple...");

    let zip_path_str = zip_path
        .to_str()
        .ok_or_else(|| SetupError::InvalidConfig("Invalid zip path".to_string()))?;

    let mut args = vec![
        "notarytool",
        "submit",
        zip_path_str,
        "--output-format",
        "json",
    ];

    if wait {
        args.push("--wait");
    }

    let mut cmd = Command::new("xcrun");
    cmd.args(&args);

    // Add authentication arguments
    match auth {
        NotarizationAuth::ApiKey {
            key_id,
            issuer_id,
            key_path,
        } => {
            cmd.arg("--key-id")
                .arg(key_id)
                .arg("--key")
                .arg(key_path)
                .arg("--issuer")
                .arg(issuer_id);
        }
        NotarizationAuth::AppleId {
            apple_id,
            password,
            team_id,
        } => {
            cmd.arg("--apple-id")
                .arg(apple_id)
                .arg("--password")
                .arg(password)
                .arg("--team-id")
                .arg(team_id);
        }
    }

    let output = cmd
        .output()
        .map_err(|e| SetupError::CommandExecution(format!("xcrun notarytool failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse common errors and provide helpful context
        let help_text = if stderr.contains("invalidPEMDocument") {
            // Belt-and-suspenders check: re-validate key file
            if let NotarizationAuth::ApiKey { key_path, .. } = auth {
                if !key_path.exists() {
                    format!(
                        "\n\nThe API key file disappeared: {}\n\
                         The file existed during setup but is now missing.\n\
                         Check if it was deleted or moved.",
                        key_path.display()
                    )
                } else {
                    format!(
                        "\n\nThe API key file appears to be invalid: {}\n\n\
                         Troubleshooting:\n\
                         1. Verify file is a valid .p8 key from App Store Connect\n\
                         2. Check file is not corrupted: cat {}\n\
                         3. Re-download key from: https://appstoreconnect.apple.com/access/api\n\
                         4. Ensure file contains:\n\
                            -----BEGIN PRIVATE KEY-----\n\
                            ...\n\
                            -----END PRIVATE KEY-----",
                        key_path.display(),
                        key_path.display()
                    )
                }
            } else {
                String::new()
            }
        } else if stderr.contains("UNAUTHORIZED") || stderr.contains("401") {
            "\n\nAuthentication failed. Check:\n\
             1. APPLE_API_KEY matches your key ID\n\
             2. APPLE_API_ISSUER matches your issuer ID\n\
             3. API key has 'Developer' role in App Store Connect\n\
             4. Key hasn't been revoked"
                .to_string()
        } else if stderr.contains("FORBIDDEN") || stderr.contains("403") {
            "\n\nPermission denied. Verify:\n\
             1. Your App Store Connect account has notarization access\n\
             2. API key has correct permissions (Admin or Developer role)\n\
             3. Team ID is correct"
                .to_string()
        } else {
            String::new()
        };

        return Err(SetupError::CommandExecution(format!(
            "Notarization submission failed:\n{}\n{}",
            stderr, help_text
        )));
    }

    // Step 4: Parse JSON response
    let result: NotarytoolOutput =
        serde_json::from_slice(&output.stdout).map_err(SetupError::Json)?;

    println!("  → Submission ID: {}", result.id);

    if wait {
        match result.status.as_deref() {
            Some("Accepted") => {
                success!("Notarization succeeded!");

                // Step 5: Staple the ticket
                println!("  → Stapling ticket...");
                staple_app(app_bundle_path)?;
                success!("Ticket stapled to app");
            }
            Some(status) => {
                return Err(SetupError::AppStoreConnectApi(format!(
                    "Notarization failed with status: {}\n\
                     Message: {}\n\
                     \n\
                     View detailed log:\n\
                     xcrun notarytool log {} --key-id <KEY_ID> --issuer <ISSUER>",
                    status, result.message, result.id
                )));
            }
            None => {
                return Err(SetupError::AppStoreConnectApi(
                    "Notarization status unknown".to_string(),
                ));
            }
        }
    } else {
        println!("  → Submitted (not waiting for completion)");
        println!(
            "     Status: {}",
            result.status.unwrap_or_else(|| "Pending".to_string())
        );
        println!("     Message: {}", result.message);
        println!("\n  Check status with:");
        println!("     xcrun notarytool log {}", result.id);
    }

    Ok(())
}

/// Staple notarization ticket to app bundle
fn staple_app(app_bundle_path: &Path) -> Result<()> {
    let app_name = app_bundle_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| SetupError::InvalidConfig("Invalid app name".to_string()))?;

    let app_parent = app_bundle_path
        .parent()
        .ok_or_else(|| SetupError::InvalidConfig("App has no parent directory".to_string()))?;

    let output = Command::new("xcrun")
        .args(["stapler", "staple", "-v", app_name])
        .current_dir(app_parent)
        .output()
        .map_err(|e| SetupError::CommandExecution(format!("stapler failed: {e}")))?;

    if !output.status.success() {
        return Err(SetupError::CommandExecution(format!(
            "Stapling failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

/// Search for .p8 API key in standard locations
///
/// Search order:
/// 1. ./`private_keys/AuthKey`_{`KEY_ID}.p8`
/// 2. ~/`private_keys/AuthKey`_{`KEY_ID}.p8`
/// 3. ~/.`private_keys/AuthKey`_{`KEY_ID}.p8`
/// 4. ~/.`appstoreconnect/private_keys/AuthKey`_{`KEY_ID}.p8`
fn find_p8_key(key_id: &str) -> Option<std::path::PathBuf> {
    let filename = format!("AuthKey_{key_id}.p8");

    let mut search_paths = vec![std::path::PathBuf::from("./private_keys")];

    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join("private_keys"));
        search_paths.push(home.join(".private_keys"));
        search_paths.push(home.join(".appstoreconnect/private_keys"));
    }

    for dir in search_paths {
        let key_path = dir.join(&filename);
        if key_path.exists() && key_path.is_file() {
            println!("✓ Found API key: {}", key_path.display());
            return Some(key_path);
        }
    }

    None
}

/// Diagnose notarization setup
///
/// Validates credentials and dependencies without attempting notarization.
/// Useful for troubleshooting setup issues.
pub fn diagnose_notarization_setup() -> Result<()> {
    println!("🔍 Diagnosing notarization setup...\n");

    // Check for credentials
    match NotarizationAuth::from_env() {
        Ok(NotarizationAuth::ApiKey {
            key_id,
            issuer_id,
            key_path,
        }) => {
            println!("✓ API Key authentication configured");
            println!("  Key ID: {}", key_id);
            println!("  Issuer: {}", issuer_id);
            println!("  Key path: {}", key_path.display());

            // Re-validate to show current status
            match validate_p8_file(&key_path) {
                Ok(()) => println!("  ✓ Key file is valid"),
                Err(e) => {
                    println!("  ✗ Key file validation failed:\n{}", e);
                    return Err(e);
                }
            }
        }
        Ok(NotarizationAuth::AppleId {
            apple_id, team_id, ..
        }) => {
            println!("✓ Apple ID authentication configured");
            println!("  Apple ID: {}", apple_id);
            println!("  Team ID: {}", team_id);
            println!("  ⚠️  Consider switching to API Key (more reliable)");
        }
        Err(e) => {
            println!("✗ No notarization credentials found");
            return Err(e);
        }
    }

    // Check xcrun notarytool
    let xcrun_check = Command::new("xcrun")
        .args(["notarytool", "--version"])
        .output();

    match xcrun_check {
        Ok(output) if output.status.success() => {
            println!("\n✓ xcrun notarytool is available");
        }
        _ => {
            println!("\n✗ xcrun notarytool not found");
            println!("  Install Xcode Command Line Tools:");
            println!("  xcode-select --install");
            return Err(SetupError::MissingDependency(
                "xcrun notarytool not available".to_string(),
            ));
        }
    }

    println!("\n✅ Notarization setup looks good!");
    Ok(())
}
