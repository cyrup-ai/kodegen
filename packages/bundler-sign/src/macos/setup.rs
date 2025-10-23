//! Main setup workflows for macOS certificate provisioning

use crate::error::{Result, SetupError};
use crate::config::{MacOSSetupConfig, DEFAULT_KEYCHAIN, DEFAULT_COMMON_NAME, CertificateType};
use crate::apple_api;
use crate::{success, error as error_msg};
use std::process::Command;
use std::io::{self, Write};
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};
use tempfile::NamedTempFile;

use super::validation::{check_dependencies, ensure_keychain_accessible, ensure_config_directory_writable, expand_tilde_path, validate_p8_file};
use super::prompts::{prompt_for_p8_path, prompt_yes_no};
use super::keychain::{check_for_developer_certificates, import_certificate_to_keychain, TempKeychain};

// ============================================================================
// CI/CD AUTHENTICATION
// ============================================================================

/// Authentication from environment variables (CI/CD)
enum EnvAuth {
    Certificate {
        cert_base64: String,
        password: String,
    },
    ApiKey {
        key_id: String,
        issuer_id: String,
        key_path: std::path::PathBuf,
    },
}

/// Try to load authentication from environment variables
///
/// Priority order:
/// 1. `APPLE_CERTIFICATE` + `APPLE_CERTIFICATE_PASSWORD` (base64 .p12)
/// 2. `APPLE_API_KEY` + `APPLE_API_ISSUER` + `APPLE_API_KEY_PATH`
fn try_auth_from_env() -> Option<EnvAuth> {
    // Priority 1: Certificate (most common in CI/CD)
    if let (Ok(cert_b64), Ok(password)) = (
        std::env::var("APPLE_CERTIFICATE"),
        std::env::var("APPLE_CERTIFICATE_PASSWORD")
    ) {
        return Some(EnvAuth::Certificate {
            cert_base64: cert_b64,
            password,
        });
    }
    
    // Priority 2: API key
    if let (Ok(key_id), Ok(issuer)) = (
        std::env::var("APPLE_API_KEY"),
        std::env::var("APPLE_API_ISSUER")
    ) {
        // Explicit path
        if let Ok(key_path) = std::env::var("APPLE_API_KEY_PATH") {
            return Some(EnvAuth::ApiKey {
                key_id,
                issuer_id: issuer,
                key_path: key_path.into(),
            });
        }
        
        // Auto-search standard locations
        if let Some(key_path) = find_p8_key_in_standard_locations(&key_id) {
            return Some(EnvAuth::ApiKey {
                key_id,
                issuer_id: issuer,
                key_path,
            });
        }
    }
    
    None
}

/// Setup using certificate from environment (CI/CD)
fn setup_with_certificate(
    cert_base64: &str,
    password: &str,
    dry_run: bool,
) -> Result<TempKeychain> {
    if dry_run {
        success!("DRY RUN: Would import certificate from APPLE_CERTIFICATE env var");
        return Err(SetupError::InvalidConfig(
            "Cannot create temp keychain in dry-run mode".to_string()
        ));
    }
    
    println!("📦 Importing certificate from environment variable...");
    
    // Decode base64 certificate
    use base64::Engine;
    let cert_bytes = base64::engine::general_purpose::STANDARD.decode(cert_base64)
        .map_err(|e| SetupError::InvalidConfig(format!(
            "Invalid APPLE_CERTIFICATE (not valid base64): {e}"
        )))?;
    
    // Create temporary keychain
    let temp_keychain = TempKeychain::from_certificate_bytes(&cert_bytes, password)?;
    
    success!("Certificate imported to temporary keychain");
    println!("  Identity: {}", temp_keychain.signing_identity());
    println!("  Path: {}", temp_keychain.path().display());
    
    Ok(temp_keychain)
}

