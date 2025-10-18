//! macOS certificate provisioning for kodegen

use crate::config::MacOSSetupConfig;
use crate::apple_api;
use std::process::Command;
use std::io::{self, Write};
use anyhow::{Result, bail};

pub fn show_config() -> Result<()> {
    // Check for Developer ID certificates
    let output = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map_err(|e| anyhow::anyhow!(format!("Failed to run security command: {}", e)))?;
    
    let identities = String::from_utf8_lossy(&output.stdout);
    
    if identities.contains("Developer ID Application") {
        println!("✅ Developer ID Certificate: Found");
        for line in identities.lines() {
            if line.contains("Developer ID Application") {
                println!("   {}", line.trim());
            }
        }
    } else {
        println!("❌ Developer ID Certificate: Not found");
    }
    
    // Check for stored credentials
    if let Some(home) = dirs::home_dir() {
        let cred_path = home.join(".config/kodegen/signing.toml");
        if cred_path.exists() {
            println!("\n✅ Signing Config: {}", cred_path.display());
        } else {
            println!("\n❌ Signing Config: Not found");
        }
    }
    
    Ok(())
}

pub fn interactive_setup() -> Result<()> {
    println!("\n{}", "━".repeat(60));
    println!("Step 1: Checking for existing Developer ID certificate...\n");
    
    // Check for existing certificate
    let output = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to check certificates: {}", e))?;
    
    let identities = String::from_utf8_lossy(&output.stdout);
    
    if identities.contains("Developer ID Application") {
        println!("✅ Found existing Developer ID certificate!\n");
        for line in identities.lines() {
            if line.contains("Developer ID Application") {
                println!("   {}", line.trim());
            }
        }
        
        println!("\n{}", "━".repeat(60));
        print!("\nDo you want to provision a new certificate? (y/n): ");
        io::stdout().flush()?;
        
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        
        if !response.trim().eq_ignore_ascii_case("y") {
            println!("\nSetup complete! Using existing certificate.");
            return Ok(());
        }
    } else {
        println!("❌ No Developer ID certificate found.\n");
    }
    
    println!("\n{}", "━".repeat(60));
    println!("Step 2: App Store Connect API Setup\n");
    
    println!("To provision a certificate, you need API credentials from:");
    println!("  https://appstoreconnect.apple.com/access/api\n");
    
    println!("Instructions:");
    println!("  1. Click the '+' button to create a new key");
    println!("  2. Name it 'Kodegen Signing' (or similar)");
    println!("  3. Select 'Developer' role");
    println!("  4. Click 'Generate'");
    println!("  5. Download the .p8 file (can only download once!)");
    println!("  6. Note the Key ID and Issuer ID from the page\n");
    
    println!("{}", "━".repeat(60));
    print!("\nDo you have your API credentials ready? (y/n): ");
    io::stdout().flush().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    
    let mut ready = String::new();
    io::stdin().read_line(&mut ready).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    
    if !ready.trim().eq_ignore_ascii_case("y") {
        println!("\nSetup paused. Run 'kodegen-setup --interactive' when ready.");
        return Ok(());
    }
    
    // Collect credentials
    println!("\n{}", "━".repeat(60));
    println!("Enter your API credentials:\n");
    
    print!("Issuer ID: ");
    io::stdout().flush().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let mut issuer_id = String::new();
    io::stdin().read_line(&mut issuer_id).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let issuer_id = issuer_id.trim();
    
    print!("Key ID: ");
    io::stdout().flush().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let mut key_id = String::new();
    io::stdin().read_line(&mut key_id).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let key_id = key_id.trim();
    
    print!("Path to .p8 file: ");
    io::stdout().flush().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let mut key_path = String::new();
    io::stdin().read_line(&mut key_path).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let key_path = shellexpand::tilde(key_path.trim()).to_string();
    
    print!("Email: ");
    io::stdout().flush().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let mut email = String::new();
    io::stdin().read_line(&mut email).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let email = email.trim();
    
    // Create API client
    println!("\n✓ Validating credentials...");
    let client = apple_api::AppleAPIClient::new(
        key_id,
        issuer_id,
        std::path::Path::new(&key_path)
    )?;
    
    // Generate CSR
    println!("✓ Generating certificate signing request...");
    let (csr_pem, private_key_pem) = apple_api::generate_csr(
        "Developer ID Application",
        email
    )?;
    
    // Request certificate
    println!("✓ Requesting certificate from Apple...");
    let cert_der = client.request_certificate(&csr_pem)?;
    
    // Import to keychain
    println!("✓ Installing certificate to Keychain...");
    
    // Save cert and key temporarily
    let temp_cert = std::env::temp_dir().join("kodegen_cert.der");
    let temp_key = std::env::temp_dir().join("kodegen_key.pem");
    std::fs::write(&temp_cert, &cert_der).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    std::fs::write(&temp_key, &private_key_pem).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    
    // Create p12 bundle using openssl
    let temp_p12 = std::env::temp_dir().join("kodegen_cert.p12");
    let output = Command::new("openssl")
        .args([
            "pkcs12", "-export",
            "-inkey", temp_key.to_str().ok_or_else(|| anyhow::anyhow!("Invalid temp key path".to_string()))?,
            "-in", temp_cert.to_str().ok_or_else(|| anyhow::anyhow!("Invalid temp cert path".to_string()))?,
            "-out", temp_p12.to_str().ok_or_else(|| anyhow::anyhow!("Invalid temp p12 path".to_string()))?,
            "-passout", "pass:",  // No password
        ])
        .output()
        .map_err(|e| anyhow::anyhow!(format!("Failed to create p12: {}", e)))?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(format!("Failed to create p12: {}", String::from_utf8_lossy(&output.stderr))));
    }
    
    // Import p12 to keychain
    let import_output = Command::new("security")
        .args(["import", temp_p12.to_str().ok_or_else(|| anyhow::anyhow!("Invalid p12 path".to_string()))?, "-k", "login.keychain-db", "-P", "", "-T", "/usr/bin/codesign"])
        .output()
        .map_err(|e| anyhow::anyhow!(format!("Failed to import to keychain: {}", e)))?;
    
    if !import_output.status.success() {
        let stderr = String::from_utf8_lossy(&import_output.stderr);
        return Err(anyhow::anyhow!(format!("Keychain import failed: {}", stderr)));
    }
    
    // Clean up temp files
    let _ = std::fs::remove_file(&temp_cert);
    let _ = std::fs::remove_file(&temp_key);
    let _ = std::fs::remove_file(&temp_p12);
    
    // Save config
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory".to_string()))?
        .join("kodegen");
    std::fs::create_dir_all(&config_dir).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    
    let config_path = config_dir.join("signing.toml");
    let config_content = format!(
        "[macos]\napi_key_id = \"{}\"\napi_issuer_id = \"{}\"\napi_key_path = \"{}\"\n",
        key_id, issuer_id, key_path
    );
    std::fs::write(&config_path, config_content).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    
    println!("\n✅ Certificate installed successfully!");
    println!("Configuration saved to: {}", config_path.display());
    println!("\n✅ Setup complete! You can now run 'cargo build --package kodegen_daemon'");
    
    Ok(())
}

