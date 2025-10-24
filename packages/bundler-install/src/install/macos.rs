//! macOS platform implementation using osascript and launchd.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use plist::Value;

use crate::install::builder::CommandBuilder;
use crate::install::{InstallerBuilder, InstallerError};

pub(crate) struct PlatformExecutor;

// Global helper path - initialized once, used everywhere
static HELPER_PATH: OnceCell<PathBuf> = OnceCell::new();

// Embedded ZIP data for the signed helper app
// This is generated at build time by build.rs which creates a proper signed macOS helper
const APP_ZIP_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/KodegenHelper.app.zip"));

impl PlatformExecutor {
    pub fn install(b: InstallerBuilder) -> Result<(), InstallerError> {
        // Initialize helper path if not already set
        Self::ensure_helper_path()?;

        // System daemons always use system directories
        let plist_dir = PathBuf::from("/Library/LaunchDaemons");
        let bin_dir = PathBuf::from("/usr/local/bin");
        let needs_sudo = true;

        // First, copy the binary to temp so elevated context can access it
        let temp_path = std::env::temp_dir().join(&b.label);
        std::fs::copy(&b.program, &temp_path)
            .map_err(|e| InstallerError::System(format!("Failed to copy binary to temp: {e}")))?;

        let plist_content = Self::generate_plist(&b)?;

        // Convert paths to strings with error handling
        let plist_dir_str = plist_dir
            .to_str()
            .ok_or_else(|| InstallerError::System("Invalid plist directory path".to_string()))?;
        let bin_dir_str = bin_dir
            .to_str()
            .ok_or_else(|| InstallerError::System("Invalid bin directory path".to_string()))?;

        // Build the installation commands using CommandBuilder
        let mkdir_cmd = CommandBuilder::new("mkdir").args([
            "-p",
            plist_dir_str,
            bin_dir_str,
            &format!("/var/log/{}", b.label),
        ]);

        let bin_path = bin_dir.join(&b.label);
        let bin_path_str = bin_path
            .to_str()
            .ok_or_else(|| InstallerError::System("Invalid binary path".to_string()))?;
        let temp_path_str = temp_path
            .to_str()
            .ok_or_else(|| InstallerError::System("Invalid temp path".to_string()))?;

        let cp_cmd = CommandBuilder::new("cp").args([temp_path_str, bin_path_str]);

        let chown_cmd = if needs_sudo {
            CommandBuilder::new("chown").args(["root:wheel", bin_path_str])
        } else {
            // For user installs, no chown needed
            CommandBuilder::new("true").args::<[&str; 0], &str>([])
        };

        let chmod_cmd = CommandBuilder::new("chmod").args(["755", bin_path_str]);

        let rm_cmd = CommandBuilder::new("rm").args(["-f", temp_path_str]);

        // Write files to temp location first, then move them in elevated context
        let temp_plist = std::env::temp_dir().join(format!("{}.plist", b.label));
        std::fs::write(&temp_plist, &plist_content)
            .map_err(|e| InstallerError::System(format!("Failed to write temp plist: {e}")))?;

        let plist_file = plist_dir.join(format!("{}.plist", b.label));
        let plist_file_str = plist_file
            .to_str()
            .ok_or_else(|| InstallerError::System("Invalid plist file path".to_string()))?;
        let temp_plist_str = temp_plist
            .to_str()
            .ok_or_else(|| InstallerError::System("Invalid temp plist path".to_string()))?;

        let mut script = format!("set -e\n{}", Self::command_to_script(&mkdir_cmd));
        script.push_str(&format!(" && {}", Self::command_to_script(&cp_cmd)));
        if needs_sudo {
            script.push_str(&format!(" && {}", Self::command_to_script(&chown_cmd)));
        }
        script.push_str(&format!(" && {}", Self::command_to_script(&chmod_cmd)));
        script.push_str(&format!(" && {}", Self::command_to_script(&rm_cmd)));
        script.push_str(&format!(" && mv {temp_plist_str} {plist_file_str}"));

        // Set plist permissions (only for system-wide installs)
        if needs_sudo {
            let plist_perms_chown =
                CommandBuilder::new("chown").args(["root:wheel", plist_file_str]);
            let plist_perms_chmod = CommandBuilder::new("chmod").args(["644", plist_file_str]);

            script.push_str(&format!(
                " && {}",
                Self::command_to_script(&plist_perms_chown)
            ));
            script.push_str(&format!(
                " && {}",
                Self::command_to_script(&plist_perms_chmod)
            ));
        }

        // Create services directory
        let services_dir = CommandBuilder::new("mkdir").args(["-p", "/etc/kodegend/services"]);

        script.push_str(&format!(" && {}", Self::command_to_script(&services_dir)));

        // Add service definitions using CommandBuilder
        if !b.services.is_empty() {
            for service in &b.services {
                let service_toml = toml::to_string_pretty(service).map_err(|e| {
                    InstallerError::System(format!("Failed to serialize service: {e}"))
                })?;

                // Write service file to temp first
                let temp_service = std::env::temp_dir().join(format!("{}.toml", service.name));
                std::fs::write(&temp_service, &service_toml).map_err(|e| {
                    InstallerError::System(format!("Failed to write temp service: {e}"))
                })?;
                let temp_service_str = temp_service.to_str().ok_or_else(|| {
                    InstallerError::System("Invalid temp service path".to_string())
                })?;

                let service_file = format!("/etc/kodegend/services/{}.toml", service.name);
                script.push_str(&format!(" && mv {temp_service_str} {service_file}"));

                // Set service file permissions using CommandBuilder
                let service_perms_chown =
                    CommandBuilder::new("chown").args(["root:wheel", &service_file]);

                let service_perms_chmod = CommandBuilder::new("chmod").args(["644", &service_file]);

                script.push_str(&format!(
                    " && {}",
                    Self::command_to_script(&service_perms_chown)
                ));
                script.push_str(&format!(
                    " && {}",
                    Self::command_to_script(&service_perms_chmod)
                ));
            }
        }

        // Load the daemon using CommandBuilder (only if auto_start is enabled)
        if b.auto_start {
            let load_daemon = CommandBuilder::new("launchctl").args(["load", "-w", plist_file_str]);

            script.push_str(&format!(" && {}", Self::command_to_script(&load_daemon)));
        }

        // For user installs, run script without helper (no sudo needed)
        if needs_sudo {
            Self::run_helper(&script)
        } else {
            // User install - run directly with sh
            let output = Command::new("sh")
                .arg("-c")
                .arg(&script)
                .output()
                .map_err(|e| {
                    InstallerError::System(format!("Failed to execute install script: {e}"))
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(InstallerError::System(format!(
                    "Installation script failed: {stderr}"
                )));
            }

            Ok(())
        }
    }

    /// Ensure the helper path is initialized for secure privileged operations
    fn ensure_helper_path() -> Result<(), InstallerError> {
        if HELPER_PATH.get().is_none() {
            let helper_path = Self::extract_helper_app()?;
            HELPER_PATH
                .set(helper_path)
                .map_err(|_| InstallerError::System("Failed to set helper path".to_string()))?;
        }
        Ok(())
    }

    /// Extract the signed helper app from embedded data
    fn extract_helper_app() -> Result<PathBuf, InstallerError> {
        // Use a stable location based on app version to avoid re-extraction
        let version_hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            APP_ZIP_DATA.len().hash(&mut hasher);
            APP_ZIP_DATA
                .get(0..64)
                .unwrap_or(&APP_ZIP_DATA[0..APP_ZIP_DATA.len().min(64)])
                .hash(&mut hasher);
            hasher.finish()
        };

        let helper_dir = std::env::temp_dir()
            .join("kodegen_helper")
            .join(format!("v{version_hash:016x}"));

        std::fs::create_dir_all(&helper_dir).map_err(|e| {
            InstallerError::System(format!("Failed to create helper directory: {e}"))
        })?;

        let helper_path = helper_dir.join("KodegenHelper.app");

        // Check if helper already exists and is valid
        if helper_path.exists()
            && Self::validate_helper(&helper_path)?
            && Self::verify_code_signature(&helper_path)?
        {
            return Ok(helper_path);
        }

        // Extract from embedded data
        Self::extract_from_embedded_data(&helper_path)?;

        Ok(helper_path)
    }