fn find_p8_key_in_standard_locations(key_id: &str) -> Option<std::path::PathBuf> {
    let filename = format!("AuthKey_{key_id}.p8");
    
    let mut search_paths = vec![
        std::path::PathBuf::from("./private_keys"),
    ];
    
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

/// Display current signing configuration and certificate status
pub fn show_config() -> Result<()> {
    // Check for Developer ID certificates using robust parsing
    let output = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map_err(|e| SetupError::CommandExecution(format!("Failed to run security command: {e}")))?;
    
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    
    // Parse certificates with proper error handling
    match check_for_developer_certificates(output) {
        Ok(certs) if !certs.is_empty() => {
            // Colored status output - errors ignored (see module-level docs)
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = writeln!(&mut buffer, "✅ Developer ID Certificate: Found {} certificate(s)", certs.len());
            let _ = buffer.reset();
            for cert in certs {
                let _ = writeln!(&mut buffer, "   {cert}");
            }
        },
        Ok(_) => {
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
            let _ = writeln!(&mut buffer, "❌ Developer ID Certificate: Not found");
            let _ = buffer.reset();
        },
        Err(e) => {
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
            let _ = writeln!(&mut buffer, "⚠️  Developer ID Certificate: Could not check");
            let _ = buffer.reset();
            let _ = writeln!(&mut buffer, "   Error: {e}");
        }
    }
    
    // Check for stored credentials
    if let Some(home) = dirs::home_dir() {
        let cred_path = home.join(".config/kodegen/signing.toml");
        if cred_path.exists() {
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = writeln!(&mut buffer, "\n✅ Signing Config: {}", cred_path.display());
            let _ = buffer.reset();
        } else {
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
            let _ = writeln!(&mut buffer, "\n❌ Signing Config: Not found");
            let _ = buffer.reset();
        }
    }
    
    let _ = bufwtr.print(&buffer);
    Ok(())
}

/// Interactive certificate provisioning workflow
pub fn interactive_setup() -> Result<()> {
    // Check required dependencies before proceeding
    check_dependencies()?;
    
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(&mut buffer, "\n{}", "━".repeat(60));
    let _ = writeln!(&mut buffer, "Step 1: Checking for existing Developer ID certificate...\n");
    let _ = bufwtr.print(&buffer);
    
    // Check for existing certificate with robust parsing
    let output = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map_err(|e| SetupError::CommandExecution(format!("Failed to check certificates: {e}")))?;
    
    // Use new parsing function
    match check_for_developer_certificates(output) {
        Ok(certs) if !certs.is_empty() => {
            let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            // Colored success output - errors ignored (see module-level docs)
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            let _ = writeln!(&mut buffer, "✅ Found existing Developer ID certificate!\n");
            let _ = buffer.reset();
            for cert in &certs {
                let _ = writeln!(&mut buffer, "   {cert}");
            }
            let _ = writeln!(&mut buffer);
            let _ = bufwtr.print(&buffer);
            
            // ASK USER if they want to provision a new certificate
            if !prompt_yes_no("Do you want to provision a new certificate?")? {
                let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
                let mut buffer = bufwtr.buffer();
                let _ = writeln!(&mut buffer, "\nSetup complete! Using existing certificate.");
                let _ = bufwtr.print(&buffer);
                return Ok(());
            }
            // If user says YES, fall through to continue provisioning
        },
        Ok(_) => {
            let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
            let _ = writeln!(&mut buffer, "⚠️  No Developer ID certificate found");
            let _ = buffer.reset();
            let _ = writeln!(&mut buffer, "   Proceeding with certificate provisioning...\n");
            let _ = bufwtr.print(&buffer);
        },
        Err(e) => {
            // Non-fatal: warn but proceed with provisioning
            let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
            let _ = writeln!(&mut buffer, "⚠️  Warning: Could not check existing certificates");
            let _ = buffer.reset();
            let _ = writeln!(&mut buffer, "   Error: {e}");
            let _ = writeln!(&mut buffer, "   Proceeding with certificate provisioning...\n");
            let _ = bufwtr.print(&buffer);
        }
    }
    
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(&mut buffer, "\n{}", "━".repeat(60));
    let _ = writeln!(&mut buffer, "Step 2: App Store Connect API Setup\n");
    let _ = writeln!(&mut buffer, "To provision a certificate, you need API credentials from:");
    let _ = writeln!(&mut buffer, "  https://appstoreconnect.apple.com/access/api\n");
    let _ = writeln!(&mut buffer, "Instructions:");
    let _ = writeln!(&mut buffer, "  1. Click the '+' button to create a new key");
    let _ = writeln!(&mut buffer, "  2. Name it 'Kodegen Signing' (or similar)");
    let _ = writeln!(&mut buffer, "  3. Select 'Developer' role");
    let _ = writeln!(&mut buffer, "  4. Click 'Generate'");
    let _ = writeln!(&mut buffer, "  5. Download the .p8 file (can only download once!)");
    let _ = writeln!(&mut buffer, "  6. Note the Key ID and Issuer ID from the page\n");
    let _ = writeln!(&mut buffer, "{}", "━".repeat(60));
    let _ = bufwtr.print(&buffer);
    
    if !prompt_yes_no("\nDo you have your API credentials ready?")? {
        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = writeln!(&mut buffer, "\nSetup paused. Run 'kodegen-setup --interactive' when ready.");
        let _ = bufwtr.print(&buffer);
        return Ok(());
    }
    
    // Collect credentials
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(&mut buffer, "\n{}", "━".repeat(60));
    let _ = writeln!(&mut buffer, "Enter your API credentials:\n");
    let _ = bufwtr.print(&buffer);
    
    print!("Issuer ID: ");
    io::stdout().flush()?;
    let mut issuer_id = String::new();
    io::stdin().read_line(&mut issuer_id)?;
    let issuer_id = issuer_id.trim();
    
    print!("Key ID: ");
    io::stdout().flush()?;
    let mut key_id = String::new();
    io::stdin().read_line(&mut key_id)?;
    let key_id = key_id.trim();
    
    let key_path = if let Some(path) = prompt_for_p8_path()? { path } else {
        println!("\nSetup cancelled by user.");
        return Ok(());
    };
    
    // Create API client
    println!();
    success!("Validating credentials...");
    
    let client = apple_api::AppleAPIClient::new(
        key_id,
        issuer_id,
        std::path::Path::new(&key_path)
    )?;
    
    // ===== PRE-FLIGHT VALIDATION (prevents orphaned certificates) =====
    success!("Validating API credentials...");
    client.test_credentials()?;
    
    success!("Checking keychain accessibility...");
    ensure_keychain_accessible("login.keychain-db")?;
    
    success!("Verifying config directory permissions...");
    ensure_config_directory_writable()?;
    
    // All preconditions validated - safe to proceed
    // ===== END PRE-FLIGHT VALIDATION =====
    
    // Generate CSR
    success!("Generating certificate signing request...");
    
    let (csr_pem, private_key_pem) = apple_api::generate_csr(
        DEFAULT_COMMON_NAME
    )?;
    
    // Request certificate
    success!("Requesting certificate from Apple...");
    
    let cert_der = client.request_certificate(&csr_pem, CertificateType::DeveloperIdApplication)?;
    
    // Inform user about keychain customization
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(&mut buffer, "\nℹ️  Using default keychain: {DEFAULT_KEYCHAIN}");
    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
    let _ = writeln!(&mut buffer, "   To customize, edit ~/.config/kodegen/signing.toml after setup");
    let _ = buffer.reset();
    let _ = bufwtr.print(&buffer);
    
    // Import to keychain
    success!("Installing certificate to Keychain...");
    
    if let Err(e) = import_certificate_to_keychain(&cert_der, &private_key_pem, "login.keychain-db", DEFAULT_COMMON_NAME) {
        error_msg!("Certificate was created in Apple Developer account");
        eprintln!("   but could not be imported to keychain: {e}");
        eprintln!("\n⚠️  Manual cleanup required:");
        eprintln!("   1. Visit https://appstoreconnect.apple.com/access/api");
        eprintln!("   2. Go to Certificates section");
        eprintln!("   3. Delete the newly created certificate");
        eprintln!("   4. Fix the keychain issue and retry setup");
        return Err(e);
    }
    
    // Save config with secure permissions
    let config_dir = dirs::config_dir()
        .ok_or_else(|| SetupError::MissingConfig("Could not determine config directory".to_string()))?
        .join("kodegen");
    std::fs::create_dir_all(&config_dir)?;
    
    // Set secure directory permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir_perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&config_dir, dir_perms)?;
    }
    
    let config_path = config_dir.join("signing.toml");
    let config_content = format!(
        "# Kodegen Signing Configuration\n\
         # Generated by interactive setup\n\
         \n\
         [macos]\n\
         api_key_id = \"{key_id}\"\n\
         api_issuer_id = \"{issuer_id}\"\n\
         api_key_path = \"{key_path}\"\n\
         keychain = \"{DEFAULT_KEYCHAIN}\"\n\
         common_name = \"{DEFAULT_COMMON_NAME}\"\n\
         # certificate_type = \"developer_id\"  # or \"mac_app_distribution\"\n"
    );
    
    // Atomic write using temp file + rename
    let mut temp_file = NamedTempFile::new_in(&config_dir)?;
    temp_file.write_all(config_content.as_bytes())?;
    temp_file.flush()?;
    temp_file.persist(&config_path)
        .map_err(|e| SetupError::Io(e.error))?;
    
    // Set secure file permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file_perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&config_path, file_perms)?;
    }
    
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(&mut buffer, "\n✅ Certificate installed successfully!");
    let _ = buffer.reset();
    let _ = writeln!(&mut buffer, "Configuration saved to: {}", config_path.display());
    let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
    let _ = writeln!(&mut buffer, "\n✅ Setup complete! You can now run 'cargo build --package kodegen_daemon'");
    let _ = buffer.reset();
    let _ = bufwtr.print(&buffer);
    
    Ok(())
}

