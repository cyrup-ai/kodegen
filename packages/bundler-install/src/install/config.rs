//! Configuration and service setup for installer
//!
//! This module provides configuration generation, service setup, and platform-specific
//! installation logic with zero allocation fast paths and blazing-fast performance.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use log::{info, warn};
// Removed unused import: use time::OffsetDateTime;
use pem;
use rcgen::string::Ia5String;
use rcgen::{CertificateParams, DistinguishedName, DnType, SanType};
use x509_parser;

use super::core::{AsyncTask, CertificateConfig, InstallContext, InstallProgress, ServiceConfig};
use crate::install::fluent_voice;
use crate::install::{install_daemon_async, InstallerBuilder};
use crate::wizard::InstallationResult;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::timeout;

// Timeout constants
const RUSTUP_INSTALL_TIMEOUT: Duration = Duration::from_secs(1800); // 30 minutes
const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);         // 5 minutes default

/// Verify that rust-toolchain.toml exists and specifies nightly channel
///
/// This function checks that the project root contains a rust-toolchain.toml file
/// that specifies the nightly channel. The presence of this file ensures that cargo
/// will automatically use nightly when building this project, without requiring any
/// changes to the user's global default toolchain.
fn verify_rust_toolchain_file() -> Result<()> {
    // Get the project root (3 levels up from packages/bundler-install/src/install)
    let current_file = std::path::Path::new(file!());
    let project_root = current_file
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("Could not determine project root"))?;
    
    let toolchain_file = project_root.join("rust-toolchain.toml");
    
    if !toolchain_file.exists() {
        return Err(anyhow::anyhow!(
            "Missing rust-toolchain.toml in project root!\n\
             This file is required to specify the nightly toolchain for this project.\n\
             Expected location: {}",
            toolchain_file.display()
        ));
    }
    
    // Read and verify the file specifies nightly channel
    let content = fs::read_to_string(&toolchain_file)
        .with_context(|| format!("Failed to read {}", toolchain_file.display()))?;
    
    if !content.contains("channel") || !content.contains("nightly") {
        return Err(anyhow::anyhow!(
            "rust-toolchain.toml doesn't specify nightly channel!\n\
             The file must contain: channel = \"nightly\"\n\
             File location: {}",
            toolchain_file.display()
        ));
    }
    
    info!(
        "Verified rust-toolchain.toml specifies nightly at {}",
        toolchain_file.display()
    );
    Ok(())
}