pub fn setup_from_config(config: &MacOSSetupConfig, _dry_run: bool, _verbose: bool) -> Result<()> {
    println!("Provisioning certificate with provided configuration...\n");
    
    let client = apple_api::AppleAPIClient::new(
        &config.key_id,
        &config.issuer_id,
        &config.private_key_path
    )?;
    
    let (csr_pem, private_key_pem) = apple_api::generate_csr(
        &config.common_name,
        "developer@example.com"
    )?;
    
    println!("✓ Requesting certificate from Apple...");
    let cert_der = client.request_certificate(&csr_pem)?;
    
    println!("✓ Installing to keychain...");
    // Same installation logic as interactive setup
    let temp_cert = std::env::temp_dir().join("kodegen_cert.der");
    let temp_key = std::env::temp_dir().join("kodegen_key.pem");
    std::fs::write(&temp_cert, &cert_der).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    std::fs::write(&temp_key, &private_key_pem).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    
    let temp_p12 = std::env::temp_dir().join("kodegen_cert.p12");
    let output = Command::new("openssl")
        .args([
            "pkcs12", "-export",
            "-inkey", temp_key.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path".to_string()))?,
            "-in", temp_cert.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path".to_string()))?,
            "-out", temp_p12.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path".to_string()))?,
            "-passout", "pass:",
        ])
        .output()
        .map_err(|e| anyhow::anyhow!(format!("openssl failed: {}", e)))?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(format!("pkcs12 creation failed: {}", String::from_utf8_lossy(&output.stderr))));
    }
    
    let import_output = Command::new("security")
        .args(["import", temp_p12.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path".to_string()))?, "-k", &config.keychain, "-P", "", "-T", "/usr/bin/codesign"])
        .output()
        .map_err(|e| anyhow::anyhow!(format!("security import failed: {}", e)))?;
    
    if !import_output.status.success() {
        return Err(anyhow::anyhow!(format!("Import failed: {}", String::from_utf8_lossy(&import_output.stderr))));
    }
    
    let _ = std::fs::remove_file(&temp_cert);
    let _ = std::fs::remove_file(&temp_key);
    let _ = std::fs::remove_file(&temp_p12);
    
    println!("\n✅ Certificate provisioned successfully!");
    
    Ok(())
}