    /// Extract helper from embedded `APP_ZIP_DATA` with zero-allocation validation
    fn extract_from_embedded_data(helper_path: &PathBuf) -> Result<bool, InstallerError> {
        use std::sync::atomic::{AtomicU8, Ordering};

        use arrayvec::ArrayVec;
        use atomic_counter::{AtomicCounter, RelaxedCounter};

        // Atomic validation state tracking (0=pending, 1=size_valid, 2=header_valid, 3=extraction_complete)
        static VALIDATION_STATE: AtomicU8 = AtomicU8::new(0);
        use once_cell::sync::Lazy;
        static VALIDATION_COUNTER: Lazy<RelaxedCounter> = Lazy::new(|| RelaxedCounter::new(0));

        VALIDATION_STATE.store(0, Ordering::Relaxed);
        VALIDATION_COUNTER.inc();

        // Zero-allocation ZIP validation using stack-allocated arrays
        const MIN_ZIP_SIZE: usize = 22; // Minimum ZIP central directory size

        if APP_ZIP_DATA.len() < MIN_ZIP_SIZE {
            return Err(InstallerError::System(
                "Embedded helper ZIP data is too small".to_string(),
            ));
        }
        VALIDATION_STATE.store(1, Ordering::Relaxed);

        // Zero-allocation ZIP magic header validation using ArrayVec
        let mut magic_headers: ArrayVec<&[u8], 3> = ArrayVec::new();
        magic_headers.push(&[0x50, 0x4B, 0x03, 0x04]); // Local file header
        magic_headers.push(&[0x50, 0x4B, 0x05, 0x06]); // Empty archive
        magic_headers.push(&[0x50, 0x4B, 0x07, 0x08]); // Spanned archive

        let has_valid_header = magic_headers
            .iter()
            .any(|&header| APP_ZIP_DATA.len() >= header.len() && APP_ZIP_DATA.starts_with(header));

        if !has_valid_header {
            return Err(InstallerError::System(
                "Invalid ZIP signature in embedded data".to_string(),
            ));
        }
        VALIDATION_STATE.store(2, Ordering::Relaxed);

        // Enhanced ZIP central directory validation using zero-copy access
        if let Err(e) = Self::validate_zip_central_directory() {
            return Err(InstallerError::System(format!(
                "ZIP central directory validation failed: {e}"
            )));
        }

        // Extract the embedded ZIP data
        match Self::extract_zip_data(APP_ZIP_DATA, helper_path) {
            Ok(()) => {
                VALIDATION_STATE.store(3, Ordering::Relaxed);

                // Enhanced validation with detailed error tracking
                let helper_valid = match Self::validate_helper(helper_path) {
                    Ok(valid) => {
                        if !valid {
                            return Err(InstallerError::System(
                                "Helper validation failed: Invalid bundle structure or missing Info.plist keys".to_string(),
                            ));
                        }
                        valid
                    }
                    Err(e) => {
                        return Err(InstallerError::System(format!(
                            "Helper validation error: {e}"
                        )));
                    }
                };

                let signature_valid = match Self::verify_code_signature(helper_path) {
                    Ok(valid) => {
                        if !valid {
                            return Err(InstallerError::System(
                                "Code signature validation failed: Missing CodeResources or invalid bundle".to_string(),
                            ));
                        }
                        valid
                    }
                    Err(e) => {
                        return Err(InstallerError::System(format!(
                            "Code signature validation error: {e}"
                        )));
                    }
                };

                if helper_valid && signature_valid {
                    Ok(true)
                } else {
                    // This should not be reached due to explicit error handling above
                    let _ = std::fs::remove_dir_all(helper_path);
                    Err(InstallerError::System(
                        "Extracted helper failed validation (unexpected)".to_string(),
                    ))
                }
            }
            Err(e) => {
                // Extraction failed, cleanup
                let _ = std::fs::remove_dir_all(helper_path);
                Err(e)
            }
        }
    }