/// Ensure Rust nightly toolchain is installed without changing global default
///
/// This function checks if Rust is installed and ensures the nightly toolchain
/// is available. It NEVER changes the user's global default toolchain, which would
/// be destructive to their existing Rust projects.
///
/// The function follows these principles:
/// 1. Only install toolchains, never change the default
/// 2. Preserve the user's existing default toolchain
/// 3. Rely on rust-toolchain.toml to activate nightly for this project
/// 4. Provide clear feedback about what was done
async fn ensure_rust_toolchain() -> Result<()> {
    // Check if rustc is installed
    let rustc_check = timeout(
        COMMAND_TIMEOUT,
        Command::new("rustc")
            .arg("--version")
            .output()
    ).await;
    
    match rustc_check {
        Ok(Ok(output)) if output.status.success() => {
            // Rust is installed, get current default
            let default_output = timeout(
                COMMAND_TIMEOUT,
                Command::new("rustup")
                    .args(["default"])
                    .output()
            ).await
                .context("Rustup default check timed out after 5 minutes")?
                .context("Failed to check rustup default toolchain")?;
            
            if default_output.status.success() {
                let default_toolchain = String::from_utf8_lossy(&default_output.stdout);
                let default_name = default_toolchain
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().next())
                    .unwrap_or("unknown");
                
                info!("Rust already installed: {default_name}");
                
                // Check if nightly is installed
                let list_output = timeout(
                    COMMAND_TIMEOUT,
                    Command::new("rustup")
                        .args(["toolchain", "list"])
                        .output()
                ).await
                    .context("Rustup toolchain list timed out after 5 minutes")?
                    .context("Failed to list rustup toolchains")?;
                
                if !list_output.status.success() {
                    return Err(anyhow::anyhow!("Failed to list rustup toolchains"));
                }
                
                let toolchains = String::from_utf8_lossy(&list_output.stdout);
                
                if toolchains.lines().any(|line| line.contains("nightly")) {
                    info!("Nightly toolchain already available");
                } else {
                    // Install nightly without changing default
                    info!("Installing nightly toolchain for kodegen (this may take up to 30 minutes)...");
                    
                    let install_output = timeout(
                        RUSTUP_INSTALL_TIMEOUT,
                        Command::new("rustup")
                            .args(["toolchain", "install", "nightly"])
                            .output()
                    ).await
                        .context("Rustup nightly install timed out after 30 minutes")?
                        .context("Failed to install nightly toolchain")?;
                    
                    if !install_output.status.success() {
                        let stderr = String::from_utf8_lossy(&install_output.stderr);
                        return Err(anyhow::anyhow!(
                            "Failed to install nightly toolchain: {stderr}"
                        ));
                    }
                    
                    info!("Nightly toolchain installed");
                }
                
                info!("Project will use nightly via rust-toolchain.toml (global default unchanged: {default_name})");
            } else {
                warn!("Could not determine current default toolchain, but Rust is installed");
            }
        }
        Ok(_) | Err(_) => {
            // Rust not installed, install with stable as default and nightly as additional
            info!("Installing Rust toolchain (this may take up to 30 minutes)...");
            
            // Download and run rustup installer
            let rustup_init = if cfg!(unix) {
                timeout(
                    RUSTUP_INSTALL_TIMEOUT,
                    Command::new("sh")
                        .args([
                            "-c",
                            "curl --proto '=https' --tlsv1.2 -sSf --max-time 300 --connect-timeout 30 https://sh.rustup.rs | sh -s -- -y --default-toolchain stable"
                        ])
                        .output()
                ).await
                    .context("Rustup installation timed out after 30 minutes")?
                    .context("Failed to download and run rustup installer")?
            } else {
                return Err(anyhow::anyhow!(
                    "Automatic Rust installation only supported on Unix systems"
                ));
            };
            
            if !rustup_init.status.success() {
                let stderr = String::from_utf8_lossy(&rustup_init.stderr);
                return Err(anyhow::anyhow!(
                    "Failed to install Rust: {stderr}"
                ));
            }
            
            // Get path to rustup binary (it's not in current process PATH yet)
            let home_dir = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
            
            let cargo_env = home_dir.join(".cargo").join("env");
            if cargo_env.exists() {
                info!("Rust stable installed: {}", cargo_env.display());
            }
            
            // Use full path to rustup since it's not in current process PATH yet
            let rustup_path = home_dir.join(".cargo").join("bin").join("rustup");
            
            if !rustup_path.exists() {
                return Err(anyhow::anyhow!(
                    "Rustup binary not found at expected location: {}",
                    rustup_path.display()
                ));
            }
            
            // Install nightly as additional toolchain using full path
            info!("Installing nightly toolchain for kodegen (this may take up to 30 minutes)...");
            
            let install_nightly = timeout(
                RUSTUP_INSTALL_TIMEOUT,
                Command::new(&rustup_path)
                    .args(["toolchain", "install", "nightly"])
                    .output()
            ).await
                .context("Nightly toolchain installation timed out after 30 minutes")?
                .context("Failed to install nightly toolchain")?;
            
            if !install_nightly.status.success() {
                let stderr = String::from_utf8_lossy(&install_nightly.stderr);
                return Err(anyhow::anyhow!(
                    "Failed to install nightly toolchain: {stderr}"
                ));
            }
            
            info!("Rust stable installed as default");
            info!("Nightly available for kodegen");
        }
    }
    
    Ok(())
}