/// Setup from existing configuration file
pub fn setup_from_config(config: &MacOSSetupConfig, dry_run: bool, verbose: bool) -> Result<()> {
    // Check required dependencies before proceeding
    check_dependencies()?;
    
    // NEW: Check for environment variable override (CI/CD takes priority)
    if let Some(env_auth) = try_auth_from_env() {
        match env_auth {
            EnvAuth::Certificate { cert_base64, password } => {
                if verbose {
                    println!("Using certificate from APPLE_CERTIFICATE environment variable");
                }
                // Store the keychain - it will be cleaned up when function returns
                let _temp_keychain = setup_with_certificate(&cert_base64, &password, dry_run)?;
                
                println!("\n✅ Temporary keychain ready for signing operations");
                println!("   Keychain will be deleted when setup completes");
                
                // The keychain is now available for the rest of this function's lifetime
                // It will auto-cleanup when setup_from_config returns
                return Ok(());
            }
            EnvAuth::ApiKey { key_id, issuer_id, key_path } => {
                if verbose {
                    println!("Using API credentials from environment variables");
                }
                // Override config with env vars - continue with rest of function
                // using env-based config instead of file-based config
                let env_config = MacOSSetupConfig {
                    issuer_id,
                    key_id,
                    private_key_path: key_path,
                    certificate_type: config.certificate_type,
                    common_name: config.common_name.clone(),
                    keychain: config.keychain.clone(),
                };
                
                // Continue with the rest of this function using env_config
                return continue_setup_from_config(&env_config, dry_run, verbose);
            }
        }
    }
    
    // Continue with regular config-file based setup
    continue_setup_from_config(config, dry_run, verbose)
}