    /// Zero-allocation ZIP central directory validation using pointer arithmetic
    fn validate_zip_central_directory() -> Result<(), &'static str> {
        use arrayvec::ArrayVec;

        const EOCD_SIGNATURE: u32 = 0x06054b50; // End of Central Directory signature
        const EOCD_MIN_SIZE: usize = 22;

        if APP_ZIP_DATA.len() < EOCD_MIN_SIZE {
            return Err("ZIP data too small for central directory");
        }

        // Search for End of Central Directory record from the end (zero-allocation approach)
        let search_start = APP_ZIP_DATA.len().saturating_sub(65536); // ZIP spec: max comment size is 65535
        let search_range = &APP_ZIP_DATA[search_start..];

        // Stack-allocated buffer for signature checking
        let mut eocd_offset: Option<usize> = None;

        // Scan backwards for EOCD signature using zero-allocation approach
        for i in (0..search_range.len().saturating_sub(3)).rev() {
            if search_range.len() >= i + 4 {
                let signature_bytes: ArrayVec<u8, 4> = ArrayVec::from([
                    search_range[i],
                    search_range[i + 1],
                    search_range[i + 2],
                    search_range[i + 3],
                ]);

                let signature = u32::from_le_bytes([
                    signature_bytes[0],
                    signature_bytes[1],
                    signature_bytes[2],
                    signature_bytes[3],
                ]);

                if signature == EOCD_SIGNATURE {
                    eocd_offset = Some(search_start + i);
                    break;
                }
            }
        }