/// Configure and install the Kodegen daemon with optimized installation flow
pub async fn install_kodegen_daemon(
    exe_path: PathBuf,
    config_path: PathBuf,
    auto_start: bool,
    progress_tx: Option<mpsc::UnboundedSender<InstallProgress>>,
) -> Result<InstallationResult> {
    let mut context = InstallContext::new(exe_path.clone());
    context.config_path = config_path.clone();
    
    // Clone progress channel BEFORE moving context into AsyncTask chain
    let progress_tx_for_error = if let Some(tx) = progress_tx {
        context.set_progress_channel(tx.clone());
        Some(tx)
    } else {
        None
    };

    // Build custom certificate configuration using builder pattern
    let cert_config = CertificateConfig::new("Kodegen Local CA".to_string())
        .organization("Kodegen".to_string())
        .country("US".to_string())
        .validity_days(365)
        .key_size(2048)
        .add_san("mcp.kodegen.ai".to_string())
        .add_san("localhost".to_string())
        .add_san("127.0.0.1".to_string())
        .add_san("::1".to_string());
    
    context.set_certificate_config(cert_config);

    // Chain installation steps with AsyncTask combinators
    let result_context = {
        let ctx = context;
        AsyncTask::from_future(async {
            verify_rust_toolchain_file()
        })
        .and_then(|()| async {
            ensure_rust_toolchain().await
        })
        .and_then(move |()| async move {
            ctx.validate_prerequisites()?;
            Ok(ctx)
        })
        .and_then(|ctx| async move {
            ctx.create_directories()?;
            Ok(ctx)
        })
        .and_then(|ctx| async move {
            ctx.generate_certificates()?;
            Ok(ctx)
        })
        .and_then(move |mut ctx| async move {
            configure_services(&mut ctx, auto_start)?;
            Ok(ctx)
        })
        .and_then(move |ctx| async move {
            let installer = build_installer_config(&ctx, auto_start)?;
            install_daemon_async(installer).await?;
            Ok(ctx)
        })
        .map(|ctx| {
            info!("Installation pipeline completed successfully");
            ctx
        })
        .map_err(move |e: anyhow::Error| {
            if let Some(ref tx) = progress_tx_for_error {
                let _ = tx.send(InstallProgress::error(
                    "installation".to_string(),
                    format!("Installation failed: {e}")
                ));
            }
            anyhow::anyhow!("Installation pipeline failed: {e}")
        })
        .await?
    };
    
    let context = result_context;

    // Track installation results for each component
    let mut certificates_installed = true;
    let mut host_entries_added = true;
    let mut fluent_voice_installed = true;
    let service_started = false; // Will be true if auto_start enabled (from INSTALL_4_FIX_2)

    info!("Daemon installed successfully");

    // Generate wildcard certificate and import to trust store
    if let Err(e) = generate_and_import_wildcard_certificate().await {
        warn!("Failed to generate wildcard certificate and import: {e}");
        certificates_installed = false;
    }

    // Add host entries for all Kodegen domains pointing to 127.0.0.1
    if let Err(e) = add_kodegen_host_entries() {
        warn!("Failed to add Kodegen host entries: {e}");
        host_entries_added = false;
    }

    // Install fluent-voice components
    let fluent_voice_path = std::path::Path::new("/opt/kodegen/fluent-voice");
    if let Err(e) = fluent_voice::install_fluent_voice(fluent_voice_path).await {
        warn!("Failed to install fluent-voice components: {e}");
        fluent_voice_installed = false;
    }

    // Determine actual service path
    let service_path = get_service_path(&context);

    context.send_progress(InstallProgress::complete(
        "installation".to_string(),
        "Kodegen daemon installed successfully".to_string(),
    ));

    Ok(InstallationResult {
        data_dir: context.data_dir.clone(),
        service_path,
        service_started,
        certificates_installed,
        host_entries_added,
        fluent_voice_installed,
    })
}

/// Determine the platform-specific service file path (always system-wide for system daemons)
fn get_service_path(_context: &InstallContext) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/LaunchDaemons/com.kodegen.daemon.plist")
    }

    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/etc/systemd/system/kodegend.service")
    }

    #[cfg(target_os = "windows")]
    {
        PathBuf::from("HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Services\\kodegend")
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PathBuf::from("unknown-platform")
    }
}

/// Configure services for the installer with optimized service configuration
fn configure_services(context: &mut InstallContext, _auto_start: bool) -> Result<()> {
    // Configure autoconfig service
    let autoconfig_service = ServiceConfig::new(
        "kodegen-autoconfig".to_string(),
        "internal:autoconfig".to_string(), // Special command handled internally
    )
    .description("Automatic MCP client configuration service".to_string())
    .env("RUST_LOG".to_string(), "info".to_string())
    .auto_restart(true)
    .depends_on("kodegen_daemon".to_string());

    context.add_service(autoconfig_service);

    context.send_progress(InstallProgress::new(
        "services".to_string(),
        0.6,
        "Configured system services".to_string(),
    ));

    Ok(())
}