fn continue_setup_from_config(config: &MacOSSetupConfig, dry_run: bool, verbose: bool) -> Result<()> {
    // Create verbose logging helper
    let log = |msg: &str| {
        if verbose {
            let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            // Verbose/progress output - errors ignored (see module-level docs)
            let _ = writeln!(&mut buffer, "[VERBOSE] {msg}");
            let _ = bufwtr.print(&buffer);
        }
    };
    
    log(&format!("Starting setup with config: dry_run={dry_run}, verbose={verbose}"));
    log(&format!("Certificate type: {:?}", config.certificate_type));
    log(&format!("Keychain: {}", config.keychain));
    
    let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = bufwtr.buffer();
    let _ = writeln!(&mut buffer, "Provisioning certificate with provided configuration...\n");
    let _ = bufwtr.print(&buffer);
    
    // Expand tilde in config path with error checking
    log(&format!("Expanding path: {}", config.private_key_path.display()));
    let expanded_path = expand_tilde_path(&config.private_key_path.to_string_lossy())
        .map_err(|e| SetupError::InvalidConfig(format!(
            "Invalid private_key_path in configuration:\n{e}\n\n\
             Config file: ~/.config/kodegen/signing.toml\n\
             Please update private_key_path to use absolute path"
        )))?;
    let key_path = std::path::Path::new(&expanded_path);
    
    // Validate path before attempting API call
    log(&format!("Validating .p8 file: {expanded_path}"));
    validate_p8_file(key_path).map_err(|e| {
        SetupError::InvalidConfig(format!(
            "Invalid private_key_path in configuration:\n{e}\n\n\
             Config file: ~/.config/kodegen/signing.toml\n\
             Please update the private_key_path setting"
        ))
    })?;
    
    // Dry-run mode implementation
    if dry_run {
        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = buffer.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)));
        let _ = writeln!(&mut buffer, "\n🔍 DRY RUN MODE - No changes will be made");
        let _ = buffer.reset();
        let _ = bufwtr.print(&buffer);
        
        log("Validating API credentials");
        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = writeln!(&mut buffer, "\n✓ Validating Apple API credentials...");
        let _ = bufwtr.print(&buffer);
        
        // Create client and test credentials
        let client = apple_api::AppleAPIClient::new(
            &config.key_id,
            &config.issuer_id,
            key_path
        )?;
        
        client.test_credentials()?;
        
        success!("API credentials are valid");
        
        // Show what would happen
        let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = writeln!(&mut buffer, "\n📋 Would perform these actions:");
        let _ = writeln!(&mut buffer, "  1. Generate CSR with Common Name: {}", config.common_name);
        let _ = writeln!(&mut buffer, "  2. Request certificate type: {:?}", config.certificate_type);
        let _ = writeln!(&mut buffer, "  3. Import certificate to keychain: {}", config.keychain);
        let _ = writeln!(&mut buffer, "\n✅ Dry-run validation complete - configuration is valid");
        let _ = bufwtr.print(&buffer);
        
        return Ok(());
    }
    
    // Create API client with validated path
    log(&format!("Creating Apple API client (key_id: {})", config.key_id));
    let client = apple_api::AppleAPIClient::new(
        &config.key_id,
        &config.issuer_id,
        key_path
    )?;
    
    // ===== PRE-FLIGHT VALIDATION (prevents orphaned certificates) =====
    log("Validating API credentials");
    client.test_credentials()?;
    
    log(&format!("Checking keychain accessibility: {}", config.keychain));
    ensure_keychain_accessible(&config.keychain)?;
    
    // Note: setup_from_config doesn't save config file, so no directory check needed
    // ===== END PRE-FLIGHT VALIDATION =====
    
    log("Generating certificate signing request");
    let (csr_pem, private_key_pem) = apple_api::generate_csr(
        &config.common_name
    )?;
    
    if verbose {
        log(&format!("CSR length: {} bytes", csr_pem.len()));
        log(&format!("Private key length: {} bytes", private_key_pem.len()));
    }
    
    log("Requesting certificate from Apple API");
    success!("Requesting certificate from Apple...");
    
    let cert_der = client.request_certificate(&csr_pem, config.certificate_type)?;
    
    if verbose {
        log(&format!("Received certificate: {} bytes", cert_der.len()));
    }
    
    success!("Installing to keychain...");
    
    import_certificate_to_keychain(&cert_der, &private_key_pem, &config.keychain, &config.common_name)?;
    
    println!();
    success!("✅ Certificate provisioned successfully!");
    
    Ok(())
}
/// Write .p8 API key file from environment to standard location
///
/// Checks for `APPLE_API_KEY_CONTENT` environment variable and writes it to
/// `~/.private_keys/AuthKey_{KEY_ID}.p8`.
///
/// This enables CI/CD environments to provide the .p8 file contents as a
/// secret instead of requiring a file path.
///
/// # Returns
/// * `Ok(Some(PathBuf))` - File written successfully, returns path to the file
/// * `Ok(None)` - Env var not set, no action taken
/// * `Err(SetupError)` - Failed to write file
pub fn ensure_api_key_file() -> Result<Option<std::path::PathBuf>> {
    if let (Ok(key_id), Ok(key_content)) = (
        std::env::var("APPLE_API_KEY"),
        std::env::var("APPLE_API_KEY_CONTENT")
    ) {
        let key_dir = dirs::home_dir()
            .ok_or_else(|| SetupError::MissingConfig("HOME not set".to_string()))?
            .join(".private_keys");

        std::fs::create_dir_all(&key_dir)?;

        let key_path = key_dir.join(format!("AuthKey_{key_id}.p8"));
        std::fs::write(&key_path, &key_content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
        }

        println!("✓ Wrote API key to {}", key_path.display());
        return Ok(Some(key_path));
    }
    Ok(None)
}