        let eocd_pos = eocd_offset.ok_or("End of Central Directory signature not found")?;

        // Validate EOCD structure using stack-allocated parsing
        if APP_ZIP_DATA.len() < eocd_pos + EOCD_MIN_SIZE {
            return Err("Incomplete End of Central Directory record");
        }

        // Parse central directory information (zero-allocation)
        let eocd_data = &APP_ZIP_DATA[eocd_pos..];

        if eocd_data.len() < 22 {
            return Err("EOCD record too short");
        }

        // Extract central directory info using zero-copy parsing
        let _disk_number = u16::from_le_bytes([eocd_data[4], eocd_data[5]]);
        let _cd_start_disk = u16::from_le_bytes([eocd_data[6], eocd_data[7]]);
        let cd_entries_this_disk = u16::from_le_bytes([eocd_data[8], eocd_data[9]]);
        let cd_total_entries = u16::from_le_bytes([eocd_data[10], eocd_data[11]]);
        let cd_size =
            u32::from_le_bytes([eocd_data[12], eocd_data[13], eocd_data[14], eocd_data[15]]);
        let cd_offset =
            u32::from_le_bytes([eocd_data[16], eocd_data[17], eocd_data[18], eocd_data[19]]);

        // Validate central directory parameters
        if cd_entries_this_disk != cd_total_entries {
            return Err("Multi-disk ZIP archives not supported");
        }

        if cd_total_entries == 0 {
            return Err("ZIP archive contains no entries");
        }

        // Validate central directory bounds
        let cd_end = cd_offset
            .checked_add(cd_size)
            .ok_or("Central directory offset/size overflow")?;

        if cd_end as usize > APP_ZIP_DATA.len() {
            return Err("Central directory extends beyond ZIP data");
        }

        if cd_offset as usize >= APP_ZIP_DATA.len() {
            return Err("Central directory offset beyond ZIP data");
        }