/// Build installer configuration with platform-specific settings
fn build_installer_config(context: &InstallContext, auto_start: bool) -> Result<InstallerBuilder> {
    let mut installer = InstallerBuilder::new("kodegend", context.exe_path.clone())
        .description("kodegen Service Manager")
        .args([
            "run",
            "--foreground",
            "--config",
            &context.config_path.to_string_lossy(),
        ])
        .env("RUST_LOG", "info")
        .auto_restart(true)
        .network(true)
        .auto_start(auto_start);

    // Add configured services
    for service in &context.services {
        installer = installer.service(convert_to_service_definition(service)?);
    }

    // Platform-specific user/group settings
    #[cfg(target_os = "linux")]
    let installer = {
        if let Ok(group) = nix::unistd::Group::from_name("cyops")? {
            if group.is_some() {
                installer.group("cyops")
            } else {
                installer
            }
        } else {
            installer
        }
    };

    // On macOS, run as root with wheel group for system daemon privileges
    #[cfg(target_os = "macos")]
    let installer = installer.user("root").group("wheel");

    Ok(installer)
}

/// Convert `ServiceConfig` to service definition with optimized conversion
fn convert_to_service_definition(
    service: &ServiceConfig,
) -> Result<crate::config::ServiceDefinition> {
    let mut env_vars = std::collections::HashMap::new();
    for (key, value) in &service.env_vars {
        env_vars.insert(key.clone(), value.clone());
    }

    // Add default RUST_LOG if not present
    if !env_vars.contains_key("RUST_LOG") {
        env_vars.insert("RUST_LOG".to_string(), "info".to_string());
    }

    // Build command with args concatenated
    let full_command = if service.args.is_empty() {
        service.command.clone()
    } else {
        format!("{} {}", service.command, service.args.join(" "))
    };

    // Create health check configuration based on service type
    let health_check = match service.name.as_str() {
        "kodegen-autoconfig" => Some(crate::config::HealthCheckConfig {
            check_type: "tcp".to_string(),
            target: "127.0.0.1:8443".to_string(),
            interval_secs: 300, // Check every 5 minutes
            timeout_secs: 30,
            retries: 3,
            expected_response: None,
            on_failure: vec![],
        }),
        _ => None,
    };

    Ok(crate::config::ServiceDefinition {
        name: service.name.clone(),
        description: Some(service.description.clone()),
        command: full_command,
        working_dir: service
            .working_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        env_vars,
        auto_restart: service.auto_restart,
        user: service.user.clone(),
        group: service.group.clone(),
        restart_delay_s: Some(10),
        depends_on: service.dependencies.clone(),
        health_check,
        log_rotation: None,
        watch_dirs: Vec::new(),
        ephemeral_dir: None,
        service_type: Some(match service.name.as_str() {
            "kodegen-autoconfig" => "autoconfig".to_string(),
            _ => "service".to_string(),
        }),
        memfs: None,
    })
}

/// Generate and import wildcard certificate with optimized certificate generation
async fn generate_and_import_wildcard_certificate() -> Result<()> {
    let cert_dir = get_cert_dir();
    let wildcard_cert_path = cert_dir.join("wildcard.pem");

    // Check if wildcard certificate already exists and is valid
    if wildcard_cert_path.exists() {
        if let Ok(()) = validate_existing_wildcard_cert(&wildcard_cert_path) {
            info!("Valid wildcard certificate already exists, skipping generation");
            return Ok(());
        }
        info!("Existing wildcard certificate is invalid, regenerating");
    }

    // Ensure certificate directory exists
    tokio::fs::create_dir_all(&cert_dir)
        .await
        .context("Failed to create certificate directory")?;

    info!("Generating Kodegen certificate for mcp.kodegen.ai...");

    // Create certificate parameters for mcp.kodegen.ai
    let mut params = CertificateParams::new(vec!["mcp.kodegen.ai".to_string()])?;

    // Add subject alternative names for local MCP server
    params.subject_alt_names = vec![
        SanType::DnsName(Ia5String::try_from("mcp.kodegen.ai").context("Invalid DNS name")?),
        SanType::DnsName(Ia5String::try_from("localhost").context("Invalid DNS name")?),
        SanType::IpAddress("127.0.0.1".parse()?),
        SanType::IpAddress("::1".parse()?),
    ];

    // Set distinguished name
    let mut dn = DistinguishedName::new();
    dn.push(DnType::OrganizationName, "Kodegen");
    dn.push(DnType::CommonName, "mcp.kodegen.ai");
    params.distinguished_name = dn;

    // Set non-expiring validity period (100 years)
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + time::Duration::seconds(100 * 365 * 24 * 60 * 60);

    // Generate self-signed certificate with key pair
    let key_pair = rcgen::KeyPair::generate()?;
    let cert = params
        .self_signed(&key_pair)
        .context("Failed to generate certificate")?;

    // Create combined PEM file with certificate and private key
    let combined_pem = format!("{}\n{}", cert.pem(), key_pair.serialize_pem());

    // Write combined PEM file
    tokio::fs::write(&wildcard_cert_path, &combined_pem)
        .await
        .context("Failed to write wildcard certificate")?;

    // Set secure permissions on certificate file
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = tokio::fs::metadata(&wildcard_cert_path)
            .await
            .context("Failed to get file metadata")?
            .permissions();
        perms.set_mode(0o600); // Owner read/write only
        tokio::fs::set_permissions(&wildcard_cert_path, perms)
            .await
            .context("Failed to set file permissions")?;
    }

    info!(
        "Kodegen certificate generated successfully at {}",
        wildcard_cert_path.display()
    );

    // Import certificate to system trust store
    import_certificate_to_system(&wildcard_cert_path).await?;

    Ok(())
}

/// Import certificate to system trust store
async fn import_certificate_to_system(cert_path: &Path) -> Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            import_certificate_macos(cert_path).await
        } else if #[cfg(target_os = "linux")] {
            import_certificate_linux(cert_path).await
        } else if #[cfg(target_os = "windows")] {
            import_certificate_windows(cert_path).await
        } else {
            warn!("Certificate import not supported on this platform");
            Ok(())
        }
    }
}

/// Import certificate to macOS System keychain
#[cfg(target_os = "macos")]
async fn import_certificate_macos(cert_path: &Path) -> Result<()> {
    info!("Importing certificate to macOS System keychain...");

    // Extract just the certificate part (not private key) for system trust
    let combined_pem = tokio::fs::read_to_string(cert_path)
        .await
        .context("Failed to read certificate file")?;

    // Find the certificate part (everything before the private key)
    let cert_only = if let Some(key_start) = combined_pem.find("-----BEGIN PRIVATE KEY-----") {
        &combined_pem[..key_start]
    } else {
        &combined_pem
    };

    // Write certificate-only file to temp location
    let temp_cert = std::env::temp_dir().join("kodegen_mcp_cert.crt");
    tokio::fs::write(&temp_cert, cert_only)
        .await
        .context("Failed to write temp certificate")?;

    // Import to System keychain (requires elevated privileges)
    let output = tokio::process::Command::new("security")
        .args([
            "add-trusted-cert",
            "-d",  // Add to admin trust settings
            "-r", "trustRoot",  // Trust as root certificate
            "-k", "/Library/Keychains/System.keychain",
            temp_cert.to_str().ok_or_else(|| anyhow::anyhow!("Invalid temp cert path"))?,
        ])
        .output()
        .await
        .context("Failed to execute security command")?;

    // Clean up temp file
    let _ = tokio::fs::remove_file(&temp_cert).await;

    if output.status.success() {
        info!("Successfully imported certificate to macOS System keychain");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to import certificate to macOS keychain: {stderr}"))
    }
}

/// Import certificate to Linux system trust store
#[cfg(target_os = "linux")]
async fn import_certificate_linux(cert_path: &Path) -> Result<()> {
    info!("Importing certificate to Linux system trust store...");

    // Extract just the certificate part (not private key)
    let combined_pem = tokio::fs::read_to_string(cert_path)
        .await
        .context("Failed to read certificate file")?;

    let cert_only = if let Some(key_start) = combined_pem.find("-----BEGIN PRIVATE KEY-----") {
        &combined_pem[..key_start]
    } else {
        &combined_pem
    };

    // Copy to system CA certificates directory
    let system_cert_path = "/usr/local/share/ca-certificates/kodegen-mcp.crt";
    
    // Ensure directory exists
    tokio::fs::create_dir_all("/usr/local/share/ca-certificates")
        .await
        .context("Failed to create ca-certificates directory")?;

    tokio::fs::write(system_cert_path, cert_only)
        .await
        .context("Failed to write certificate to system trust store")?;

    // Update certificate trust store
    let output = tokio::process::Command::new("update-ca-certificates")
        .output()
        .await
        .context("Failed to execute update-ca-certificates")?;

    if output.status.success() {
        info!("Successfully imported certificate to Linux system trust store");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to update certificate trust store: {stderr}"))
    }
}