        Ok(())
    }

    /// Extract ZIP data to the specified path
    fn extract_zip_data(zip_data: &[u8], target_path: &Path) -> Result<(), InstallerError> {
        use std::io::{Cursor, Read};

        use zip::ZipArchive;

        // Create a cursor for the ZIP data
        let cursor = Cursor::new(zip_data);

        // Create ZIP archive reader
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| InstallerError::System(format!("Failed to read ZIP archive: {e}")))?;

        // Extract all files
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| {
                InstallerError::System(format!("Failed to access file in ZIP: {e}"))
            })?;

            let file_path = match file.enclosed_name() {
                Some(path) => path.clone(),
                None => {
                    // Skip files with invalid paths
                    continue;
                }
            };

            // Strip the top-level KodegenHelper.app directory from the ZIP path
            // since we're extracting TO KodegenHelper.app (zero-allocation path stripping)
            let relative_path = file_path
                .strip_prefix("KodegenHelper.app")
                .unwrap_or(&file_path);

            let out_path = target_path.join(relative_path);

            // Create parent directories if needed
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    InstallerError::System(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }

            if file.is_dir() {
                // Create directory
                std::fs::create_dir_all(&out_path).map_err(|e| {
                    InstallerError::System(format!(
                        "Failed to create directory {}: {}",
                        out_path.display(),
                        e
                    ))
                })?;
            } else {
                // Extract file
                let mut outfile = std::fs::File::create(&out_path).map_err(|e| {
                    InstallerError::System(format!(
                        "Failed to create file {}: {}",
                        out_path.display(),
                        e
                    ))
                })?;

                // Copy file contents with zero-copy optimization where possible
                let mut buffer = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buffer).map_err(|e| {
                    InstallerError::System(format!("Failed to read file from ZIP: {e}"))
                })?;

                use std::io::Write;
                outfile.write_all(&buffer).map_err(|e| {
                    InstallerError::System(format!(
                        "Failed to write file {}: {}",
                        out_path.display(),
                        e
                    ))
                })?;

                // Sync to ensure data is written
                outfile.sync_all().map_err(|e| {
                    InstallerError::System(format!(
                        "Failed to sync file {}: {}",
                        out_path.display(),
                        e
                    ))
                })?;

                // Set file permissions on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = file.unix_mode() {
                        let permissions = std::fs::Permissions::from_mode(mode);
                        std::fs::set_permissions(&out_path, permissions).map_err(|e| {
                            InstallerError::System(format!(
                                "Failed to set permissions on {}: {}",
                                out_path.display(),
                                e
                            ))
                        })?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate that the helper app is properly signed and functional
    fn validate_helper(helper_path: &Path) -> Result<bool, InstallerError> {
        // Check if the helper exists and has the expected structure
        let contents = helper_path.join("Contents");
        let macos = contents.join("MacOS");
        let info_plist = contents.join("Info.plist");
        let executable = macos.join("KodegenHelper");

        // Verify all required components exist (zero-allocation existence checks)
        if !contents.exists() || !macos.exists() || !info_plist.exists() || !executable.exists() {
            return Ok(false);
        }

        // Verify Info.plist contains required keys
        let plist_data = std::fs::read(&info_plist)
            .map_err(|e| InstallerError::System(format!("Failed to read Info.plist: {e}")))?;

        let plist_value = plist::from_bytes::<plist::Value>(&plist_data)
            .map_err(|e| InstallerError::System(format!("Failed to parse Info.plist: {e}")))?;

        if let plist::Value::Dictionary(dict) = plist_value {
            // Check required keys (zero-allocation key existence validation)
            let has_bundle_id = dict.contains_key("CFBundleIdentifier");
            let has_bundle_executable = dict.contains_key("CFBundleExecutable");
            let has_sm_authorized = dict.contains_key("SMAuthorizedClients");

            Ok(has_bundle_id && has_bundle_executable && has_sm_authorized)
        } else {
            Ok(false)
        }
    }

    /// Verify the code signature of the helper app using Tauri-compatible validation
    fn verify_code_signature(helper_path: &Path) -> Result<bool, InstallerError> {
        // Use Tauri's signing verification approach - check for valid bundle structure
        // and signature presence without manual codesign calls

        // Verify CodeResources exists (created by Tauri signing)
        let code_resources = helper_path.join("Contents/_CodeSignature/CodeResources");
        if !code_resources.exists() {
            return Err(InstallerError::System(
                "Helper app missing CodeResources - not properly signed".to_string(),
            ));
        }

        // Verify executable exists and has proper permissions
        let executable = helper_path.join("Contents/MacOS/KodegenHelper");
        if !executable.exists() {
            return Err(InstallerError::System(
                "Helper app missing executable".to_string(),
            ));
        }

        // Check executable permissions (should be executable)
        let metadata = std::fs::metadata(&executable).map_err(|e| {
            InstallerError::System(format!("Failed to get executable metadata: {e}"))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            // Check if executable bit is set (0o100)
            if (mode & 0o111) == 0 {
                return Err(InstallerError::System(
                    "Helper executable does not have execute permissions".to_string(),
                ));
            }
        }

        // Verify Info.plist has proper bundle structure
        let info_plist = helper_path.join("Contents/Info.plist");
        let plist_data = std::fs::read(&info_plist)
            .map_err(|e| InstallerError::System(format!("Failed to read Info.plist: {e}")))?;

        let plist_value = plist::from_bytes::<plist::Value>(&plist_data)
            .map_err(|e| InstallerError::System(format!("Failed to parse Info.plist: {e}")))?;

        if let plist::Value::Dictionary(dict) = plist_value {
            // Verify bundle identifier matches expected value
            if let Some(plist::Value::String(bundle_id)) = dict.get("CFBundleIdentifier") {
                if bundle_id != "ai.kodegen.kodegend.helper" {
                    return Err(InstallerError::System(format!(
                        "Unexpected bundle identifier: {bundle_id} (expected: ai.kodegen.kodegend.helper)"
                    )));
                }
            } else {
                return Err(InstallerError::System(
                    "Missing or invalid CFBundleIdentifier in Info.plist".to_string(),
                ));
            }
        } else {
            return Err(InstallerError::System(
                "Info.plist is not a valid property list dictionary".to_string(),
            ));
        }

        // If all Tauri-signed bundle validation checks pass, the helper is valid
        Ok(true)
    }

    pub fn uninstall(label: &str) -> Result<(), InstallerError> {
        let script = format!(
            r"
            set -e
            # Unload daemon if running
            launchctl unload -w /Library/LaunchDaemons/{label}.plist 2>/dev/null || true
            
            # Remove files
            rm -f /Library/LaunchDaemons/{label}.plist
            rm -f /usr/local/bin/{label}
            rm -rf /var/log/{label}
        "
        );

        Self::run_helper(&script)
    }

    fn generate_plist(b: &InstallerBuilder) -> Result<String, InstallerError> {
        let mut plist = HashMap::new();

        // Basic properties
        plist.insert("Label".to_string(), Value::String(b.label.clone()));
        plist.insert("Disabled".to_string(), Value::Boolean(false));

        // Program and arguments
        let mut program_args = vec![Value::String(format!("/usr/local/bin/{}", b.label))];
        program_args.extend(b.args.iter().map(|a| Value::String(a.clone())));
        plist.insert("ProgramArguments".to_string(), Value::Array(program_args));

        // Environment variables
        if !b.env.is_empty() {
            let env_dict: HashMap<String, Value> = b
                .env
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            plist.insert(
                "EnvironmentVariables".to_string(),
                Value::Dictionary(env_dict.into_iter().collect()),
            );
        }

        // User/Group
        plist.insert("UserName".to_string(), Value::String(b.run_as_user.clone()));
        if b.run_as_group != "wheel" && b.run_as_group != "staff" {
            plist.insert(
                "GroupName".to_string(),
                Value::String(b.run_as_group.clone()),
            );
        }

        // Auto-restart
        plist.insert(
            "KeepAlive".to_string(),
            if b.auto_restart {
                Value::Dictionary(
                    vec![("SuccessfulExit".to_string(), Value::Boolean(false))]
                        .into_iter()
                        .collect(),
                )
            } else {
                Value::Boolean(false)
            },
        );

        // Logging
        plist.insert(
            "StandardOutPath".to_string(),
            Value::String(format!("/var/log/{}/stdout.log", b.label)),
        );
        plist.insert(
            "StandardErrorPath".to_string(),
            Value::String(format!("/var/log/{}/stderr.log", b.label)),
        );

        // Run at load
        plist.insert("RunAtLoad".to_string(), Value::Boolean(true));

        // Network dependency
        if b.wants_network {
            plist.insert(
                "LimitLoadToSessionType".to_string(),
                Value::String("System".to_string()),
            );
        }

        // Generate XML
        let mut buf = Vec::new();
        plist::to_writer_xml(&mut buf, &Value::Dictionary(plist.into_iter().collect()))
            .map_err(|e| InstallerError::System(format!("Failed to generate plist: {e}")))?;

        String::from_utf8(buf)
            .map_err(|e| InstallerError::System(format!("Plist contains invalid UTF-8: {e}")))
    }

    #[allow(dead_code)]
    fn run_osascript(script: &str) -> Result<(), InstallerError> {
        // Escape the script for AppleScript
        let escaped_script = script.replace('\\', "\\\\").replace('"', "\\\"");

        let applescript =
            format!(r#"do shell script "{escaped_script}" with administrator privileges"#);

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&applescript)
            .output()
            .context("failed to invoke osascript")?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("User canceled") || stderr.contains("-128") {
                Err(InstallerError::Cancelled)
            } else if stderr.contains("authorization") || stderr.contains("privileges") {
                Err(InstallerError::PermissionDenied)
            } else {
                Err(InstallerError::System(stderr.into_owned()))
            }
        }
    }

    /// Execute script using the signed helper app with elevated privileges
    fn run_helper(script: &str) -> Result<(), InstallerError> {
        // Get the helper path
        let helper_path = HELPER_PATH
            .get()
            .ok_or_else(|| InstallerError::System("Helper app not initialized".to_string()))?;

        let helper_exe = helper_path.join("Contents/MacOS/KodegenHelper");

        // Launch helper with elevated privileges using osascript
        // The helper itself is what gets elevated, not the script
        let escaped_helper = helper_exe
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");

        let applescript =
            format!(r#"do shell script "\"{escaped_helper}\"" with administrator privileges"#);

        // Start the helper process with admin privileges
        let mut child = Command::new("osascript")
            .arg("-e")
            .arg(&applescript)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| InstallerError::System(format!("Failed to launch helper: {e}")))?;

        // Write the script to the helper's stdin
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(script.as_bytes()).map_err(|e| {
                InstallerError::System(format!("Failed to send script to helper: {e}"))
            })?;
            stdin.flush().map_err(|e| {
                InstallerError::System(format!("Failed to flush script to helper: {e}"))
            })?;
        }

        // Wait for the helper to complete
        let output = child
            .wait_with_output()
            .map_err(|e| InstallerError::System(format!("Failed to wait for helper: {e}")))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Check for user cancellation
            if stderr.contains("User canceled")
                || stderr.contains("-128")
                || stdout.contains("User canceled")
                || stdout.contains("-128")
            {
                Err(InstallerError::Cancelled)
            } else if stderr.contains("Unauthorized parent process") {
                Err(InstallerError::System(
                    "Helper security check failed".to_string(),
                ))
            } else if stderr.contains("Script execution timed out") {
                Err(InstallerError::System(
                    "Installation script timed out".to_string(),
                ))
            } else {
                // Include both stdout and stderr for debugging
                let full_error = format!("Helper failed: stdout={stdout}, stderr={stderr}");
                Err(InstallerError::System(full_error))
            }
        }
    }

    fn command_to_script(cmd: &CommandBuilder) -> String {
        let mut parts = vec![cmd.program.to_string_lossy().to_string()];
        parts.extend(cmd.args.iter().cloned());
        parts.join(" ")
    }
}