/// Import certificate to Windows certificate store
#[cfg(target_os = "windows")]
async fn import_certificate_windows(cert_path: &Path) -> Result<()> {
    info!("Importing certificate to Windows certificate store...");

    // Extract just the certificate part (not private key)
    let combined_pem = tokio::fs::read_to_string(cert_path)
        .await
        .context("Failed to read certificate file")?;

    let cert_only = if let Some(key_start) = combined_pem.find("-----BEGIN PRIVATE KEY-----") {
        &combined_pem[..key_start]
    } else {
        &combined_pem
    };

    // Write certificate-only file to temp location
    let temp_cert = std::env::temp_dir().join("kodegen_mcp_cert.crt");
    tokio::fs::write(&temp_cert, cert_only)
        .await
        .context("Failed to write temp certificate")?;

    // Import to Trusted Root Certification Authorities store
    let output = tokio::process::Command::new("certutil")
        .args([
            "-addstore",
            "-f",
            "Root",
            temp_cert.to_str().ok_or_else(|| anyhow::anyhow!("Invalid temp cert path"))?,
        ])
        .output()
        .await
        .context("Failed to execute certutil command")?;

    // Clean up temp file
    let _ = tokio::fs::remove_file(&temp_cert).await;

    if output.status.success() {
        info!("Successfully imported certificate to Windows certificate store");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to import certificate to Windows store: {stderr}"))
    }
}

/// Get certificate directory path with platform-specific logic
fn get_cert_dir() -> PathBuf {
    InstallContext::get_data_dir().join("certs")
}

/// Validate existing wildcard certificate with fast validation
fn validate_existing_wildcard_cert(cert_path: &Path) -> Result<()> {
    // Read certificate file
    let cert_pem = fs::read_to_string(cert_path).context("Failed to read certificate file")?;

    // Parse certificate to validate it's well-formed
    let cert_der = pem::parse(&cert_pem).context("Failed to parse certificate PEM")?;

    if cert_der.tag() != "CERTIFICATE" {
        return Err(anyhow::anyhow!("Invalid certificate format"));
    }

    // Parse X.509 certificate
    let cert = x509_parser::parse_x509_certificate(cert_der.contents())
        .context("Failed to parse X.509 certificate")?
        .1;

    // Check if certificate is still valid (not expired)
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .context("Failed to get current time")?
        .as_secs();

    let not_after = cert.validity().not_after.timestamp() as u64;

    if now > not_after {
        return Err(anyhow::anyhow!("Certificate has expired"));
    }

    // Check if certificate expires within 30 days
    if now + (30 * 24 * 60 * 60) > not_after {
        warn!("Certificate expires within 30 days, consider regenerating");
    }

    Ok(())
}

/// Check if a hosts file line contains the specified IP and hostname entry
fn check_hosts_entry(line: &str, ip: &str, hostname: &str) -> bool {
    let trimmed = line.trim();
    
    // Skip comments and empty lines
    if trimmed.starts_with('#') || trimmed.is_empty() {
        return false;
    }
    
    // Split by whitespace (handles both spaces and tabs)
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() < 2 {
        return false;
    }
    
    // Check if IP matches and hostname matches (case insensitive for DNS)
    parts[0] == ip && parts[1..].iter().any(|h| h.eq_ignore_ascii_case(hostname))
}

/// Remove Kodegen block from hosts file content
fn remove_kodegen_block(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = Vec::new();
    let mut in_kodegen_section = false;

    for line in lines {
        if line.trim() == "# Kodegen entries" {
            in_kodegen_section = true;
            continue;
        }
        if line.trim() == "# End Kodegen entries" {
            in_kodegen_section = false;
            continue;
        }
        if !in_kodegen_section {
            new_lines.push(line);
        }
    }

    new_lines.join("\n")
}

/// Write file atomically using temp file + rename pattern
fn write_hosts_file_atomic(path: &Path, content: &str) -> Result<()> {
    use std::io::Write;
    
    // Create temp file in same directory as target (ensures same filesystem for atomic rename)
    let temp_path = path.with_extension("tmp");
    
    // Write to temp file with explicit sync
    {
        let mut file = fs::File::create(&temp_path)
            .with_context(|| format!("Failed to create temp file: {}", temp_path.display()))?;
        
        file.write_all(content.as_bytes())
            .context("Failed to write to temp file")?;
        
        file.sync_all()
            .context("Failed to sync temp file to disk")?;
    }
    
    // Atomically rename temp to target
    fs::rename(&temp_path, path)
        .with_context(|| format!("Failed to rename temp file to {}", path.display()))?;
    
    Ok(())
}

/// Add Kodegen host entries with lock-tight atomic modification
fn add_kodegen_host_entries() -> Result<()> {
    let hosts_file_path = get_hosts_file_path();

    // Read existing hosts file
    let existing_content =
        fs::read_to_string(&hosts_file_path).context("Failed to read hosts file")?;

    // Check if the actual entry already exists (not just the marker)
    let has_entry = existing_content.lines().any(|line| {
        check_hosts_entry(line, "127.0.0.1", "mcp.kodegen.ai")
    });

    if has_entry {
        info!("Entry 127.0.0.1 mcp.kodegen.ai already exists in hosts file, skipping");
        return Ok(());
    }

    // Remove any existing Kodegen block (handles broken/partial entries)
    let cleaned_content = remove_kodegen_block(&existing_content);

    // Build new content with Kodegen block
    let mut new_content = cleaned_content;
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push('\n');
    new_content.push_str("# Kodegen entries\n");
    new_content.push_str("127.0.0.1 mcp.kodegen.ai\n");
    new_content.push_str("# End Kodegen entries\n");

    // Write atomically (temp + rename)
    write_hosts_file_atomic(&hosts_file_path, &new_content)
        .context("Failed to write hosts file atomically")?;

    info!(
        "Added Kodegen host entry to {}",
        hosts_file_path.display()
    );
    Ok(())
}

/// Get hosts file path with platform-specific logic
fn get_hosts_file_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/etc/hosts")
    }
    #[cfg(windows)]
    {
        PathBuf::from("C:\\Windows\\System32\\drivers\\etc\\hosts")
    }
    #[cfg(not(any(unix, windows)))]
    {
        PathBuf::from("/etc/hosts")
    }
}

/// Remove Kodegen host entries with atomic file modification
pub fn remove_kodegen_host_entries() -> Result<()> {
    let hosts_file_path = get_hosts_file_path();

    // Read existing hosts file
    let existing_content =
        fs::read_to_string(&hosts_file_path).context("Failed to read hosts file")?;

    // Check if Kodegen block exists
    if !existing_content.contains("# Kodegen entries") {
        info!("No Kodegen host entries found, skipping removal");
        return Ok(());
    }

    // Remove Kodegen block
    let new_content = remove_kodegen_block(&existing_content);

    // Write atomically (temp + rename)
    write_hosts_file_atomic(&hosts_file_path, &new_content)
        .context("Failed to write hosts file atomically")?;

    info!(
        "Removed Kodegen host entries from {}",
        hosts_file_path.display()
    );
    Ok(())
}

/// Create default configuration file with optimized config generation
#[allow(dead_code)] // Library function for installer/setup operations
pub fn create_default_configuration(config_path: &Path) -> Result<()> {
    let config_dir = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid configuration path"))?;

    // Create configuration directory if it doesn't exist
    fs::create_dir_all(config_dir).context("Failed to create configuration directory")?;

    // Default configuration content
    let default_config = r#"
# Kodegen Daemon Configuration

[daemon]
# Daemon process settings
pid_file = "/var/run/kodegen/daemon.pid"
log_level = "info"
log_file = "/var/log/kodegen/daemon.log"

[network]
# Network configuration
bind_address = "127.0.0.1"
port = 33399
max_connections = 1000

[security]
# Security settings
enable_tls = true
cert_file = "/usr/local/var/kodegen/certs/server.crt"
key_file = "/usr/local/var/kodegen/certs/server.key"
ca_file = "/usr/local/var/kodegen/certs/ca.crt"

[services]
# Service configuration
enable_autoconfig = true
enable_voice = false

[database]
# Database configuration
url = "surrealkv:///usr/local/var/kodegen/data/kodegen.db"
namespace = "kodegen"
database = "main"

[plugins]
# Plugin configuration
plugin_dir = "/usr/local/var/kodegen/plugins"
enable_sandboxing = true
max_memory_mb = 256
timeout_seconds = 30
"#;

    // Write default configuration
    fs::write(config_path, default_config).context("Failed to write default configuration")?;

    info!("Created default configuration at {config_path:?}");
    Ok(())
}
